#!/usr/bin/env python3
"""
Run the Vera task suite against an arbitrary local binary.

This is meant for evidence-driven retrieval tuning loops where we want to
compare old releases, regression points, and candidate builds under the same
corpus and the same CLI flags.
"""

from __future__ import annotations

import argparse
import json
import math
import shutil
import subprocess
import time
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path
from statistics import median
from typing import Any


REPO_ROOT = Path(__file__).resolve().parent.parent.parent
CORPUS_DIR = REPO_ROOT / ".bench" / "repos"
TASKS_DIR = REPO_ROOT / "eval" / "tasks"
RESULTS_DIR = REPO_ROOT / "benchmarks" / "results" / "local-binaries"
METRIC_KEYS = ["recall_at_1", "recall_at_5", "recall_at_10", "mrr", "ndcg"]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path, help="Path to Vera binary")
    parser.add_argument("--label", required=True, help="Short run label")
    parser.add_argument(
        "--extra-arg",
        action="append",
        default=[],
        help="Extra CLI arg passed to both index and search, repeatable",
    )
    parser.add_argument("--limit", type=int, default=20, help="Search result limit")
    parser.add_argument(
        "--output",
        type=Path,
        help="Output JSON path (defaults under benchmarks/results/local-binaries)",
    )
    parser.add_argument(
        "--compare",
        action="append",
        default=[],
        type=Path,
        help="Optional prior result JSON to diff against, repeatable",
    )
    return parser.parse_args()


def load_tasks() -> list[dict[str, Any]]:
    tasks: list[dict[str, Any]] = []
    for task_file in sorted(TASKS_DIR.glob("*.json")):
        tasks.extend(json.loads(task_file.read_text()))
    return tasks


def repo_order(tasks: list[dict[str, Any]]) -> list[str]:
    seen = set()
    ordered = []
    for task in tasks:
        repo = task["repo"]
        if repo not in seen:
            seen.add(repo)
            ordered.append(repo)
    return ordered


def run_json(cmd: list[str], cwd: Path) -> tuple[Any, float]:
    start = time.monotonic()
    proc = subprocess.run(cmd, cwd=cwd, text=True, capture_output=True, timeout=600)
    elapsed = time.monotonic() - start
    if proc.returncode != 0:
        stderr = proc.stderr.strip()[:500]
        raise RuntimeError(f"command failed: {' '.join(cmd)}\n{stderr}")
    stdout = proc.stdout.strip()
    return (json.loads(stdout) if stdout else [], elapsed)


def binary_version(binary: Path) -> str:
    proc = subprocess.run(
        [str(binary), "--version"],
        text=True,
        capture_output=True,
        timeout=10,
    )
    return proc.stdout.strip() or proc.stderr.strip() or "unknown"


def git_sha() -> str:
    proc = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        timeout=10,
    )
    return proc.stdout.strip()


def effective_config(binary: Path, cwd: Path) -> dict[str, Any]:
    config, _ = run_json([str(binary), "--json", "config"], cwd)
    return config if isinstance(config, dict) else {}


def is_match(result: dict[str, Any], gt: dict[str, Any]) -> bool:
    return (
        result.get("file_path") == gt["file_path"]
        and result.get("line_start", 0) <= gt["line_end"]
        and result.get("line_end", 0) >= gt["line_start"]
    )


def recall_at_k(results: list[dict[str, Any]], ground_truth: list[dict[str, Any]], k: int) -> float:
    if not ground_truth:
        return 0.0
    top_k = results[:k]
    found = sum(1 for gt in ground_truth if any(is_match(result, gt) for result in top_k))
    return found / len(ground_truth)


def mrr(results: list[dict[str, Any]], ground_truth: list[dict[str, Any]]) -> float:
    for idx, result in enumerate(results):
        if any(is_match(result, gt) for gt in ground_truth):
            return 1.0 / (idx + 1)
    return 0.0


def matched_relevances(
    results: list[dict[str, Any]],
    ground_truth: list[dict[str, Any]],
    k: int,
) -> list[int]:
    used = [False] * len(ground_truth)
    relevances = []
    for result in results[:k]:
        best_idx = None
        best_rel = 0
        for idx, gt in enumerate(ground_truth):
            if used[idx] or not is_match(result, gt):
                continue
            rel = gt.get("relevance", 1)
            if rel > best_rel:
                best_idx = idx
                best_rel = rel
        if best_idx is None:
            relevances.append(0)
        else:
            used[best_idx] = True
            relevances.append(best_rel)
    return relevances


def ndcg(results: list[dict[str, Any]], ground_truth: list[dict[str, Any]], k: int = 10) -> float:
    dcg = sum(
        rel / math.log2(rank + 2.0)
        for rank, rel in enumerate(matched_relevances(results, ground_truth, k))
    )
    ideal = sorted((gt.get("relevance", 1) for gt in ground_truth), reverse=True)[:k]
    idcg = sum(rel / math.log2(rank + 2.0) for rank, rel in enumerate(ideal))
    return dcg / idcg if idcg > 0 else 0.0


def aggregate_metrics(rows: list[dict[str, Any]]) -> dict[str, float]:
    return {
        key: sum(row["metrics"][key] for row in rows) / len(rows) if rows else 0.0
        for key in METRIC_KEYS
    }


def aggregate_latency(rows: list[dict[str, Any]]) -> dict[str, float]:
    latencies = [row["latency_ms"] for row in rows]
    if not latencies:
        return {"p50_ms": 0.0, "p95_ms": 0.0}
    ordered = sorted(latencies)
    p95_index = max(0, math.ceil(len(ordered) * 0.95) - 1)
    return {
        "p50_ms": median(ordered),
        "p95_ms": ordered[p95_index],
    }


def summarize_per_category(rows: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        grouped[row["category"]].append(row)

    summary = {}
    for category, items in sorted(grouped.items()):
        summary[category] = {
            "count": len(items),
            "metrics": aggregate_metrics(items),
            "latency": aggregate_latency(items),
        }
    return summary


def index_repo(binary: Path, repo_name: str, extra_args: list[str]) -> dict[str, Any]:
    repo_path = CORPUS_DIR / repo_name
    shutil.rmtree(repo_path / ".vera", ignore_errors=True)
    cmd = [str(binary), "--json", "index", str(repo_path), *extra_args]
    summary, elapsed = run_json(cmd, repo_path)
    storage_bytes = 0
    for file_path in (repo_path / ".vera").rglob("*"):
        if file_path.is_file():
            storage_bytes += file_path.stat().st_size
    return {
        "repo": repo_name,
        "elapsed_secs": elapsed,
        "files_parsed": summary.get("files_parsed"),
        "chunks_created": summary.get("chunks_created"),
        "storage_bytes": storage_bytes,
    }


def run_task(
    binary: Path,
    task: dict[str, Any],
    extra_args: list[str],
    limit: int,
) -> dict[str, Any]:
    repo_path = CORPUS_DIR / task["repo"]
    cmd = [str(binary), "--json", "search", task["query"], "--limit", str(limit), *extra_args]
    results, elapsed = run_json(cmd, repo_path)
    metrics = {
        "recall_at_1": recall_at_k(results, task["ground_truth"], 1),
        "recall_at_5": recall_at_k(results, task["ground_truth"], 5),
        "recall_at_10": recall_at_k(results, task["ground_truth"], 10),
        "mrr": mrr(results, task["ground_truth"]),
        "ndcg": ndcg(results, task["ground_truth"], 10),
    }
    return {
        "task_id": task["id"],
        "category": task["category"],
        "repo": task["repo"],
        "query": task["query"],
        "latency_ms": elapsed * 1000.0,
        "metrics": metrics,
        "results": results,
    }


def print_summary(result: dict[str, Any]) -> None:
    agg = result["aggregate"]["metrics"]
    latency = result["aggregate"]["latency"]
    print(
        f"{result['label']}: "
        f"R@1 {agg['recall_at_1']:.4f} | "
        f"R@5 {agg['recall_at_5']:.4f} | "
        f"R@10 {agg['recall_at_10']:.4f} | "
        f"MRR {agg['mrr']:.4f} | "
        f"nDCG {agg['ndcg']:.4f} | "
        f"p50 {latency['p50_ms']:.0f}ms | "
        f"p95 {latency['p95_ms']:.0f}ms"
    )


def print_comparison(result: dict[str, Any], compare_path: Path) -> None:
    baseline = json.loads(compare_path.read_text())
    current = result["aggregate"]["metrics"]
    previous = baseline["aggregate"]["metrics"]
    deltas = []
    for key, label in [
        ("recall_at_1", "R@1"),
        ("recall_at_5", "R@5"),
        ("recall_at_10", "R@10"),
        ("mrr", "MRR"),
        ("ndcg", "nDCG"),
    ]:
        delta = current[key] - previous[key]
        deltas.append(f"{label} {delta:+.4f}")
    print(f"vs {baseline['label']}: {' | '.join(deltas)}")


def main() -> None:
    args = parse_args()
    binary = args.binary.resolve()
    if not binary.exists():
        raise SystemExit(f"binary not found: {binary}")

    tasks = load_tasks()
    repos = repo_order(tasks)
    output_path = args.output or RESULTS_DIR / f"{args.label}.json"
    output_path.parent.mkdir(parents=True, exist_ok=True)

    print(f"Benchmarking {binary}")
    print(f"Label: {args.label}")
    print(f"Extra args: {args.extra_arg or 'none'}")

    effective_config_by_repo: dict[str, dict[str, Any]] = {}
    for repo_name in repos:
        repo_path = CORPUS_DIR / repo_name
        effective_config_by_repo[repo_name] = effective_config(binary, repo_path)

    reference_config_repo = repos[0] if repos else "<none>"
    reference_config = (
        effective_config_by_repo.get(reference_config_repo, {}) if repos else effective_config(binary, REPO_ROOT)
    )
    all_repo_configs_match = (
        len(
            {
                json.dumps(cfg, sort_keys=True)
                for cfg in effective_config_by_repo.values()
            }
        )
        <= 1
    )

    indexing = []
    for repo_name in repos:
        stat = index_repo(binary, repo_name, args.extra_arg)
        indexing.append(stat)
        print(
            f"INDEX {repo_name}: {stat['files_parsed']} files, "
            f"{stat['chunks_created']} chunks, {stat['elapsed_secs']:.1f}s"
        )

    per_task = []
    for task in tasks:
        row = run_task(binary, task, args.extra_arg, args.limit)
        per_task.append(row)
        print(
            f"TASK {row['task_id']}: "
            f"R1={row['metrics']['recall_at_1']:.2f} "
            f"R5={row['metrics']['recall_at_5']:.2f} "
            f"MRR={row['metrics']['mrr']:.3f}"
        )

    result = {
        "label": args.label,
        "binary": str(binary),
        "binary_version": binary_version(binary),
        "git_sha": git_sha(),
        "timestamp_utc": datetime.now(timezone.utc).isoformat(),
        "config_reference_repo": reference_config_repo,
        "effective_config": reference_config,
        "all_repo_configs_match": all_repo_configs_match,
        "effective_config_by_repo": effective_config_by_repo,
        "extra_args": args.extra_arg,
        "limit": args.limit,
        "indexing": indexing,
        "per_task": per_task,
        "per_category": summarize_per_category(per_task),
        "aggregate": {
            "metrics": aggregate_metrics(per_task),
            "latency": aggregate_latency(per_task),
        },
    }

    output_path.write_text(json.dumps(result, indent=2))
    print_summary(result)
    for compare_path in args.compare:
        print_comparison(result, compare_path)
    print(f"Wrote {output_path}")


if __name__ == "__main__":
    main()
