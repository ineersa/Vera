# ADR-000: Milestone 1 Decision Summary

**Status:** Accepted
**Date:** 2026-03-20

## Purpose

This document consolidates all architecture spike results from Milestone 1 (Evaluation Foundation & Architecture Decisions) into a single decision summary. It provides:

1. All decided questions with ADR references
2. Remaining open questions (≤2)
3. Prior assumptions from `vera-planning.md` categorized as validated, invalidated, or hypothetical
4. Clear implementation direction for Milestone 2

---

## Decided Questions

### 1. Implementation Language → **Rust**

**ADR:** [ADR-001](001-implementation-language.md)

Rust was chosen over TypeScript/Bun based on spike benchmarks across four key operations:

| Metric | Rust | TypeScript/Bun | Advantage |
|--------|------|----------------|-----------|
| Tree-sitter parsing | 16.9ms (8K LOC) | 29.7ms | **1.6–1.8× faster** |
| CLI cold start | 0.51ms | 5.09ms (Bun) / 35.5ms (Node) | **10–70× faster** |
| Binary size | ~10–15MB estimated | 60–80MB compiled | **~5× smaller** |
| Ecosystem fit | ignore, Tantivy, Lance, Clap | npm ecosystem | **Superior for core deps** |

**Key rationale:** Sub-millisecond startup is critical for a CLI tool invoked frequently by agents. Single binary distribution via `cargo install` with zero runtime dependencies.

### 2. Storage Backend → **SQLite + sqlite-vec + Tantivy**

**ADR:** [ADR-002](002-storage-backend.md)

Chosen over LanceDB despite LanceDB's raw performance advantage:

| Metric | SQLite + sqlite-vec | LanceDB |
|--------|:---:|:---:|
| Vector query p50 | 9.7ms | 1.9ms |
| Write throughput | 7,596 chunks/sec | 241,440 chunks/sec |
| Dependency count | ~60 crates | 537 crates |
| Build time (full) | ~40s | ~150s |
| Async required | No | Yes |

**Key rationale:** SQLite's 10ms vector queries and 7.6K chunks/sec writes are well within Vera's performance budget (500ms p95 query, 60s index for 100K LOC). The dramatically simpler dependency tree (~60 vs 537 crates), synchronous API, and single-file database model make SQLite the pragmatic choice. Tantivy provides sub-millisecond BM25 search (0.067ms p50) — uncontested for full-text.

### 3. Embedding Model → **Qwen3-Embedding-8B**

**ADR:** [ADR-003](003-embedding-model.md)

Chosen over bge-en-icl and Qwen3-Embedding-0.6B based on Vera's 21-task benchmark suite:

| Metric | Qwen3-8B | bge-en-icl | Qwen3-0.6B |
|--------|:---:|:---:|:---:|
| Recall@5 | **0.4921** | 0.2778 | 0.4048 |
| Recall@10 | **0.6627** | 0.3254 | 0.4048 |
| MRR | **0.2814** | 0.1429 | 0.2551 |
| nDCG@10 | **0.7077** | 0.3308 | 0.5030 |

**Key rationale:** Qwen3-8B outperforms all M1 competitor baselines (ripgrep, cocoindex-code, vector-only) on Recall and nDCG. Its MRR (0.28) is lower than cocoindex-code (0.35), confirming that reranking is essential. Qwen3-0.6B (1024-dim) is designated as the lightweight fallback for local/offline use.

### 4. Chunking Strategy → **Symbol-Aware (tree-sitter AST)**

**ADR:** [ADR-004](004-chunking-strategy.md)

Chosen over sliding-window and file-level chunking:

| Metric | Symbol-Aware | Sliding-Window | File-Level |
|--------|:---:|:---:|:---:|
| Symbol Lookup MRR | **0.5504** | 0.2431 | 0.1759 |
| Overall MRR | **0.3792** | 0.2814 | 0.2646 |
| Intent R@5 | **0.9000** | 0.5000 | 0.8000 |
| Token efficiency | **0.86** | 1.00 | 0.80 |

**Key rationale:** Symbol-aware chunking produces 2.3× higher MRR on symbol lookup (the most common agent query type) while using 14% fewer tokens. Each chunk is a complete semantic code unit (function, class, struct) rather than an arbitrary line range. Sliding-window serves as the Tier 0 fallback for unsupported languages.

### 5. Retrieval Pipeline Shape → **Hybrid BM25 + Vector + RRF Fusion + Reranking**

**Evidence:** Competitor baselines (ADR-003 evidence, competitive landscape report), cross-ADR analysis.

The retrieval pipeline shape is determined by converging evidence across all spikes:

| Stage | Technology | Justification |
|-------|-----------|---------------|
| **Lexical candidates** | Tantivy BM25 | Sub-millisecond (0.067ms p50). Handles exact identifier lookup where ripgrep baseline excels (MRR=0.26). |
| **Semantic candidates** | sqlite-vec cosine similarity | Qwen3-8B vectors achieve Recall@10=0.66, best of any tool tested. |
| **Fusion** | Reciprocal Rank Fusion (RRF) | Standard approach. No competitor uses hybrid fusion — this is Vera's differentiator. |
| **Reranking** | Qwen3-Reranker (API) | MRR gap (0.28 vector-only vs 0.35 cocoindex-code) confirms reranking is essential. No competitor has reranking. |

**Key rationale:** M1 baselines show lexical search (ripgrep) is fast for identifiers but misses semantics, while vector search (Qwen3-8B) has high recall but poor precision (MRR). Hybrid fusion with reranking addresses both weaknesses — and no tool in the competitive landscape currently does all of this.

---

## Remaining Open Questions

### Open Question 1: Exact Retrieval Pipeline Parameters

While the pipeline *shape* is decided (BM25 + vector + RRF + reranking), the operational parameters are not yet tuned:

- BM25 candidate count (top-k to fetch before fusion)
- Vector candidate count (top-k to fetch before fusion)
- RRF constant `k` (standard is 60, but code search may benefit from tuning)
- Reranker cutoff size (how many candidates to rerank — affects latency vs quality)
- Context expansion rules (when to include surrounding code for a matched symbol)

**Resolution plan:** These will be tuned during M2 integration testing using the eval harness. The harness and benchmark tasks are already built.

### Open Question 2: Graph-Lite Metadata Scope

Cross-file discovery is the weakest category across all tools and strategies:

| Tool/Strategy | Cross-File Recall@10 |
|--------------|:---:|
| ripgrep (lexical) | 0.2222 |
| cocoindex-code (semantic) | **0.4444** |
| Qwen3-8B + symbol-aware | 0.2778 |
| Qwen3-8B + sliding-window | 0.3889 |

Graph-lite metadata (imports, file relationships, call adjacency) may improve cross-file discovery, but the scope and implementation cost are unclear:

- **Minimum viable:** File-to-file import edges, containment relationships
- **Extended:** Simple call site detection, type reference tracking
- **Uncertain:** Whether the quality improvement justifies the parsing complexity

**Resolution plan:** Defer to M2/M3. Implement the core pipeline first without graph metadata, then run ablation studies to measure whether graph-lite signals improve cross-file Recall@10.

---

## Prior Assumption Categorization

The following categorizes assumptions from `vera-planning.md` and `vera_reference_hypotheses.md` based on M1 experimental evidence.

### Validated ✅

| # | Assumption | Evidence | Reference |
|---|-----------|----------|-----------|
| 1 | **Rust is a strong fit for a local CLI tool** | 1.6–1.8× faster parsing, 10× faster cold start, 5× smaller binary, superior ecosystem for core dependencies. | ADR-001, spike benchmarks |
| 2 | **Symbol-aware chunks beat purely file-based chunks** | 2.3× higher MRR on symbol lookup (0.55 vs 0.24). Best overall MRR (0.38) and intent R@5 (0.90). | ADR-004, chunking spike |
| 3 | **Hybrid BM25+vector is clearly justified** | Lexical baseline (ripgrep): fast for identifiers but MRR=0.26. Vector baseline: Recall@10=0.66 but MRR=0.28. Neither alone is sufficient. | ADR-003 evidence, competitor baselines |
| 4 | **Reranking is essential** | Vector-only MRR (0.28) lags cocoindex-code (0.35) despite higher recall. High recall + low MRR = ranking problem solvable by reranking. | ADR-003, competitor baselines |
| 5 | **CLI primary interface** | Landscape analysis confirms all major AI coding tools (Claude Code, Codex) prefer CLI tools. MCP is growing but secondary. | Competitive landscape report §5.5 |
| 6 | **Local-first design** | Universal pattern in landscape. Every modern code indexer is local-first. | Competitive landscape report §3.1 |
| 7 | **Retrieval/reranking-first (not graph-first)** | Graph-heavy approaches (grepai RPG) add complexity. Core retrieval quality improvements come from chunking + embedding + reranking, not graphs. Cross-file discovery is hard for *all* tools regardless of approach. | ADR-003, ADR-004, baselines |
| 8 | **Compact context capsules over verbose dumps** | Symbol-aware chunks are 14% more token-efficient than sliding-window. Landscape gap analysis confirms no tool produces structured, compact agent outputs. | ADR-004, landscape report §6 Gap 3 |
| 9 | **Code-optimized embeddings matter** | Qwen3-8B (code-trained) outperforms bge-en-icl (general-purpose) by 2× on Recall@10 (0.66 vs 0.33) and 2× on MRR (0.28 vs 0.14). | ADR-003 |
| 10 | **Standard RRF is a strong simple fusion default** | Landscape confirms no competitor uses weighted RRF or learned fusion. Standard RRF is the consensus baseline. | Competitive landscape report §5.4 |
| 11 | **Vera should not require LSP** | No competitor requires LSP. Tree-sitter provides sufficient structural understanding for chunking and symbol extraction. | Competitive landscape report §1 |
| 12 | **SCIP/LSIF is not a priority** | No competitor depends on SCIP. Tree-sitter + embedding-based retrieval provides the core value. | Competitive landscape report §1 |
| 13 | **Wide language support with tiered depth is strategically important** | Competitors support 13–30 languages uniformly. Tiered approach (deep Tier 1, broad Tier 0 fallback) is a differentiator. | Competitive landscape report §6 Gap 5 |

### Invalidated ❌

| # | Assumption | Evidence | Reference |
|---|-----------|----------|-----------|
| 1 | **LanceDB as preferred local backend** | LanceDB is 5× faster at vector queries and 32× faster at writes, but SQLite + sqlite-vec is sufficient for Vera's scale and dramatically simpler: 60 vs 537 crates, sync API, single-file DB, 40s vs 150s build time. Performance is adequate; simplicity wins. | ADR-002, storage spike |
| 2 | **Sliding AST windows as secondary recall tool** | Sliding-window has marginally better Recall@10 (0.66 vs 0.61) than symbol-aware, but symbol-aware's MRR advantage (0.38 vs 0.28) dominates. Sliding-window is relegated to Tier 0 fallback, not a "secondary recall tool" alongside symbol-aware. | ADR-004 |

### Hypothetical (Not Yet Validated) ⚠️

| # | Assumption | Status | Resolution Plan |
|---|-----------|--------|----------------|
| 1 | **Graph-lite metadata is useful for cross-file discovery** | Cross-file discovery is weak across all strategies (max R@10=0.44). Graph-lite *may* help, but no evidence yet. | M2/M3 ablation study after core pipeline is built |
| 2 | **Weighted RRF may improve over standard RRF** | No experiment conducted. Standard RRF is the M2 baseline; weighted can be tested as an ablation. | M4 ablation study |
| 3 | **Qwen3-Embedding-0.6B is viable for local/offline use** | Competitive quality (Recall@5=0.40, MRR=0.26) with 4× smaller vectors, but not tested with local inference (Ollama). | M3 when local model path is implemented |
| 4 | **Jina embeddings/reranker as experimental fallback** | Not tested. Qwen3 models chosen as primary path; Jina remains a potential alternative for non-commercial-sensitive local use. | Deferred; revisit if local model path is prioritized |
| 5 | **Symbol-first ranking may beat chunk-first ranking** | ADR-004 tested symbol-aware *chunking* but not symbol-first *ranking* (ranking by symbol entity vs ranking by chunk). | M2 when ranking pipeline is implemented |
| 6 | **MCP as secondary interface** | Validated as a landscape pattern, but Vera's MCP implementation and parity with CLI not yet tested. | M3 MCP implementation |
| 7 | **Provider abstraction should be explicit and testable** | Designed in (env vars: EMBEDDING_MODEL_BASE_URL, etc.) but not tested with multiple providers. | M2 when embedding pipeline is integrated |

---

## Implementation Direction for Milestone 2

Based on all M1 decisions, Milestone 2 (Core Engine) should proceed with this architecture:

### Technology Stack

| Component | Technology | ADR |
|-----------|-----------|-----|
| Language | Rust 1.94 | ADR-001 |
| Metadata + vectors | SQLite + sqlite-vec (rusqlite) | ADR-002 |
| Full-text search | Tantivy | ADR-002 |
| Embedding model | Qwen3-Embedding-8B via OpenAI-compatible API | ADR-003 |
| Parsing | tree-sitter (Rust crate) | ADR-004 |
| CLI framework | clap | ADR-001 |
| Serialization | serde + serde_json | ADR-001 |

### Core Pipeline (Build Order)

1. **Tree-sitter parsing layer** — Parse source files into ASTs for Tier 1 languages (Rust, Python, TypeScript, JavaScript, Go, Java, C, C++). Tier 0 fallback for others.
2. **Symbol-aware chunker** — Extract top-level symbols (functions, classes, structs, enums, traits, interfaces) as chunks. Split large symbols (>150 lines). Capture inter-symbol gaps. Store metadata (symbol_name, symbol_type, language, file_path, line_start, line_end).
3. **Storage layer** — SQLite schema for chunk metadata + sqlite-vec virtual table for 4096-dim vectors. Tantivy index for BM25 over chunk content + metadata fields.
4. **Embedding pipeline** — Provider abstraction (EMBEDDING_MODEL_BASE_URL, MODEL_ID, API_KEY). Batch embedding requests. Error handling for API failures.
5. **BM25 indexing** — Index chunk content, symbol names, file paths, and language tags in Tantivy.
6. **Hybrid retrieval** — BM25 candidates + vector candidates → RRF fusion → ranked results.
7. **Reranking** — Qwen3-Reranker via API on top-N fused candidates.
8. **Basic CLI** — `vera index <path>`, `vera search <query>`, `vera stats`.

### Key Design Principles (from M1 Evidence)

- **Simplicity over cleverness:** SQLite chosen over LanceDB specifically for simplicity. Apply the same principle to all M2 decisions.
- **Measure before optimizing:** Use the eval harness (33 tests, 21 benchmark tasks, 4 repos) to validate every pipeline component.
- **Degrade gracefully:** If reranker API is unavailable, return unreranked hybrid results. If embedding API is down, fall back to BM25-only.
- **File size budgets:** Files under 300 lines (soft), 500 lines (hard). Functions under 40 lines (soft), 80 lines (hard).

---

## Summary

| Area | Decision | Confidence | ADR |
|------|----------|:---:|:---:|
| Implementation language | Rust | High | 001 |
| Storage backend | SQLite + sqlite-vec + Tantivy | High | 002 |
| Embedding model | Qwen3-Embedding-8B (primary), 0.6B (fallback) | High | 003 |
| Chunking strategy | Symbol-aware (tree-sitter AST) | High | 004 |
| Retrieval pipeline shape | BM25 + Vector + RRF + Reranking | High | Cross-ADR |
| **Open questions** | **Pipeline parameters, graph-lite scope** | — | — |

All 5 major architecture questions are answered with experimental evidence. At most 2 open questions remain, both deferrable to M2/M3. Vera is ready to begin production implementation.
