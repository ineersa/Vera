#!/usr/bin/env python3
"""
Run remaining embedding models (bge-en-icl, Qwen3-0.6B) efficiently.

Reuses the same chunking as the Qwen3-8B run. Embeds and searches with
smaller batch sizes to avoid timeout issues.
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

SOURCE_EXTENSIONS = {
    ".rs", ".py", ".js", ".ts", ".tsx", ".go", ".java", ".c", ".cpp",
    ".h", ".hpp", ".rb", ".toml", ".json", ".yaml", ".yml",
    ".css", ".html", ".sh",
}


def load_tasks():
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


def embed_batch(texts, api_base, api_key, model_id):
    import urllib.request
    import urllib.error

    url = f"{api_base}/embeddings"
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {api_key}",
    }
    payload = {"input": texts, "model": model_id}

    req = urllib.request.Request(
        url, data=json.dumps(payload).encode(), headers=headers, method="POST",
    )

    with urllib.request.urlopen(req, timeout=120) as resp:
        data = json.loads(resp.read())
    return [item["embedding"] for item in data["data"]]


def cosine_similarity(a, b):
    dot = sum(x * y for x, y in zip(a, b))
    na = math.sqrt(sum(x * x for x in a))
    nb = math.sqrt(sum(x * x for x in b))
    return dot / (na * nb) if na and nb else 0.0


def get_source_files(repo_path, max_files=300):
    src, other = [], []
    try:
        r = subprocess.run(["rg", "--files", repo_path], capture_output=True, text=True, timeout=30)
        for line in r.stdout.strip().split("\n"):
            if not line:
                continue
            rel = line[len(repo_path):].lstrip("/") if line.startswith(repo_path) else line
            ext = os.path.splitext(rel)[1].lower()
            if ext not in SOURCE_EXTENSIONS:
                continue
            if any(p in rel for p in ["node_modules/", ".git/", "vendor/", "target/",
                                       "__pycache__/", ".venv/", "dist/", "build/",
                                       "test_fixtures/", "testdata/"]):
                continue
            if any(p in rel for p in ["src/", "lib/", "crates/", "packages/"]):
                src.append(rel)
            else:
                other.append(rel)
    except Exception:
        pass
    return (src + other)[:max_files]


def chunk_file(file_path, repo_path, chunk_size=50, overlap=10):
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
        chunks.append({"file_path": file_path, "line_start": i + 1, "line_end": end, "content": content})
        i += chunk_size - overlap
    return chunks


# Metrics
def is_match(r, gt):
    return r["file_path"] == gt["file_path"] and r["line_start"] <= gt["line_end"] and r["line_end"] >= gt["line_start"]

def recall_at_k(results, gt, k):
    if not gt:
        return 0.0
    found = sum(1 for g in gt if any(is_match(r, g) for r in results[:k]))
    return found / len(gt)

def mrr_fn(results, gt):
    for i, r in enumerate(results):
        if any(is_match(r, g) for g in gt):
            return 1.0 / (i + 1)
    return 0.0

def ndcg_fn(results, gt, k=10):
    top_k = results[:k]
    dcg = sum(max((g.get("relevance", 1) for g in gt if is_match(r, g)), default=0) / math.log2(i + 2)
              for i, r in enumerate(top_k))
    ideal = sorted([g.get("relevance", 1) for g in gt], reverse=True)[:k]
    idcg = sum(r / math.log2(i + 2) for i, r in enumerate(ideal))
    return dcg / idcg if idcg > 0 else 0.0


def run_model(name, model_id, api_base, api_key, description, tasks):
    print(f"\n{'='*60}")
    print(f"Model: {name} ({description})")
    print(f"{'='*60}")

    repos_needed = set(t["repo"] for t in tasks)
    indexes = {}
    total_index_time = 0.0

    for repo_name in sorted(repos_needed):
        repo_path = str(BENCH_REPOS / repo_name)
        if not os.path.isdir(repo_path):
            continue

        start = time.time()
        files = get_source_files(repo_path)
        print(f"  [{name}] {repo_name}: {len(files)} files")

        all_chunks = []
        for f in files:
            all_chunks.extend(chunk_file(f, repo_path))
        print(f"  [{name}] {repo_name}: {len(all_chunks)} chunks")

        all_embs = []
        bs = 20  # smaller batch size for reliability
        for i in range(0, len(all_chunks), bs):
            batch = all_chunks[i:i+bs]
            texts = [c["content"][:2000] for c in batch]
            try:
                embs = embed_batch(texts, api_base, api_key, model_id)
                all_embs.extend(embs)
            except Exception as e:
                print(f"  [{name}] Batch {i//bs} failed: {e}")
                dim = len(all_embs[0]) if all_embs else 768
                all_embs.extend([[0.0]*dim]*len(batch))

            if (i//bs) % 30 == 0 and i > 0:
                elapsed = time.time() - start
                print(f"  [{name}] {min(i+bs, len(all_chunks))}/{len(all_chunks)} ({elapsed:.0f}s)")

        idx_time = time.time() - start
        total_index_time += idx_time
        dim = len(all_embs[0]) if all_embs else 0
        indexes[repo_name] = {"chunks": all_chunks, "embeddings": all_embs}
        print(f"  [{name}] {repo_name} done: {idx_time:.1f}s, dim={dim}")

    # Run tasks
    per_task = []
    latencies = []
    for task in tasks:
        repo = task["repo"]
        if repo not in indexes:
            continue
        idx = indexes[repo]
        if not idx["chunks"]:
            continue

        st = time.time()
        try:
            qe = embed_batch([task["query"]], api_base, api_key, model_id)[0]
        except Exception:
            per_task.append({"task_id": task["id"], "category": task["category"],
                            "metrics": {"recall_at_1":0,"recall_at_5":0,"recall_at_10":0,"mrr":0,"ndcg":0},
                            "latency_ms": 0, "result_count": 0})
            continue

        scores = [(i, cosine_similarity(qe, e)) for i, e in enumerate(idx["embeddings"])]
        scores.sort(key=lambda x: -x[1])

        results = [{"file_path": idx["chunks"][i]["file_path"],
                     "line_start": idx["chunks"][i]["line_start"],
                     "line_end": idx["chunks"][i]["line_end"],
                     "score": s} for i, s in scores[:20]]
        lat = (time.time() - st) * 1000
        latencies.append(lat)

        gt = task["ground_truth"]
        metrics = {
            "recall_at_1": recall_at_k(results, gt, 1),
            "recall_at_5": recall_at_k(results, gt, 5),
            "recall_at_10": recall_at_k(results, gt, 10),
            "mrr": mrr_fn(results, gt),
            "ndcg": ndcg_fn(results, gt),
        }
        per_task.append({"task_id": task["id"], "category": task["category"],
                        "metrics": metrics, "latency_ms": lat, "result_count": len(results)})

    # Aggregates
    by_cat = defaultdict(list)
    for t in per_task:
        by_cat[t["category"]].append(t)

    per_category = {}
    for cat, ct in sorted(by_cat.items()):
        n = len(ct)
        per_category[cat] = {k: sum(t["metrics"][k] for t in ct)/n
                             for k in ["recall_at_1","recall_at_5","recall_at_10","mrr","ndcg"]}
        per_category[cat]["task_count"] = n

    n = len(per_task)
    aggregate = {k: sum(t["metrics"][k] for t in per_task)/n if n else 0
                 for k in ["recall_at_1","recall_at_5","recall_at_10","mrr","ndcg"]}

    latencies.sort()
    p50 = latencies[len(latencies)//2] if latencies else 0
    p95 = latencies[min(int(len(latencies)*0.95), len(latencies)-1)] if latencies else 0

    report = {
        "model_name": name, "model_id": model_id, "description": description,
        "embedding_dim": dim,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "per_task": per_task, "per_category": per_category, "aggregate": aggregate,
        "performance": {"total_index_time_secs": total_index_time,
                       "latency_p50_ms": p50, "latency_p95_ms": p95},
        "task_count": n,
    }
    return report


def main():
    eb = os.environ.get("EMBEDDING_MODEL_BASE_URL", "")
    ek = os.environ.get("EMBEDDING_MODEL_API_KEY", "")
    rk = os.environ.get("RERANKER_MODEL_API_KEY", "")

    if not ek:
        print("ERROR: Set EMBEDDING_MODEL_API_KEY")
        sys.exit(1)

    tasks = load_tasks()
    print(f"Loaded {len(tasks)} tasks")

    models = [
        ("bge-en-icl", "BAAI/bge-en-icl", eb, ek,
         "General-purpose ICL embedding model by BAAI"),
        ("Qwen3-Embedding-0.6B", "Qwen/Qwen3-Embedding-0.6B",
         "https://api.siliconflow.com/v1", rk,
         "Lightweight 0.6B model, 1024-dim, fast inference"),
    ]

    for name, mid, ab, ak, desc in models:
        try:
            report = run_model(name, mid, ab, ak, desc, tasks)
            out = RESULTS_DIR / f"embedding_{name.lower().replace('/', '-')}.json"
            with open(out, "w") as f:
                json.dump(report, f, indent=2)
            print(f"\nSaved: {out}")
            print(f"  Aggregate: R@5={report['aggregate']['recall_at_5']:.4f}, "
                  f"R@10={report['aggregate']['recall_at_10']:.4f}, "
                  f"MRR={report['aggregate']['mrr']:.4f}")
        except Exception as e:
            print(f"ERROR {name}: {e}")
            import traceback
            traceback.print_exc()


if __name__ == "__main__":
    main()
