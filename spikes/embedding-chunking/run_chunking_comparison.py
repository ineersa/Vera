#!/usr/bin/env python3
"""
Chunking Strategy Comparison Spike for Vera.

Compares chunking strategies on Vera's benchmark task suite using the chosen
embedding model (Qwen3-Embedding-8B). Measures retrieval quality and token
efficiency across different chunking approaches.

Strategies compared:
  1. Sliding-window (50 lines, 10 overlap) — baseline from vector-only
  2. File-level (whole file per chunk, split at 150 lines)
  3. Symbol-aware (tree-sitter AST-based) — functions, classes, structs

Usage:
    set -a; source secrets.env; set +a
    python3 spikes/embedding-chunking/run_chunking_comparison.py
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

# Use Qwen3-Embedding-8B for all chunking comparisons (the strongest model)
EMBED_CFG = {
    "name": "Qwen3-Embedding-8B",
    "model_id": "Qwen/Qwen3-Embedding-8B",
    "api_base": os.environ.get("EMBEDDING_MODEL_BASE_URL", ""),
    "api_key": os.environ.get("EMBEDDING_MODEL_API_KEY", ""),
}

# File extensions to index
SOURCE_EXTENSIONS = {
    ".rs", ".py", ".js", ".ts", ".tsx", ".go", ".java", ".c", ".cpp",
    ".h", ".hpp", ".rb", ".toml", ".json", ".yaml", ".yml",
    ".css", ".html", ".sh",
}

# Tree-sitter language map
LANG_MAP = {
    ".rs": "rust", ".py": "python", ".js": "javascript", ".ts": "typescript",
    ".tsx": "tsx", ".go": "go", ".java": "java", ".c": "c", ".cpp": "cpp",
    ".h": "c", ".hpp": "cpp", ".rb": "ruby",
}

# Tree-sitter symbol node types per language
SYMBOL_QUERIES = {
    "rust": [
        "function_item", "impl_item", "struct_item", "enum_item",
        "trait_item", "type_item", "mod_item", "const_item", "static_item",
    ],
    "python": [
        "function_definition", "class_definition", "decorated_definition",
    ],
    "javascript": [
        "function_declaration", "class_declaration", "method_definition",
        "arrow_function", "function", "export_statement",
    ],
    "typescript": [
        "function_declaration", "class_declaration", "method_definition",
        "arrow_function", "function", "interface_declaration",
        "type_alias_declaration", "enum_declaration", "export_statement",
    ],
    "tsx": [
        "function_declaration", "class_declaration", "method_definition",
        "arrow_function", "function", "interface_declaration",
        "type_alias_declaration", "enum_declaration", "export_statement",
    ],
    "go": [
        "function_declaration", "method_declaration", "type_declaration",
    ],
    "java": [
        "method_declaration", "class_declaration", "interface_declaration",
        "enum_declaration",
    ],
    "c": [
        "function_definition", "struct_specifier", "enum_specifier",
        "type_definition",
    ],
    "cpp": [
        "function_definition", "class_specifier", "struct_specifier",
        "enum_specifier", "template_declaration", "namespace_definition",
    ],
    "ruby": [
        "method", "class", "module", "singleton_method",
    ],
}


def load_tasks() -> list[dict]:
    """Load benchmark tasks."""
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


def embed_batch(texts: list[str]) -> list[list[float]]:
    """Get embeddings from the API."""
    import urllib.request
    import urllib.error

    url = f"{EMBED_CFG['api_base']}/embeddings"
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {EMBED_CFG['api_key']}",
    }
    payload = {
        "input": texts,
        "model": EMBED_CFG["model_id"],
    }

    req = urllib.request.Request(
        url, data=json.dumps(payload).encode(), headers=headers, method="POST",
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
    dot = sum(x * y for x, y in zip(a, b))
    norm_a = math.sqrt(sum(x * x for x in a))
    norm_b = math.sqrt(sum(x * x for x in b))
    if norm_a == 0 or norm_b == 0:
        return 0.0
    return dot / (norm_a * norm_b)


def get_source_files(repo_path: str, max_files: int = 300) -> list[str]:
    """Get source files using ripgrep."""
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
            if ext not in SOURCE_EXTENSIONS:
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
    return (source_files + other_files)[:max_files]


# ─── Chunking Strategy 1: Sliding Window ───────────────────────────────────

def chunk_sliding_window(file_path: str, repo_path: str,
                          chunk_size: int = 50, overlap: int = 10) -> list[dict]:
    """Sliding-window line-based chunking."""
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


# ─── Chunking Strategy 2: File-Level ───────────────────────────────────────

def chunk_file_level(file_path: str, repo_path: str,
                      max_lines: int = 150) -> list[dict]:
    """File-level chunking: whole file as one chunk, split at max_lines."""
    abs_path = os.path.join(repo_path, file_path)
    try:
        with open(abs_path, "r", errors="replace") as f:
            lines = f.readlines()
    except (OSError, UnicodeDecodeError):
        return []

    if not lines:
        return []

    chunks = []
    if len(lines) <= max_lines:
        content = "".join(lines)
        chunks.append({
            "file_path": file_path,
            "line_start": 1,
            "line_end": len(lines),
            "content": content,
        })
    else:
        # Split into segments
        i = 0
        while i < len(lines):
            end = min(i + max_lines, len(lines))
            content = "".join(lines[i:end])
            chunks.append({
                "file_path": file_path,
                "line_start": i + 1,
                "line_end": end,
                "content": content,
            })
            i = end

    return chunks


# ─── Chunking Strategy 3: Symbol-Aware (tree-sitter) ──────────────────────

def chunk_symbol_aware(file_path: str, repo_path: str,
                        max_symbol_lines: int = 150) -> list[dict]:
    """Symbol-aware chunking using regex-based heuristic symbol detection.

    Detects top-level function/class/struct/impl/trait definitions by
    indentation and keyword patterns. This approximates what tree-sitter
    AST parsing would provide. Falls back to sliding window for unsupported
    languages or files with no detected symbols.
    """
    import re

    ext = os.path.splitext(file_path)[1].lower()
    language = LANG_MAP.get(ext)

    if not language:
        return chunk_sliding_window(file_path, repo_path, chunk_size=50, overlap=10)

    abs_path = os.path.join(repo_path, file_path)
    try:
        with open(abs_path, "r", errors="replace") as f:
            lines = f.readlines()
    except (OSError, UnicodeDecodeError):
        return []

    if not lines:
        return []

    # Language-specific symbol detection patterns (top-level definitions)
    patterns = _get_symbol_patterns(language)
    if not patterns:
        return chunk_sliding_window(file_path, repo_path)

    # Detect symbol boundaries
    symbols = _detect_symbols(lines, patterns, language)

    if not symbols:
        return chunk_sliding_window(file_path, repo_path)

    chunks = []
    for sym in symbols:
        start_line = sym["line_start"]
        end_line = min(sym["line_end"], len(lines))
        sym_lines = end_line - start_line + 1

        if sym_lines <= max_symbol_lines:
            chunk_content = "".join(lines[start_line - 1:end_line])
            chunks.append({
                "file_path": file_path,
                "line_start": start_line,
                "line_end": end_line,
                "content": chunk_content,
                "symbol_type": sym.get("type", "symbol"),
            })
        else:
            # Split large symbols
            i = start_line - 1
            while i < end_line:
                sub_end = min(i + max_symbol_lines, end_line)
                chunk_content = "".join(lines[i:sub_end])
                chunks.append({
                    "file_path": file_path,
                    "line_start": i + 1,
                    "line_end": sub_end,
                    "content": chunk_content,
                    "symbol_type": sym.get("type", "symbol"),
                })
                i = sub_end

    # Add gaps between symbols (imports, module-level code)
    covered = set()
    for sym in symbols:
        for line in range(sym["line_start"], sym["line_end"] + 1):
            covered.add(line)

    gap_start = None
    for line_num in range(1, len(lines) + 1):
        if line_num not in covered:
            if gap_start is None:
                gap_start = line_num
        else:
            if gap_start is not None:
                gap_end = line_num - 1
                if gap_end - gap_start >= 3:
                    gap_content = "".join(lines[gap_start - 1:gap_end])
                    chunks.append({
                        "file_path": file_path,
                        "line_start": gap_start,
                        "line_end": gap_end,
                        "content": gap_content,
                        "symbol_type": "gap",
                    })
                gap_start = None

    if gap_start is not None and len(lines) - gap_start >= 3:
        gap_content = "".join(lines[gap_start - 1:])
        chunks.append({
            "file_path": file_path,
            "line_start": gap_start,
            "line_end": len(lines),
            "content": gap_content,
            "symbol_type": "gap",
        })

    chunks.sort(key=lambda c: c["line_start"])
    return chunks if chunks else chunk_sliding_window(file_path, repo_path)


def _get_symbol_patterns(language: str) -> list[tuple[str, "re.Pattern"]]:
    """Get regex patterns for detecting symbol definitions in a language."""
    import re

    patterns = {
        "rust": [
            ("function", re.compile(r'^(\s*)(?:pub\s+)?(?:async\s+)?fn\s+\w+')),
            ("struct", re.compile(r'^(\s*)(?:pub\s+)?struct\s+\w+')),
            ("enum", re.compile(r'^(\s*)(?:pub\s+)?enum\s+\w+')),
            ("impl", re.compile(r'^(\s*)impl\b')),
            ("trait", re.compile(r'^(\s*)(?:pub\s+)?trait\s+\w+')),
            ("type", re.compile(r'^(\s*)(?:pub\s+)?type\s+\w+')),
            ("mod", re.compile(r'^(\s*)(?:pub\s+)?mod\s+\w+')),
        ],
        "python": [
            ("function", re.compile(r'^(\s*)(?:async\s+)?def\s+\w+')),
            ("class", re.compile(r'^(\s*)class\s+\w+')),
        ],
        "javascript": [
            ("function", re.compile(r'^(\s*)(?:export\s+)?(?:async\s+)?function\s+\w+')),
            ("class", re.compile(r'^(\s*)(?:export\s+)?class\s+\w+')),
            ("const_fn", re.compile(r'^(\s*)(?:export\s+)?(?:const|let|var)\s+\w+\s*=\s*(?:async\s+)?(?:\([^)]*\)|[\w]+)\s*=>')),
        ],
        "typescript": [
            ("function", re.compile(r'^(\s*)(?:export\s+)?(?:async\s+)?function\s+\w+')),
            ("class", re.compile(r'^(\s*)(?:export\s+)?class\s+\w+')),
            ("interface", re.compile(r'^(\s*)(?:export\s+)?interface\s+\w+')),
            ("type", re.compile(r'^(\s*)(?:export\s+)?type\s+\w+')),
            ("enum", re.compile(r'^(\s*)(?:export\s+)?enum\s+\w+')),
            ("const_fn", re.compile(r'^(\s*)(?:export\s+)?(?:const|let|var)\s+\w+\s*=\s*(?:async\s+)?(?:\([^)]*\)|[\w]+)\s*=>')),
        ],
        "go": [
            ("function", re.compile(r'^func\s+')),
            ("type", re.compile(r'^type\s+\w+')),
        ],
    }
    # tsx uses same as typescript
    patterns["tsx"] = patterns.get("typescript", [])
    patterns["java"] = [
        ("class", __import__("re").compile(r'^(\s*)(?:public\s+|private\s+|protected\s+)?(?:abstract\s+)?(?:static\s+)?class\s+\w+')),
        ("interface", __import__("re").compile(r'^(\s*)(?:public\s+|private\s+)?interface\s+\w+')),
        ("method", __import__("re").compile(r'^(\s+)(?:public\s+|private\s+|protected\s+)?(?:static\s+)?(?:abstract\s+)?[\w<>\[\],\s]+\s+\w+\s*\(')),
    ]
    patterns["c"] = [
        ("function", __import__("re").compile(r'^(?!static\s+inline\s+)[\w*]+\s+[\w*]+\s*\([^;]*$')),
    ]
    patterns["cpp"] = patterns.get("c", [])
    patterns["ruby"] = [
        ("method", __import__("re").compile(r'^(\s*)def\s+\w+')),
        ("class", __import__("re").compile(r'^(\s*)class\s+\w+')),
        ("module", __import__("re").compile(r'^(\s*)module\s+\w+')),
    ]

    return patterns.get(language, [])


def _detect_symbols(lines: list[str], patterns: list[tuple[str, "re.Pattern"]],
                     language: str) -> list[dict]:
    """Detect symbol boundaries using indent-based heuristics.

    For brace-delimited languages (Rust, JS, TS, Go, Java, C):
      Track brace depth from symbol start.
    For indent-delimited languages (Python, Ruby):
      Track indentation level from symbol start.
    """
    symbols = []
    i = 0
    brace_langs = {"rust", "javascript", "typescript", "tsx", "go", "java", "c", "cpp"}
    indent_langs = {"python", "ruby"}

    while i < len(lines):
        line = lines[i]
        matched = None
        for sym_type, pattern in patterns:
            m = pattern.match(line)
            if m:
                indent = len(m.group(1)) if m.lastindex and m.lastindex >= 1 else 0
                matched = (sym_type, indent)
                break

        if matched:
            sym_type, start_indent = matched
            start_line = i + 1  # 1-based

            if language in brace_langs:
                end_line = _find_brace_end(lines, i)
            elif language in indent_langs:
                end_line = _find_indent_end(lines, i, start_indent)
            else:
                end_line = _find_brace_end(lines, i)

            # Minimum 2 lines
            if end_line - start_line >= 1:
                symbols.append({
                    "type": sym_type,
                    "line_start": start_line,
                    "line_end": end_line,
                })
            i = end_line  # continue after this symbol
        else:
            i += 1

    return symbols


def _find_brace_end(lines: list[str], start_idx: int) -> int:
    """Find end of a brace-delimited block starting at start_idx."""
    depth = 0
    found_open = False

    for i in range(start_idx, len(lines)):
        line = lines[i]
        # Skip string contents roughly
        for ch in line:
            if ch == '{':
                depth += 1
                found_open = True
            elif ch == '}':
                depth -= 1
                if found_open and depth <= 0:
                    return i + 1  # 1-based

    # If no closing brace found, return a reasonable range
    return min(start_idx + 30, len(lines))


def _find_indent_end(lines: list[str], start_idx: int, start_indent: int) -> int:
    """Find end of an indent-delimited block (Python/Ruby)."""
    # The body starts on the next line after the definition
    for i in range(start_idx + 1, len(lines)):
        line = lines[i]
        stripped = line.rstrip()
        if not stripped:  # empty line
            continue
        indent = len(line) - len(line.lstrip())
        if indent <= start_indent and stripped:
            return i  # 1-based (the line before this one ends the block)

    return len(lines)  # Block extends to end of file


# ─── Benchmark Infrastructure ──────────────────────────────────────────────

def index_repo_with_strategy(repo_name: str, strategy_fn, strategy_name: str) -> dict:
    """Index a repo with a given chunking strategy."""
    repo_path = str(BENCH_REPOS / repo_name)
    if not os.path.isdir(repo_path):
        print(f"  WARNING: Repo {repo_name} not found")
        return {"chunks": [], "embeddings": []}

    files = get_source_files(repo_path)
    print(f"  [{strategy_name}] Found {len(files)} source files in {repo_name}")

    # Chunk all files
    all_chunks = []
    for f in files:
        all_chunks.extend(strategy_fn(f, repo_path))

    total_tokens = sum(len(c["content"]) // 4 for c in all_chunks)
    print(f"  [{strategy_name}] Created {len(all_chunks)} chunks, ~{total_tokens} tokens")

    if not all_chunks:
        return {"chunks": [], "embeddings": [], "chunk_count": 0, "total_tokens": 0}

    # Embed in batches
    batch_size = 32
    all_embeddings = []
    start_time = time.time()

    for i in range(0, len(all_chunks), batch_size):
        batch = all_chunks[i:i + batch_size]
        texts = [c["content"][:2000] for c in batch]
        try:
            embeddings = embed_batch(texts)
            all_embeddings.extend(embeddings)
        except Exception as e:
            print(f"  [{strategy_name}] Embedding batch failed: {e}")
            dim = len(all_embeddings[0]) if all_embeddings else 768
            all_embeddings.extend([[0.0] * dim] * len(batch))

        if (i // batch_size) % 20 == 0 and i > 0:
            print(f"  [{strategy_name}] Embedded {min(i + batch_size, len(all_chunks))}/{len(all_chunks)}")

    index_time = time.time() - start_time
    print(f"  [{strategy_name}] Indexed {repo_name}: {len(all_chunks)} chunks in {index_time:.1f}s")

    return {
        "chunks": all_chunks,
        "embeddings": all_embeddings,
        "chunk_count": len(all_chunks),
        "total_tokens": total_tokens,
        "index_time": index_time,
    }


def search_index(query: str, index: dict, max_results: int = 20) -> tuple[list[dict], float]:
    """Search by embedding query and cosine similarity."""
    if not index["chunks"]:
        return [], 0.0

    start = time.time()
    try:
        query_emb = embed_batch([query])[0]
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


# Metrics
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


def run_chunking_benchmark(strategy_name: str, strategy_fn, tasks: list[dict]) -> dict:
    """Run a full benchmark with one chunking strategy."""
    print(f"\n{'=' * 60}")
    print(f"Chunking Strategy: {strategy_name}")
    print(f"{'=' * 60}")

    repos_needed = set(t["repo"] for t in tasks)
    indexes = {}
    total_index_time = 0.0
    total_chunks = 0
    total_tokens = 0

    for repo_name in sorted(repos_needed):
        idx = index_repo_with_strategy(repo_name, strategy_fn, strategy_name)
        indexes[repo_name] = idx
        total_index_time += idx.get("index_time", 0)
        total_chunks += idx.get("chunk_count", 0)
        total_tokens += idx.get("total_tokens", 0)

    per_task = []
    latencies = []

    for task in tasks:
        repo_name = task["repo"]
        if repo_name not in indexes:
            continue

        results, latency = search_index(task["query"], indexes[repo_name])
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
        })

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

    report = {
        "strategy_name": strategy_name,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "per_task": per_task,
        "per_category": per_category,
        "aggregate": aggregate,
        "efficiency": {
            "total_chunks": total_chunks,
            "total_tokens": total_tokens,
            "total_index_time_secs": total_index_time,
            "latency_p50_ms": p50,
            "latency_p95_ms": p95,
        },
        "task_count": n_total,
    }

    return report


def print_comparison(reports: list[dict]):
    """Print comparison table."""
    print(f"\n{'=' * 80}")
    print("CHUNKING STRATEGY COMPARISON RESULTS")
    print(f"{'=' * 80}\n")

    print("── Overall Aggregate Metrics ──────────────────────────────────────────")
    header = f"{'Metric':<16}"
    for r in reports:
        header += f" {r['strategy_name']:>22}"
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

    # Token efficiency
    print()
    print("── Token Efficiency ───────────────────────────────────────────────────")
    print(f"{'Total chunks':<16}", end="")
    for r in reports:
        print(f" {r['efficiency']['total_chunks']:>22}", end="")
    print()

    print(f"{'Total tokens':<16}", end="")
    for r in reports:
        print(f" {r['efficiency']['total_tokens']:>22}", end="")
    print()

    # Relative to sliding window (first report)
    if reports:
        base_tokens = reports[0]["efficiency"]["total_tokens"]
        print(f"{'Token ratio':<16}", end="")
        for r in reports:
            ratio = r["efficiency"]["total_tokens"] / base_tokens if base_tokens else 0
            print(f" {ratio:>22.2f}", end="")
        print()

    print(f"{'Index time (s)':<16}", end="")
    for r in reports:
        print(f" {r['efficiency']['total_index_time_secs']:>22.1f}", end="")
    print()

    # Per-category
    categories = sorted(set(c for r in reports for c in r["per_category"]))
    for cat in categories:
        print(f"\n── {cat} ──")
        header = f"{'Metric':<16}"
        for r in reports:
            header += f" {r['strategy_name']:>22}"
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
    if not os.environ.get("EMBEDDING_MODEL_API_KEY"):
        print("ERROR: EMBEDDING_MODEL_API_KEY not set. Run: set -a; source secrets.env; set +a")
        sys.exit(1)

    tasks = load_tasks()
    print(f"Loaded {len(tasks)} benchmark tasks")

    strategies = [
        ("sliding-window", chunk_sliding_window),
        ("file-level", chunk_file_level),
        ("symbol-aware", chunk_symbol_aware),
    ]

    reports = []
    for strategy_name, strategy_fn in strategies:
        try:
            report = run_chunking_benchmark(strategy_name, strategy_fn, tasks)
            reports.append(report)

            out_path = RESULTS_DIR / f"chunking_{strategy_name}.json"
            with open(out_path, "w") as f:
                json.dump(report, f, indent=2)
            print(f"  Saved results to {out_path}")
        except Exception as e:
            print(f"ERROR running {strategy_name}: {e}")
            import traceback
            traceback.print_exc()

    if len(reports) >= 2:
        print_comparison(reports)

        combined_path = RESULTS_DIR / "chunking_comparison.json"
        with open(combined_path, "w") as f:
            json.dump({"strategies": reports, "timestamp": datetime.now(timezone.utc).isoformat()}, f, indent=2)
        print(f"\nCombined results saved to {combined_path}")
    else:
        print("ERROR: Need at least 2 strategy results for comparison")
        sys.exit(1)


if __name__ == "__main__":
    main()
