#!/usr/bin/env python3
"""
Embedding Model Comparison Spike for Vera.

Compares embedding models on Vera's benchmark task suite using the existing
eval harness metrics (Recall@k, MRR, nDCG). Tests models across all 5 workload
categories to determine which embedding model best serves Vera's hybrid pipeline.

Models compared:
  1. Qwen3-Embedding-8B  (code-optimized, 4096-dim, via Nebius)
  2. BAAI/bge-en-icl      (general-purpose, strong, via Nebius)
  3. Qwen3-Embedding-0.6B (lightweight, 1024-dim, via SiliconFlow)

Usage:
    set -a; source secrets.env; set +a
    python3 spikes/embedding-chunking/run_embedding_comparison.py
"""

import json
import math
import os
import subprocess
import sys
import time
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
TASKS_DIR = REPO_ROOT / "eval" / "tasks"
BENCH_REPOS = REPO_ROOT / ".bench" / "repos"
RESULTS_DIR = Path(__file__).resolve().parent / "results"

# Embedding model configurations
MODELS = [
    {
        "name": "Qwen3-Embedding-8B",
        "model_id": "Qwen/Qwen3-Embedding-8B",
        "api_base": os.environ.get("EMBEDDING_MODEL_BASE_URL", ""),
        "api_key": os.environ.get("EMBEDDING_MODEL_API_KEY", ""),
        "description": "Code-optimized 8B parameter model, 4096-dim embeddings",
    },
    {
        "name": "bge-en-icl",
        "model_id": "BAAI/bge-en-icl",
        "api_base": os.environ.get("EMBEDDING_MODEL_BASE_URL", ""),
        "api_key": os.environ.get("EMBEDDING_MODEL_API_KEY", ""),
        "description": "General-purpose ICL embedding model by BAAI",
    },
    {
        "name": "Qwen3-Embedding-0.6B",
        "model_id": "Qwen/Qwen3-Embedding-0.6B",
        "api_base": "https://api.siliconflow.com/v1",
        "api_key": os.environ.get("RERANKER_MODEL_API_KEY", ""),
        "description": "Lightweight 0.6B model, 1024-dim, fast inference",
    },
]


def load_tasks() -> list[dict]:
    """Load all benchmark tasks from the tasks directory."""
    tasks = []
    for path in sorted(TASKS_DIR.glob("*.json")):
        with open(path) as f:
            data = json.load(f)
        if isinstance(data, list):
            tasks.extend(data)
        else:
            tasks.append(data)
    tasks.sort(key=lambda t: t["id"])
    return tasks


def embed_batch(texts: list[str], model_cfg: dict) -> list[list[float]]:
    """Get embeddings from an OpenAI-compatible API."""
    import urllib.request
    import urllib.error

    url = f"{model_cfg['api_base']}/embeddings"
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {model_cfg['api_key']}",
    }
    payload = {
        "input": texts,
        "model": model_cfg["model_id"],
    }

    req = urllib.request.Request(
        url,
        data=json.dumps(payload).encode(),
        headers=headers,
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            data = json.loads(resp.read())
        return [item["embedding"] for item in data["data"]]
    except urllib.error.HTTPError as e:
        error_body = e.read().decode() if e.fp else ""
        print(f"  Embedding API error: {e.code} {error_body[:200]}")
        raise


def cosine_similarity(a: list[float], b: list[float]) -> float:
    """Compute cosine similarity between two vectors."""
    dot = sum(x * y for x, y in zip(a, b))
    norm_a = math.sqrt(sum(x * x for x in a))
    norm_b = math.sqrt(sum(x * x for x in b))
    if norm_a == 0 or norm_b == 0:
        return 0.0
    return dot / (norm_a * norm_b)


def get_source_files(repo_path: str, max_files: int = 300) -> list[str]:
    """Get source files using ripgrep's file listing."""
    extensions = {
        ".rs", ".py", ".js", ".ts", ".tsx", ".go", ".java", ".c", ".cpp",
        ".h", ".hpp", ".rb", ".toml", ".json", ".yaml", ".yml",
        ".css", ".html", ".sh",
    }
    source_files = []
    other_files = []
    try:
        result = subprocess.run(
            ["rg", "--files", repo_path],
            capture_output=True, text=True, timeout=30,
        )
        for line in result.stdout.strip().split("\n"):
            if not line:
                continue
            rel = line
            if line.startswith(repo_path):
                rel = line[len(repo_path):].lstrip("/")
            ext = os.path.splitext(rel)[1].lower()
            if ext not in extensions:
                continue
            if any(part in rel for part in [
                "node_modules/", ".git/", "vendor/", "target/",
                "__pycache__/", ".venv/", "dist/", "build/",
                "test_fixtures/", "testdata/",
            ]):
                continue
            if any(p in rel for p in ["src/", "lib/", "crates/", "packages/"]):
                source_files.append(rel)
            else:
                other_files.append(rel)
    except Exception:
        pass
    all_files = source_files + other_files
    return all_files[:max_files]


def chunk_file_sliding_window(file_path: str, repo_path: str,
                               chunk_size: int = 50, overlap: int = 10) -> list[dict]:
    """Sliding-window line-based chunking (same as vector-only baseline)."""
    abs_path = os.path.join(repo_path, file_path)
    try:
        with open(abs_path, "r", errors="replace") as f:
            lines = f.readlines()
    except (OSError, UnicodeDecodeError):
        return []

    if not lines:
        return []

    chunks = []
    i = 0
    while i < len(lines):
        end = min(i + chunk_size, len(lines))
        content = "".join(lines[i:end])
        chunks.append({
            "file_path": file_path,
            "line_start": i + 1,
            "line_end": end,
            "content": content,
        })
        i += chunk_size - overlap

    return chunks


def index_repo(repo_name: str, model_cfg: dict) -> dict:
    """Index a repository with sliding-window chunking and the given embedding model."""
    repo_path = str(BENCH_REPOS / repo_name)
    if not os.path.isdir(repo_path):
        print(f"  WARNING: Repo {repo_name} not found at {repo_path}")
        return {"chunks": [], "embeddings": []}

    files = get_source_files(repo_path)
    print(f"  [{model_cfg['name']}] Found {len(files)} source files in {repo_name}")

    all_chunks = []
    for f in files:
        all_chunks.extend(chunk_file_sliding_window(f, repo_path))

    print(f"  [{model_cfg['name']}] Created {len(all_chunks)} chunks")

    if not all_chunks:
        return {"chunks": [], "embeddings": []}

    # Embed in batches
    batch_size = 32
    all_embeddings = []
    start_time = time.time()

    for i in range(0, len(all_chunks), batch_size):
        batch = all_chunks[i:i + batch_size]
        texts = [c["content"][:2000] for c in batch]
        try:
            embeddings = embed_batch(texts, model_cfg)
            all_embeddings.extend(embeddings)
        except Exception as e:
            print(f"  [{model_cfg['name']}] Embedding batch {i // batch_size} failed: {e}")
            # Use zero vectors as fallback
            dim = all_embeddings[0] if all_embeddings else [0.0] * 768
            all_embeddings.extend([[0.0] * len(dim)] * len(batch))

        if (i // batch_size) % 20 == 0 and i > 0:
            elapsed = time.time() - start_time
            print(f"  [{model_cfg['name']}] Embedded {min(i + batch_size, len(all_chunks))}/{len(all_chunks)} "
                  f"chunks ({elapsed:.1f}s)")

    total_time = time.time() - start_time
    print(f"  [{model_cfg['name']}] Indexed {repo_name}: {len(all_chunks)} chunks in {total_time:.1f}s")

    return {
        "chunks": all_chunks,
        "embeddings": all_embeddings,
        "index_time": total_time,
        "dim": len(all_embeddings[0]) if all_embeddings else 0,
    }


def search(query: str, index: dict, model_cfg: dict, max_results: int = 20) -> tuple[list[dict], float]:
    """Search by embedding the query and finding nearest chunks."""
    if not index["chunks"]:
        return [], 0.0

    start = time.time()
    try:
        query_emb = embed_batch([query], model_cfg)[0]
    except Exception:
        return [], 0.0

    scores = []
    for i, emb in enumerate(index["embeddings"]):
        sim = cosine_similarity(query_emb, emb)
        scores.append((i, sim))

    scores.sort(key=lambda x: -x[1])

    results = []
    for idx, score in scores[:max_results]:
        chunk = index["chunks"][idx]
        results.append({
            "file_path": chunk["file_path"],
            "line_start": chunk["line_start"],
            "line_end": chunk["line_end"],
            "score": score,
        })

    latency = (time.time() - start) * 1000
    return results, latency


# Metrics (mirrors eval harness)
def is_match(result: dict, gt: dict) -> bool:
    return (result["file_path"] == gt["file_path"]
            and result["line_start"] <= gt["line_end"]
            and result["line_end"] >= gt["line_start"])


def recall_at_k(results: list[dict], ground_truth: list[dict], k: int) -> float:
    if not ground_truth:
        return 0.0
    top_k = results[:k]
    found = sum(1 for gt in ground_truth if any(is_match(r, gt) for r in top_k))
    return found / len(ground_truth)


def mrr_score(results: list[dict], ground_truth: list[dict]) -> float:
    for i, r in enumerate(results):
        if any(is_match(r, gt) for gt in ground_truth):
            return 1.0 / (i + 1)
    return 0.0


def ndcg_score(results: list[dict], ground_truth: list[dict], k: int = 10) -> float:
    top_k = results[:k]
    dcg = 0.0
    for i, r in enumerate(top_k):
        rel = max((gt.get("relevance", 1) for gt in ground_truth if is_match(r, gt)), default=0)
        dcg += rel / math.log2(i + 2)
    ideal_rels = sorted([gt.get("relevance", 1) for gt in ground_truth], reverse=True)[:k]
    ideal_dcg = sum(rel / math.log2(i + 2) for i, rel in enumerate(ideal_rels))
    return dcg / ideal_dcg if ideal_dcg > 0 else 0.0


def run_model_benchmark(model_cfg: dict, tasks: list[dict]) -> dict:
    """Run a full benchmark with one embedding model."""
    print(f"\n{'=' * 60}")
    print(f"Benchmarking: {model_cfg['name']} ({model_cfg['description']})")
    print(f"{'=' * 60}")

    # Index repos needed by tasks
    repos_needed = set(t["repo"] for t in tasks)
    indexes = {}
    total_index_time = 0.0

    for repo_name in sorted(repos_needed):
        idx = index_repo(repo_name, model_cfg)
        indexes[repo_name] = idx
        total_index_time += idx.get("index_time", 0)

    # Run all tasks
    per_task = []
    latencies = []

    for task in tasks:
        repo_name = task["repo"]
        if repo_name not in indexes:
            continue

        results, latency = search(task["query"], indexes[repo_name], model_cfg)
        latencies.append(latency)

        gt = task["ground_truth"]
        metrics = {
            "recall_at_1": recall_at_k(results, gt, 1),
            "recall_at_5": recall_at_k(results, gt, 5),
            "recall_at_10": recall_at_k(results, gt, 10),
            "mrr": mrr_score(results, gt),
            "ndcg": ndcg_score(results, gt),
        }

        per_task.append({
            "task_id": task["id"],
            "category": task["category"],
            "metrics": metrics,
            "latency_ms": latency,
            "result_count": len(results),
        })

    # Compute aggregates
    by_category = defaultdict(list)
    for t in per_task:
        by_category[t["category"]].append(t)

    per_category = {}
    for cat, cat_tasks in sorted(by_category.items()):
        n = len(cat_tasks)
        per_category[cat] = {
            "recall_at_1": sum(t["metrics"]["recall_at_1"] for t in cat_tasks) / n,
            "recall_at_5": sum(t["metrics"]["recall_at_5"] for t in cat_tasks) / n,
            "recall_at_10": sum(t["metrics"]["recall_at_10"] for t in cat_tasks) / n,
            "mrr": sum(t["metrics"]["mrr"] for t in cat_tasks) / n,
            "ndcg": sum(t["metrics"]["ndcg"] for t in cat_tasks) / n,
            "task_count": n,
        }

    n_total = len(per_task)
    aggregate = {
        "recall_at_1": sum(t["metrics"]["recall_at_1"] for t in per_task) / n_total if n_total else 0,
        "recall_at_5": sum(t["metrics"]["recall_at_5"] for t in per_task) / n_total if n_total else 0,
        "recall_at_10": sum(t["metrics"]["recall_at_10"] for t in per_task) / n_total if n_total else 0,
        "mrr": sum(t["metrics"]["mrr"] for t in per_task) / n_total if n_total else 0,
        "ndcg": sum(t["metrics"]["ndcg"] for t in per_task) / n_total if n_total else 0,
    }

    latencies.sort()
    p50 = latencies[len(latencies) // 2] if latencies else 0
    p95_idx = int(len(latencies) * 0.95) if latencies else 0
    p95 = latencies[min(p95_idx, len(latencies) - 1)] if latencies else 0

    dim = 0
    for idx in indexes.values():
        if idx.get("dim"):
            dim = idx["dim"]
            break

    report = {
        "model_name": model_cfg["name"],
        "model_id": model_cfg["model_id"],
        "description": model_cfg["description"],
        "embedding_dim": dim,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "per_task": per_task,
        "per_category": per_category,
        "aggregate": aggregate,
        "performance": {
            "total_index_time_secs": total_index_time,
            "latency_p50_ms": p50,
            "latency_p95_ms": p95,
        },
        "task_count": n_total,
    }

    return report


def print_comparison(reports: list[dict]):
    """Print a comparison table of all models."""
    print(f"\n{'=' * 80}")
    print("EMBEDDING MODEL COMPARISON RESULTS")
    print(f"{'=' * 80}\n")

    # Overall
    print("── Overall Aggregate Metrics ──────────────────────────────────────────")
    header = f"{'Metric':<16}"
    for r in reports:
        header += f" {r['model_name']:>22}"
    print(header)
    print("─" * (16 + 23 * len(reports)))

    for metric in ["recall_at_1", "recall_at_5", "recall_at_10", "mrr", "ndcg"]:
        label = {"recall_at_1": "Recall@1", "recall_at_5": "Recall@5",
                 "recall_at_10": "Recall@10", "mrr": "MRR", "ndcg": "nDCG@10"}[metric]
        row = f"{label:<16}"
        values = [r["aggregate"][metric] for r in reports]
        best = max(values)
        for v in values:
            marker = " *" if v == best and values.count(best) == 1 else "  "
            row += f" {v:>20.4f}{marker}"
        print(row)

    print()
    print(f"{'Embedding dim':<16}", end="")
    for r in reports:
        print(f" {r['embedding_dim']:>22}", end="")
    print()

    print(f"{'Index time (s)':<16}", end="")
    for r in reports:
        print(f" {r['performance']['total_index_time_secs']:>22.1f}", end="")
    print()

    print(f"{'Latency p50 (ms)':<16}", end="")
    for r in reports:
        print(f" {r['performance']['latency_p50_ms']:>22.1f}", end="")
    print()

    # Per-category
    categories = sorted(set(c for r in reports for c in r["per_category"]))
    for cat in categories:
        print(f"\n── {cat} ──")
        header = f"{'Metric':<16}"
        for r in reports:
            header += f" {r['model_name']:>22}"
        print(header)
        print("─" * (16 + 23 * len(reports)))

        for metric in ["recall_at_5", "recall_at_10", "mrr", "ndcg"]:
            label = {"recall_at_5": "Recall@5", "recall_at_10": "Recall@10",
                     "mrr": "MRR", "ndcg": "nDCG@10"}[metric]
            row = f"{label:<16}"
            values = []
            for r in reports:
                v = r["per_category"].get(cat, {}).get(metric, 0)
                values.append(v)
            best = max(values) if values else 0
            for v in values:
                marker = " *" if v == best and values.count(best) == 1 else "  "
                row += f" {v:>20.4f}{marker}"
            print(row)


def main():
    # Verify API credentials
    if not os.environ.get("EMBEDDING_MODEL_API_KEY"):
        print("ERROR: EMBEDDING_MODEL_API_KEY not set. Run: set -a; source secrets.env; set +a")
        sys.exit(1)

    tasks = load_tasks()
    print(f"Loaded {len(tasks)} benchmark tasks")

    reports = []
    for model_cfg in MODELS:
        if not model_cfg["api_key"]:
            print(f"Skipping {model_cfg['name']}: no API key")
            continue
        try:
            report = run_model_benchmark(model_cfg, tasks)
            reports.append(report)

            # Save individual result
            out_path = RESULTS_DIR / f"embedding_{model_cfg['name'].lower().replace('/', '-')}.json"
            with open(out_path, "w") as f:
                json.dump(report, f, indent=2)
            print(f"  Saved results to {out_path}")
        except Exception as e:
            print(f"ERROR running {model_cfg['name']}: {e}")
            import traceback
            traceback.print_exc()

    if len(reports) >= 2:
        print_comparison(reports)

        # Save combined results
        combined_path = RESULTS_DIR / "embedding_comparison.json"
        with open(combined_path, "w") as f:
            json.dump({"models": reports, "timestamp": datetime.now(timezone.utc).isoformat()}, f, indent=2)
        print(f"\nCombined results saved to {combined_path}")
    else:
        print("ERROR: Need at least 2 model results for comparison")
        sys.exit(1)


if __name__ == "__main__":
    main()
