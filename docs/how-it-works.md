# How Vera Works

Vera's search pipeline has three stages: retrieve candidates, fuse results, then rerank. Every stage was chosen based on benchmarks against real codebases, not assumptions.

## Parsing: Tree-Sitter Chunks

Vera parses source files into ASTs using tree-sitter grammars compiled into the binary. Instead of splitting code into arbitrary line ranges or whole files, it extracts discrete symbols. functions, classes, structs, methods. as individual chunks.

Each chunk carries metadata: file path, line range, language, symbol name, and symbol type. This means search results map to actual code boundaries, not random slices.

Symbol-aware chunking scores 2.3× higher MRR on symbol lookup than sliding-window chunking (0.55 vs 0.24), while using 14% fewer tokens. Large symbols (>150 lines) are split at logical boundaries. Languages without a tree-sitter grammar fall back to sliding-window chunking.

## Retrieval: BM25 + Vector Search

Two retrieval paths run in parallel for every query:

**BM25 (keyword matching)** uses a Tantivy index over chunk content, symbol names, and file paths. It handles exact identifier lookups. searching for `parse_config` finds that exact function. BM25 alone scores sub-millisecond latency (0.067ms p50).

**Vector search (semantic matching)** embeds the query and compares it against pre-computed chunk embeddings stored in sqlite-vec. This catches conceptual matches. searching "authentication middleware" finds relevant auth code even if those exact words don't appear. Vector search alone achieves 0.66 Recall@10 but only 0.28 MRR@10 (high recall, poor ranking).

Neither path alone is sufficient. BM25 misses semantic matches. Vector search misses exact identifiers and ranks poorly. Combining them covers both.

## Fusion: Reciprocal Rank Fusion

Results from both retrieval paths are merged using Reciprocal Rank Fusion (RRF). RRF scores each result based on its rank in each list:

```
score(d) = 1/(k + rank_bm25(d)) + 1/(k + rank_vector(d))
```

A result that ranks high in both lists gets a high fused score. A result that ranks high in only one list still appears, but lower. The constant `k` (default: 60) controls how much weight goes to top-ranked vs. lower-ranked results.

RRF is simple, parameter-light, and doesn't need training data. It consistently outperforms either retrieval path alone.

## Reranking: Cross-Encoder

The top 30 fused candidates are sent to a cross-encoder reranker. Unlike embeddings (which encode query and document separately), the cross-encoder reads the query and each candidate together as a single pair, scoring relevance jointly.

This is the most expensive stage but also the most impactful. Reranking lifts MRR@10 from 0.39 to 0.60. a 54% improvement in how often the best result appears at the top.

With local models, the reranker runs on-device via ONNX Runtime. With API mode, it calls your configured endpoint.

## Storage

Everything lives in two places:

- **`.vera/`** in the project root. SQLite database with chunk metadata, Tantivy BM25 index, and sqlite-vec vector store. One directory per project.
- **`~/.vera/models/`**: cached ONNX models (only in local mode). Downloaded once by `vera setup`.

The index is a single SQLite database file plus a Tantivy directory. No external services, no daemons, no background processes.

## Incremental Updates

`vera update .` detects changed files by comparing content hashes against the stored index. Only modified files are re-parsed, re-chunked, and re-embedded. For small changes this takes seconds, not minutes.

## Pipeline Summary

```
Query
  ├─→ BM25 search (Tantivy)        ──→ ranked candidates
  └─→ Vector search (sqlite-vec)    ──→ ranked candidates
                                          │
                                    RRF fusion
                                          │
                                    top 30 candidates
                                          │
                                    cross-encoder rerank
                                          │
                                    final ranked results
```

| Stage | What it does | Why it matters |
|-------|-------------|----------------|
| Tree-sitter parsing | Extracts symbols as chunks | Results map to real code boundaries |
| BM25 | Exact keyword matching | Catches identifiers, fast |
| Vector search | Semantic similarity | Catches conceptual matches |
| RRF fusion | Merges both result lists | Covers both exact and semantic |
| Cross-encoder rerank | Joint query-document scoring | Best result lands at the top |
