# ADR-004: Chunking Strategy

**Status:** Accepted
**Date:** 2026-03-20

## Question

How should Vera split source code files into chunks for embedding and retrieval? The chunking strategy directly affects retrieval quality (whether the right code fragment ranks high), token efficiency (how compact the retrieved context is), and indexing speed. The strategy must handle multiple programming languages and degrade gracefully for unsupported file types.

## Options Considered

### Option A: Sliding-Window (line-based)

- **Method:** Fixed-size windows of 50 lines with 10-line overlap
- **Pros:** Simple, language-agnostic, no parsing needed, captures all file content
- **Cons:** Splits mid-function/mid-class, creates overlapping redundant chunks, no semantic boundaries

### Option B: File-Level

- **Method:** Entire file as one chunk, split at 150 lines for large files
- **Pros:** Fewest chunks (most compact index), preserves file-level context, simplest to implement
- **Cons:** Large chunks dilute relevance signal; a 150-line chunk containing a 10-line target function carries 140 lines of noise

### Option C: Symbol-Aware (AST-based)

- **Method:** Tree-sitter parses the file's AST; each top-level symbol (function, class, struct, impl, trait, enum, interface) becomes one chunk. Large symbols (>150 lines) split into sub-chunks. Inter-symbol gaps (imports, module-level code) captured separately. Falls back to sliding-window for unsupported languages.
- **Pros:** Chunks align with semantic boundaries; each chunk is a complete, meaningful code unit; symbols rank correctly for lookup queries
- **Cons:** Requires tree-sitter grammars per language; parsing adds CPU overhead; variable chunk sizes

## Evaluation Method

Compared all three strategies on Vera's 21-task benchmark suite using the same embedding model (Qwen3-Embedding-8B) and the same retrieval method (cosine similarity, top-20 results). This isolates the chunking variable: everything else is held constant.

**Methodology:**
- ~300 source files per repo, 4 repos (ripgrep, flask, fastify, turborepo)
- Metrics: Recall@1, Recall@5, Recall@10, MRR, nDCG@10
- Token efficiency: total chunks, total tokens (~4 chars/token), token ratio vs sliding-window baseline
- Symbol-aware chunking used regex-based heuristic symbol detection (approximating tree-sitter AST extraction) with language-specific patterns for Rust, Python, JavaScript, TypeScript, Go

**Environment:** AMD Ryzen 5 7600X3D, 30GB RAM, Arch Linux. Spike code in `spikes/embedding-chunking/`.

## Evidence

### Overall Aggregate Metrics (21 tasks, 4 repos, Qwen3-Embedding-8B)

| Metric | Sliding-Window | File-Level | Symbol-Aware |
|--------|:---:|:---:|:---:|
| **Recall@1** | 0.0952 | 0.0952 | **0.2381** |
| **Recall@5** | 0.4921 | 0.4921 | **0.5873** |
| **Recall@10** | **0.6627** | 0.6468 | 0.6111 |
| **MRR** | 0.2814 | 0.2646 | **0.3792** |
| **nDCG@10** | **0.7077** | 0.5196 | 0.6955 |

### Token Efficiency

| Metric | Sliding-Window | File-Level | Symbol-Aware |
|--------|:---:|:---:|:---:|
| Total chunks | 5,028 | 1,779 | 5,801 |
| Total tokens | 1,706,571 | 1,365,644 | 1,463,180 |
| **Token ratio** (vs sliding-window) | 1.00 | **0.80** | **0.86** |
| Index time (s) | 442.4 | 168.7 | 428.1 |

### Per-Category Breakdown

#### Symbol Lookup (6 tasks)

| Metric | Sliding-Window | File-Level | Symbol-Aware |
|--------|:---:|:---:|:---:|
| Recall@5 | 0.6667 | 0.5000 | 0.6667 |
| Recall@10 | **0.8333** | **0.8333** | 0.6667 |
| **MRR** | 0.2431 | 0.1759 | **0.5504** |
| nDCG@10 | 0.5047 | 0.3272 | **0.5718** |

**Key finding:** Symbol-aware MRR (0.55) is **2.3× higher** than sliding-window MRR (0.24). This means the correct function/struct definition ranks at position ~2 on average with symbol-aware chunks, vs position ~4 with sliding-window. For exact symbol lookup — the most common agent query type — this is a dramatic improvement.

#### Intent Search (5 tasks)

| Metric | Sliding-Window | File-Level | Symbol-Aware |
|--------|:---:|:---:|:---:|
| **Recall@5** | 0.5000 | 0.8000 | **0.9000** |
| Recall@10 | 0.9000 | 0.9000 | 0.9000 |
| **MRR** | 0.5533 | 0.5833 | **0.6400** |

Symbol-aware achieves R@5=0.90 (vs 0.50 for sliding-window) on intent queries. The focused, semantically coherent chunks make the embedding space cleaner for conceptual matching.

#### Config Lookup (4 tasks)

| Metric | Sliding-Window | File-Level | Symbol-Aware |
|--------|:---:|:---:|:---:|
| Recall@5 | 0.7500 | 0.7500 | 0.7500 |
| Recall@10 | 0.7500 | 0.7500 | 0.7500 |
| MRR | 0.1958 | **0.2500** | 0.1958 |

All strategies perform similarly on config files (TOML, JSON, YAML), which don't have symbol-level structure. File-level slightly wins on MRR for config files.

#### Cross-File Discovery (3 tasks)

| Metric | Sliding-Window | File-Level | Symbol-Aware |
|--------|:---:|:---:|:---:|
| Recall@10 | **0.3889** | 0.2778 | 0.2778 |
| MRR | **0.2333** | 0.1389 | 0.1778 |

Cross-file remains the hardest category. Sliding-window marginally leads, possibly because overlapping chunks provide more coverage across file boundaries.

#### Disambiguation (3 tasks)

| Metric | Sliding-Window | File-Level | Symbol-Aware |
|--------|:---:|:---:|:---:|
| Recall@10 | 0.0833 | 0.0833 | **0.1667** |
| MRR | **0.0673** | 0.0556 | 0.0476 |

All strategies struggle with disambiguation. Symbol-aware finds slightly more results but doesn't rank them well. This category needs BM25 lexical matching, not better chunking.

### Quality-Efficiency Tradeoff

| Strategy | MRR (quality) | Token ratio (efficiency) | Quality per token |
|----------|:---:|:---:|:---:|
| Sliding-Window | 0.2814 | 1.00 | 0.28 |
| File-Level | 0.2646 | 0.80 | 0.33 |
| **Symbol-Aware** | **0.3792** | **0.86** | **0.44** |

Symbol-aware delivers **35% higher MRR** while using **14% fewer tokens** than sliding-window, yielding the best quality-per-token ratio (0.44 vs 0.28).

## Decision

**Symbol-aware (AST-based) chunking** is the primary chunking strategy for Vera, with **sliding-window as the Tier 0 fallback** for unsupported file types.

Rationale:
1. **2.3× higher symbol lookup MRR** (0.55 vs 0.24) — the most important metric for agent workflows
2. **Best Recall@5** (0.59) and **best overall MRR** (0.38) across all categories
3. **14% more token-efficient** than sliding-window while having higher quality
4. **Best intent search precision** (R@5=0.90 vs 0.50) — critical for natural language code queries
5. **Semantic coherence:** each chunk is a complete code symbol, not a random line-range cut
6. **Graceful degradation:** falls back to sliding-window for files without AST grammar support

### Strategy Tiers

| Tier | Strategy | When Used |
|------|----------|-----------|
| **Tier 1** | Symbol-aware (tree-sitter AST) | Supported languages (Rust, Python, TS, JS, Go, Java, C/C++) |
| **Tier 0** | Sliding-window (50 lines, 10 overlap) | Unsupported languages, config files, text files |

## Consequences

**Gains:**
- Dramatically better ranking for symbol lookup queries (the most common agent query type)
- Cleaner embedding space: each chunk represents a complete semantic unit
- 14% token savings over sliding-window baseline
- Metadata-rich chunks: each has symbol_type, symbol_name, file_path, line range
- Tree-sitter is already chosen for Vera's parsing layer (complements indexing strategy)

**Trade-offs accepted:**
- Requires tree-sitter grammar per language (mitigated by Tier 0 fallback)
- Slightly lower Recall@10 than sliding-window (0.61 vs 0.66) — compensated by hybrid BM25+vector retrieval
- Cross-file discovery doesn't improve with symbol-aware chunking (0.28 vs 0.39) — needs graph-lite metadata
- Variable chunk sizes may create slight embedding quality variance for very short symbols

**Mitigations:**
- BM25 hybrid search compensates for the minor Recall@10 gap (BM25 excels at exact matches)
- Minimum chunk size threshold (e.g., 3 lines) filters trivial symbols
- Large symbol splitting (>150 lines) prevents embedding quality degradation
- Gap chunks (imports, module-level code) ensure no content is lost between symbols
- File-level fallback for config/TOML/JSON/YAML files where symbol extraction doesn't apply

## Follow-up

1. Implement full tree-sitter AST chunking in Rust (replacing regex heuristics from this spike)
2. Define Tier 1 language profiles with specific node types per language
3. Benchmark with production tree-sitter parsing vs regex heuristic to validate quality holds
4. Investigate graph-lite metadata (call sites, imports, type references) to improve cross-file discovery
5. Consider adding filename/path-matching stage for config lookup queries
6. Test minimum symbol size thresholds (2, 3, 5 lines) for optimal chunk granularity
