#!/usr/bin/env python3
"""Run Qwen3-Embedding-0.6B benchmark with retry logic for SiliconFlow API."""

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

API_BASE = "https://api.siliconflow.com/v1"
API_KEY = os.environ.get("RERANKER_MODEL_API_KEY", "")
MODEL_ID = "Qwen/Qwen3-Embedding-0.6B"

SOURCE_EXTENSIONS = {
    ".rs", ".py", ".js", ".ts", ".tsx", ".go", ".java", ".c", ".cpp",
    ".h", ".hpp", ".rb", ".toml", ".json", ".yaml", ".yml", ".css", ".html", ".sh",
}


def load_tasks():
    tasks = []
    for path in sorted(TASKS_DIR.glob("*.json")):
        with open(path) as f:
            data = json.load(f)
        tasks.extend(data if isinstance(data, list) else [data])
    tasks.sort(key=lambda t: t["id"])
    return tasks


def embed_batch_with_retry(texts, max_retries=3):
    import urllib.request, urllib.error
    url = f"{API_BASE}/embeddings"
    headers = {"Content-Type": "application/json", "Authorization": f"Bearer {API_KEY}"}
    payload = {"input": texts, "model": MODEL_ID}

    for attempt in range(max_retries):
        try:
            req = urllib.request.Request(url, data=json.dumps(payload).encode(), headers=headers, method="POST")
            with urllib.request.urlopen(req, timeout=120) as resp:
                data = json.loads(resp.read())
            return [item["embedding"] for item in data["data"]]
        except Exception as e:
            if attempt < max_retries - 1:
                wait = 2 ** (attempt + 1)
                print(f"    Retry {attempt+1}/{max_retries} after {wait}s: {e}")
                time.sleep(wait)
            else:
                raise


def cosine_similarity(a, b):
    dot = sum(x*y for x, y in zip(a, b))
    na = math.sqrt(sum(x*x for x in a))
    nb = math.sqrt(sum(x*x for x in b))
    return dot / (na * nb) if na and nb else 0.0


def get_source_files(repo_path, max_files=300):
    src, other = [], []
    try:
        r = subprocess.run(["rg", "--files", repo_path], capture_output=True, text=True, timeout=30)
        for line in r.stdout.strip().split("\n"):
            if not line: continue
            rel = line[len(repo_path):].lstrip("/") if line.startswith(repo_path) else line
            ext = os.path.splitext(rel)[1].lower()
            if ext not in SOURCE_EXTENSIONS: continue
            if any(p in rel for p in ["node_modules/",".git/","vendor/","target/","__pycache__/",".venv/","dist/","build/","test_fixtures/","testdata/"]): continue
            (src if any(p in rel for p in ["src/","lib/","crates/","packages/"]) else other).append(rel)
    except Exception: pass
    return (src + other)[:max_files]


def chunk_file(fp, repo_path, size=50, overlap=10):
    try:
        with open(os.path.join(repo_path, fp), "r", errors="replace") as f:
            lines = f.readlines()
    except: return []
    if not lines: return []
    chunks, i = [], 0
    while i < len(lines):
        end = min(i + size, len(lines))
        chunks.append({"file_path": fp, "line_start": i+1, "line_end": end, "content": "".join(lines[i:end])})
        i += size - overlap
    return chunks


def is_match(r, gt):
    return r["file_path"] == gt["file_path"] and r["line_start"] <= gt["line_end"] and r["line_end"] >= gt["line_start"]

def recall_at_k(results, gt, k):
    if not gt: return 0.0
    return sum(1 for g in gt if any(is_match(r, g) for r in results[:k])) / len(gt)

def mrr_fn(results, gt):
    for i, r in enumerate(results):
        if any(is_match(r, g) for g in gt): return 1.0 / (i + 1)
    return 0.0

def ndcg_fn(results, gt, k=10):
    top_k = results[:k]
    dcg = sum(max((g.get("relevance",1) for g in gt if is_match(r,g)), default=0) / math.log2(i+2) for i, r in enumerate(top_k))
    ideal = sorted([g.get("relevance",1) for g in gt], reverse=True)[:k]
    idcg = sum(r / math.log2(i+2) for i, r in enumerate(ideal))
    return dcg / idcg if idcg > 0 else 0.0


def main():
    if not API_KEY:
        print("ERROR: RERANKER_MODEL_API_KEY not set")
        sys.exit(1)

    tasks = load_tasks()
    print(f"Loaded {len(tasks)} tasks")
    print(f"Model: {MODEL_ID}")

    repos_needed = sorted(set(t["repo"] for t in tasks))
    indexes = {}
    total_index_time = 0.0

    for repo_name in repos_needed:
        repo_path = str(BENCH_REPOS / repo_name)
        if not os.path.isdir(repo_path): continue

        start = time.time()
        files = get_source_files(repo_path)
        all_chunks = []
        for f in files:
            all_chunks.extend(chunk_file(f, repo_path))
        print(f"  {repo_name}: {len(files)} files, {len(all_chunks)} chunks")

        all_embs = []
        bs = 16  # smaller batches for reliability
        for i in range(0, len(all_chunks), bs):
            batch = all_chunks[i:i+bs]
            texts = [c["content"][:2000] for c in batch]
            try:
                embs = embed_batch_with_retry(texts)
                all_embs.extend(embs)
            except Exception as e:
                print(f"  Batch {i//bs} FAILED permanently: {e}")
                dim = len(all_embs[0]) if all_embs else 1024
                all_embs.extend([[0.0]*dim]*len(batch))

            if (i//bs) % 30 == 0 and i > 0:
                print(f"  {min(i+bs, len(all_chunks))}/{len(all_chunks)} ({time.time()-start:.0f}s)")
            # Small delay between batches to avoid rate limits
            time.sleep(0.1)

        idx_time = time.time() - start
        total_index_time += idx_time
        dim = len(all_embs[0]) if all_embs else 0
        indexes[repo_name] = {"chunks": all_chunks, "embeddings": all_embs}
        print(f"  {repo_name} done: {idx_time:.1f}s, dim={dim}")

    # Run tasks
    per_task = []
    latencies = []
    for task in tasks:
        repo = task["repo"]
        if repo not in indexes: continue
        idx = indexes[repo]
        if not idx["chunks"]: continue

        st = time.time()
        try:
            qe = embed_batch_with_retry([task["query"]])[0]
        except:
            per_task.append({"task_id": task["id"], "category": task["category"],
                            "metrics": {k:0 for k in ["recall_at_1","recall_at_5","recall_at_10","mrr","ndcg"]},
                            "latency_ms": 0, "result_count": 0})
            continue

        scores = sorted([(i, cosine_similarity(qe, e)) for i, e in enumerate(idx["embeddings"])], key=lambda x: -x[1])
        results = [{"file_path": idx["chunks"][i]["file_path"], "line_start": idx["chunks"][i]["line_start"],
                     "line_end": idx["chunks"][i]["line_end"], "score": s} for i, s in scores[:20]]
        lat = (time.time() - st) * 1000
        latencies.append(lat)

        gt = task["ground_truth"]
        per_task.append({"task_id": task["id"], "category": task["category"],
                        "metrics": {"recall_at_1": recall_at_k(results, gt, 1),
                                   "recall_at_5": recall_at_k(results, gt, 5),
                                   "recall_at_10": recall_at_k(results, gt, 10),
                                   "mrr": mrr_fn(results, gt), "ndcg": ndcg_fn(results, gt)},
                        "latency_ms": lat, "result_count": len(results)})

    # Aggregates
    by_cat = defaultdict(list)
    for t in per_task: by_cat[t["category"]].append(t)

    per_category = {}
    for cat, ct in sorted(by_cat.items()):
        n = len(ct)
        per_category[cat] = {k: sum(t["metrics"][k] for t in ct)/n for k in ["recall_at_1","recall_at_5","recall_at_10","mrr","ndcg"]}
        per_category[cat]["task_count"] = n

    n = len(per_task)
    aggregate = {k: sum(t["metrics"][k] for t in per_task)/n if n else 0 for k in ["recall_at_1","recall_at_5","recall_at_10","mrr","ndcg"]}

    latencies.sort()
    p50 = latencies[len(latencies)//2] if latencies else 0
    p95 = latencies[min(int(len(latencies)*0.95), len(latencies)-1)] if latencies else 0

    report = {
        "model_name": "Qwen3-Embedding-0.6B", "model_id": MODEL_ID,
        "description": "Lightweight 0.6B model, 1024-dim, fast inference",
        "embedding_dim": dim, "timestamp": datetime.now(timezone.utc).isoformat(),
        "per_task": per_task, "per_category": per_category, "aggregate": aggregate,
        "performance": {"total_index_time_secs": total_index_time, "latency_p50_ms": p50, "latency_p95_ms": p95},
        "task_count": n,
    }

    out = RESULTS_DIR / "embedding_qwen3-embedding-0.6b.json"
    with open(out, "w") as f:
        json.dump(report, f, indent=2)
    print(f"\nSaved: {out}")
    print(f"Aggregate: R@5={aggregate['recall_at_5']:.4f}, R@10={aggregate['recall_at_10']:.4f}, MRR={aggregate['mrr']:.4f}")


if __name__ == "__main__":
    main()
