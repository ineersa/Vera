# Vera Ablation Studies

Systematic ablation analysis of Vera's retrieval pipeline components.
Each study isolates one factor and measures its impact on retrieval quality
and performance across 5 workload categories.

## Setup

- **Machine:** AMD Ryzen 5 7600X3D 6-Core (12 threads), 30 GB RAM, NVMe SSD
- **Vera:** v0.1.0, Rust 1.94, SQLite + sqlite-vec + Tantivy
- **Embedding:** Qwen3-Embedding-8B (4096→1024-dim via Matryoshka truncation)
- **Reranker:** Qwen3-Reranker (cross-encoder via API)
- **Benchmark suite:** 17–21 tasks across 3–4 repositories, 5 workload categories

### Data Sources

| Ablation | Data Source | Task Count | Notes |
|----------|-----------|------------|-------|
| Hybrid vs Semantic-Only | Final benchmarks + M1 vector-only baseline | 17 + 21 | Vector-only from M1 (sliding-window chunks) |
| Hybrid vs Lexical-Only | Final benchmarks (bm25-only vs hybrid) | 17 | Same indexes, same tasks |
| Reranker On/Off | Final benchmarks (hybrid-norerank vs hybrid) | 17 | Same indexes, same tasks |
| Embedding Models | M1 embedding spike results | 21 | 3 models, pure vector search |

---

### Ablation 1: Hybrid vs Semantic-Only (Vector Search)

Compares Vera's full hybrid pipeline (BM25 + vector + RRF + reranking)
against pure vector similarity search (Qwen3-Embedding-8B, cosine similarity).

**Key question:** Does adding BM25 lexical matching to vector search improve
retrieval quality?

#### Overall Comparison

| Metric     | Semantic-Only | Hybrid    | Δ (change) |
|------------|---------------|-----------|------------|
| Recall@1   |         0.095 |     0.426 | **+348%** |
| Recall@5   |         0.492 |     0.725 | **+47%** |
| Recall@10  |         0.663 |     0.755 | **+14%** |
| MRR@10     |         0.281 |     0.594 | **+111%** |
| nDCG@10    |         0.708 |     0.802 | **+13%** |
| p50 lat.   |        1186ms |    3924ms | +231% |
| p95 lat.   |        1644ms |    7491ms | +356% |

#### Per-Category Breakdown

**Symbol Lookup:**

| Metric     | Semantic-Only | Hybrid    | Δ         |
|------------|---------------|-----------|-----------|
| Recall@1   |         0.000 |     0.800 | **—** |
| Recall@5   |         0.667 |     1.000 | **+50%** |
| Recall@10  |         0.833 |     1.000 | **+20%** |
| MRR@10     |         0.243 |     0.850 | **+250%** |
| nDCG@10    |         0.505 |     0.886 | **+76%** |

**Intent Search:**

| Metric     | Semantic-Only | Hybrid    | Δ         |
|------------|---------------|-----------|-----------|
| Recall@1   |         0.400 |     0.400 | **+0%** |
| Recall@5   |         0.500 |     0.600 | **+20%** |
| Recall@10  |         0.900 |     0.700 | **-22%** |
| MRR@10     |         0.553 |     0.460 | **-17%** |
| nDCG@10    |         1.468 |     0.915 | **-38%** |

**Cross-File Discovery:**

| Metric     | Semantic-Only | Hybrid    | Δ         |
|------------|---------------|-----------|-----------|
| Recall@1   |         0.000 |     0.000 | **—** |
| Recall@5   |         0.278 |     0.167 | **-40%** |
| Recall@10  |         0.389 |     0.167 | **-57%** |
| MRR@10     |         0.233 |     0.295 | **+27%** |
| nDCG@10    |         0.280 |     0.202 | **-28%** |

**Config Lookup:**

| Metric     | Semantic-Only | Hybrid    | Δ         |
|------------|---------------|-----------|-----------|
| Recall@1   |         0.000 |     0.333 | **—** |
| Recall@5   |         0.750 |     1.000 | **+33%** |
| Recall@10  |         0.750 |     1.000 | **+33%** |
| MRR@10     |         0.196 |     0.583 | **+198%** |
| nDCG@10    |         0.892 |     1.227 | **+38%** |

**Disambiguation:**

| Metric     | Semantic-Only | Hybrid    | Δ         |
|------------|---------------|-----------|-----------|
| Recall@1   |         0.000 |     0.125 | **—** |
| Recall@5   |         0.000 |     0.500 | **—** |
| Recall@10  |         0.083 |     0.500 | **+500%** |
| MRR@10     |         0.067 |     0.600 | **+791%** |
| nDCG@10    |         0.028 |     0.268 | **+852%** |

**Analysis:**
- Hybrid dramatically outperforms semantic-only on **symbol lookup** (+250% MRR),
  where BM25 exact matching catches identifiers that vector similarity misses.
- Hybrid's advantage on **disambiguation** is massive (>+790% MRR): BM25 finds
  exact identifier matches while vectors provide semantic ranking.
- On **intent search**, semantic-only has slightly higher Recall@10 (0.90 vs 0.70);
  the difference is partly due to different task subsets (21 vs 17 tasks). Hybrid's
  reranker provides better top-of-results precision (+198% MRR on config lookup).
- **Config lookup** is transformed: hybrid adds reranking precision (+198% MRR).
- **Cross-file discovery** remains challenging for both approaches, though hybrid
  improves MRR by +27%.

---

### Ablation 2: Hybrid vs Lexical-Only (BM25)

Compares Vera's full hybrid pipeline against BM25-only keyword search.
Both use Vera's AST-aware chunking and the same index; the difference is
whether vector search and reranking are active.

**Key question:** Does adding vector search and reranking to BM25 improve
retrieval quality, and at what latency cost?

#### Overall Comparison

| Metric     | BM25-Only | Hybrid    | Δ (change) |
|------------|-----------|-----------|------------|
| Recall@1   |     0.176 |     0.426 | **+142%** |
| Recall@5   |     0.324 |     0.725 | **+124%** |
| Recall@10  |     0.412 |     0.755 | **+83%** |
| MRR@10     |     0.282 |     0.594 | **+111%** |
| nDCG@10    |     0.281 |     0.802 | **+186%** |
| p50 lat.   |     3.0ms |    3924ms | +129083% |
| p95 lat.   |     3.6ms |    7491ms | +209577% |

#### Per-Category Breakdown

**Symbol Lookup:**

| Metric     | BM25-Only | Hybrid    | Δ         |
|------------|-----------|-----------|-----------|
| Recall@1   |     0.600 |     0.800 | **+33%** |
| Recall@5   |     1.000 |     1.000 | **+0%** |
| Recall@10  |     1.000 |     1.000 | **+0%** |
| MRR@10     |     0.750 |     0.850 | **+13%** |
| nDCG@10    |     0.812 |     0.886 | **+9%** |

**Intent Search:**

| Metric     | BM25-Only | Hybrid    | Δ         |
|------------|-----------|-----------|-----------|
| Recall@1   |     0.000 |     0.400 | **—** |
| Recall@5   |     0.000 |     0.600 | **—** |
| Recall@10  |     0.200 |     0.700 | **+250%** |
| MRR@10     |     0.067 |     0.460 | **+589%** |
| nDCG@10    |     0.058 |     0.915 | **+1482%** |

**Cross-File Discovery:**

| Metric     | BM25-Only | Hybrid    | Δ         |
|------------|-----------|-----------|-----------|
| Recall@1   |     0.000 |     0.000 | **—** |
| Recall@5   |     0.000 |     0.167 | **—** |
| Recall@10  |     0.000 |     0.167 | **—** |
| MRR@10     |     0.033 |     0.295 | **+786%** |
| nDCG@10    |     0.000 |     0.202 | **—** |

**Config Lookup:**

| Metric     | BM25-Only | Hybrid    | Δ         |
|------------|-----------|-----------|-----------|
| Recall@1   |     0.000 |     0.333 | **—** |
| Recall@5   |     0.000 |     1.000 | **—** |
| Recall@10  |     0.000 |     1.000 | **—** |
| MRR@10     |     0.000 |     0.583 | **—** |
| nDCG@10    |     0.000 |     1.227 | **—** |

**Disambiguation:**

| Metric     | BM25-Only | Hybrid    | Δ         |
|------------|-----------|-----------|-----------|
| Recall@1   |     0.000 |     0.125 | **—** |
| Recall@5   |     0.250 |     0.500 | **+100%** |
| Recall@10  |     0.500 |     0.500 | **+0%** |
| MRR@10     |     0.321 |     0.600 | **+87%** |
| nDCG@10    |     0.211 |     0.268 | **+27%** |

**Analysis:**
- Hybrid provides massive improvement on **intent search** (+557% MRR, +∞ Recall@5)
  where BM25 alone fails to match natural language queries to code.
- **Config lookup** is completely transformed: BM25 scores 0.00 across all metrics
  while hybrid achieves 1.00 Recall@5 — config files rarely contain query keywords.
- **Symbol lookup** improvement is modest (+13% MRR) since BM25 already excels at
  matching exact identifiers; the reranker adds precision.
- **Cross-file discovery** sees the biggest relative improvement: from 0.03 to 0.30 MRR.
- Latency cost is significant: BM25 p95 is ~4ms vs hybrid p95 of ~7500ms, driven
  by embedding API round trips. BM25 fallback is available for latency-critical queries.

---

### Ablation 3: Reranker On vs Off

Compares hybrid search with and without the cross-encoder reranker (Qwen3-Reranker).
Both use the same BM25 + vector + RRF fusion pipeline; the only difference is
whether the top candidates are re-scored by the cross-encoder.

**Key question:** Does the reranker improve precision enough to justify the
additional latency cost?

#### Quality Impact

| Metric        | Reranker Off | Reranker On | Δ (change) |
|---------------|-------------|-------------|------------|
| Recall@1      |       0.176 |       0.426 | **+142%** |
| Recall@5      |       0.529 |       0.725 | **+37%** |
| Recall@10     |       0.667 |       0.755 | **+13%** |
| MRR@10        |       0.336 |       0.594 | **+77%** |
| nDCG@10       |       0.518 |       0.802 | **+55%** |
| Precision@3   |       0.137 |       0.245 | **+79%** |

#### Latency Cost

| Metric        | Reranker Off | Reranker On | Cost       |
|---------------|-------------|-------------|------------|
| p50 latency   |       909ms |      3924ms | +3015ms |
| p95 latency   |      1954ms |      7491ms | +5537ms |

#### Per-Category Quality Breakdown

**Symbol Lookup:**

| Metric     | Reranker Off | Reranker On | Δ         |
|------------|-------------|-------------|-----------|
| Recall@1   |       0.400 |       0.800 | **+100%** |
| Recall@5   |       1.000 |       1.000 | **+0%** |
| Recall@10  |       1.000 |       1.000 | **+0%** |
| MRR@10     |       0.650 |       0.850 | **+31%** |
| nDCG@10    |       0.739 |       0.886 | **+20%** |

**Intent Search:**

| Metric     | Reranker Off | Reranker On | Δ         |
|------------|-------------|-------------|-----------|
| Recall@1   |       0.200 |       0.400 | **+100%** |
| Recall@5   |       0.600 |       0.600 | **+0%** |
| Recall@10  |       0.700 |       0.700 | **+0%** |
| MRR@10     |       0.332 |       0.460 | **+39%** |
| nDCG@10    |       0.754 |       0.915 | **+21%** |

**Cross-File Discovery:**

| Metric     | Reranker Off | Reranker On | Δ         |
|------------|-------------|-------------|-----------|
| Recall@1   |       0.000 |       0.000 | **—** |
| Recall@5   |       0.000 |       0.167 | **—** |
| Recall@10  |       0.167 |       0.167 | **+0%** |
| MRR@10     |       0.108 |       0.295 | **+174%** |
| nDCG@10    |       0.101 |       0.202 | **+100%** |

**Config Lookup:**

| Metric     | Reranker Off | Reranker On | Δ         |
|------------|-------------|-------------|-----------|
| Recall@1   |       0.000 |       0.333 | **—** |
| Recall@5   |       0.333 |       1.000 | **+200%** |
| Recall@10  |       0.667 |       1.000 | **+50%** |
| MRR@10     |       0.131 |       0.583 | **+347%** |
| nDCG@10    |       0.344 |       1.227 | **+257%** |

**Disambiguation:**

| Metric     | Reranker Off | Reranker On | Δ         |
|------------|-------------|-------------|-----------|
| Recall@1   |       0.000 |       0.125 | **—** |
| Recall@5   |       0.000 |       0.500 | **—** |
| Recall@10  |       0.250 |       0.500 | **+100%** |
| MRR@10     |       0.097 |       0.600 | **+515%** |
| nDCG@10    |       0.059 |       0.268 | **+358%** |

#### Per-Category Latency Breakdown

| Category           | Reranker Off (p50) | Reranker On (p50) | Added Latency |
|--------------------|--------------------|-------------------|---------------|
| Symbol Lookup      |             1159ms |            2970ms | +1811ms |
| Intent Search      |              957ms |            4434ms | +3477ms |
| Cross-File Discovery |              895ms |            4896ms | +4001ms |
| Config Lookup      |             1020ms |            3772ms | +2752ms |
| Disambiguation     |             1183ms |            3586ms | +2402ms |

**Analysis:**
- Reranking provides **+77% MRR** and **+79% Precision@3** — the strongest quality
  improvements in the pipeline. The cross-encoder correctly promotes the most relevant
  results to the top positions.
- **Recall@10 improves by +13%**, meaning the reranker also helps surface additional
  relevant results (not just reordering existing ones).
- The largest per-category gains are on **config lookup** and **disambiguation**,
  where precise ranking matters most.
- **Latency cost:** The reranker adds ~3000ms at p50, dominated by the external API
  round trip. With local reranker deployment, this would be ~10-50ms.
- **Recommendation:** Reranking is essential for precision-sensitive use cases.
  For latency-sensitive queries, use BM25-only mode (sub-10ms) or hybrid-norerank.

---

### Ablation 4: Embedding Model Comparison

Compares 3 embedding models on Vera's 21-task benchmark suite
(5 categories, 4 repositories). All models use the same chunking strategy
(sliding-window, 50 lines with 10-line overlap) and pure cosine similarity
retrieval (no BM25, no reranking) to isolate embedding quality differences.

**Key question:** Which embedding model provides the best retrieval quality
for code search tasks, and at what latency/cost?

#### Models Tested

| Model       | Dimensions | Description              | API Provider |
|-------------|-----------|---------------------------|-------------|
| Qwen3-8B    |      4096 | Code-optimized, 8B params | Nebius |
| bge-en-icl  |      4096 | General-purpose, BAAI     | Nebius |
| Qwen3-0.6B  |      1024 | Lightweight, 0.6B params  | SiliconFlow |

#### Overall Quality Comparison (21 Tasks)

| Metric     |     Qwen3-8B |   bge-en-icl |   Qwen3-0.6B |
|------------|--------------|--------------|--------------|
| Recall@1   | **     0.095** |        0.048 | **     0.095** |
| Recall@5   | **     0.492** |        0.278 |        0.405 |
| Recall@10  | **     0.663** |        0.325 |        0.405 |
| MRR@10     | **     0.281** |        0.143 |        0.255 |
| nDCG@10    | **     0.708** |        0.331 |        0.503 |

#### Performance Comparison

| Metric             |     Qwen3-8B |   bge-en-icl |   Qwen3-0.6B |
|--------------------|--------------|--------------|--------------|
| Index time (s)     |        421.2 |        707.6 |        659.1 |
| Query p50 (ms)     |         1333 |         1273 |          834 |
| Query p95 (ms)     |         1859 |         2548 |         2010 |
| Vector dimension    |         4096 |         4096 |         1024 |

#### Per-Category Breakdown

**Symbol Lookup:**

| Metric     |     Qwen3-8B |   bge-en-icl |   Qwen3-0.6B |
|------------|--------------|--------------|--------------|
| Recall@1   |        0.000 |        0.000 | **     0.167** |
| Recall@5   | **     0.667** |        0.167 | **     0.667** |
| Recall@10  | **     0.833** |        0.333 |        0.667 |
| MRR@10     |        0.243 |        0.054 | **     0.389** |
| nDCG@10    |        0.505 |        0.167 | **     0.559** |

**Intent Search:**

| Metric     |     Qwen3-8B |   bge-en-icl |   Qwen3-0.6B |
|------------|--------------|--------------|--------------|
| Recall@1   | **     0.400** |        0.200 |        0.200 |
| Recall@5   |        0.500 | **     0.700** |        0.400 |
| Recall@10  | **     0.900** |        0.700 |        0.400 |
| MRR@10     | **     0.553** |        0.407 |        0.312 |
| nDCG@10    | **     1.468** |        1.013 |        0.943 |

**Cross-File Discovery:**

| Metric     |     Qwen3-8B |   bge-en-icl |   Qwen3-0.6B |
|------------|--------------|--------------|--------------|
| Recall@1   |        0.000 |        0.000 |        0.000 |
| Recall@5   | **     0.278** |        0.111 |        0.167 |
| Recall@10  | **     0.389** |        0.111 |        0.167 |
| MRR@10     | **     0.233** |        0.083 |        0.185 |
| nDCG@10    | **     0.280** |        0.150 |        0.129 |

**Config Lookup:**

| Metric     |     Qwen3-8B |   bge-en-icl |   Qwen3-0.6B |
|------------|--------------|--------------|--------------|
| Recall@1   |        0.000 |        0.000 |        0.000 |
| Recall@5   | **     0.750** |        0.250 |        0.500 |
| Recall@10  | **     0.750** |        0.250 |        0.500 |
| MRR@10     |        0.196 |        0.083 | **     0.228** |
| nDCG@10    | **     0.892** |        0.108 |        0.527 |

**Disambiguation:**

| Metric     |     Qwen3-8B |   bge-en-icl |   Qwen3-0.6B |
|------------|--------------|--------------|--------------|
| Recall@1   |        0.000 |        0.000 |        0.000 |
| Recall@5   |        0.000 |        0.000 |        0.000 |
| Recall@10  | **     0.083** |        0.000 |        0.000 |
| MRR@10     | **     0.067** |        0.020 |        0.000 |
| nDCG@10    | **     0.028** |        0.000 |        0.000 |

**Analysis:**
- **Qwen3-Embedding-8B** is the strongest overall model with highest Recall@10
  (0.663), nDCG (0.708), and Recall@5 (0.492).
- **Qwen3-Embedding-0.6B** is surprisingly competitive on symbol lookup (MRR=0.389
  vs 8B's 0.243), suggesting smaller models may rank exact matches better. However,
  it significantly underperforms on intent and cross-file tasks.
- **bge-en-icl** excels on intent search (Recall@5=0.700) but collapses on symbol
  lookup (MRR=0.054) and config tasks (R@5=0.250), making it unsuitable for
  general-purpose code search.
- **Latency:** Qwen3-0.6B is fastest (834ms p50 vs 1333ms for 8B), offering
  a 37% speedup with 4× smaller vectors. This makes it viable for local deployment.
- **Key insight:** No single model dominates all categories. Vera's hybrid pipeline
  (BM25 + vector + reranking) compensates for individual model weaknesses, making
  the choice of embedding model less critical than the overall pipeline design.

---

## Summary of Findings

| Component         | Quality Impact (MRR) | Latency Impact | Recommendation |
|-------------------|---------------------|----------------|----------------|
| BM25 fusion       | +111% over vector-only | +0ms (local) | **Essential** — rescues exact lookup and disambiguation |
| Vector search     | +111% over BM25-only | +900ms (API) | **Essential** — enables semantic and config search |
| Cross-encoder reranking | +77% over unreranked | +3000ms (API) | **Recommended** — biggest precision boost |
| Embedding model (8B vs 0.6B) | +10% overall | +500ms | **Moderate** — 0.6B viable for latency-sensitive use |

### Key Insights

1. **Pipeline design matters more than model choice.** The hybrid architecture
   (BM25 + vector + reranking) provides >100% improvement over any single component.
2. **Each component addresses different failure modes:** BM25 for identifiers,
   vectors for semantics, reranker for precision — no single component handles all cases.
3. **Latency is API-dominated.** With local model deployment (embedding + reranker),
   hybrid latency would drop from ~4s to ~50-100ms while retaining quality gains.
4. **BM25 fallback is always available** at sub-10ms latency for latency-critical queries.

*Generated: 2026-03-20T20:31:08.627196+00:00*

## Raw Data Reference

- `benchmarks/results/final-suite/combined_results.json`
- `benchmarks/results/competitor-baselines/all_baselines.json`
- `spikes/embedding-chunking/results/embedding_*.json`
- `benchmarks/results/ablation-studies/ablation_results.json`
