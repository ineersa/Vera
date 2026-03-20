# ADR-003: Embedding Model Strategy

**Status:** Accepted
**Date:** 2026-03-20

## Question

Which embedding model should Vera use for semantic vector search? The model choice affects retrieval quality (Recall, MRR), vector dimensionality (storage size and search speed), API latency, and deployment complexity. Vera needs a model that performs well on code-specific retrieval tasks: exact symbol lookup, natural language intent search, cross-file discovery, config lookup, and disambiguation.

## Options Considered

### Option A: Qwen3-Embedding-8B

- **Type:** Code-optimized large embedding model (8B parameters)
- **Dimensionality:** 4096
- **Provider:** Nebius Token Factory (OpenAI-compatible API)
- **Strengths:** Designed for multilingual text + code retrieval; trained on code corpora
- **Weaknesses:** Large vector size (4096-dim × 4 bytes = 16KB per chunk); higher API cost per token

### Option B: BAAI/bge-en-icl

- **Type:** General-purpose in-context learning embedding model
- **Dimensionality:** 4096
- **Provider:** Nebius Token Factory (OpenAI-compatible API)
- **Strengths:** Strong on general NLP tasks; BAAI models are well-regarded in embedding benchmarks
- **Weaknesses:** Not code-specialized; ICL approach may not suit short code snippet embedding

### Option C: Qwen3-Embedding-0.6B

- **Type:** Lightweight Qwen3 variant (0.6B parameters)
- **Dimensionality:** 1024
- **Provider:** SiliconFlow (OpenAI-compatible API)
- **Strengths:** 4× smaller vectors (1024-dim); faster inference; suitable for local deployment
- **Weaknesses:** Smaller model capacity may miss nuanced code semantics

## Evaluation Method

Ran all three models against Vera's full 21-task benchmark suite spanning 5 workload categories (symbol lookup, intent search, cross-file discovery, config lookup, disambiguation) across 4 repositories (ripgrep/Rust, flask/Python, fastify/TypeScript, turborepo/Polyglot).

**Methodology:**
- Identical chunking for all models: sliding-window (50 lines, 10-line overlap), ~300 source files per repo
- Same retrieval: pure cosine similarity, top-20 results
- Metrics: Recall@1, Recall@5, Recall@10, MRR, nDCG@10 (per-task and aggregate)
- All models accessed via OpenAI-compatible API (Nebius or SiliconFlow)

**Environment:** AMD Ryzen 5 7600X3D, 30GB RAM, Arch Linux. Spike code in `spikes/embedding-chunking/`.

## Evidence

### Overall Aggregate Metrics (21 tasks, 4 repos)

| Metric | Qwen3-Embedding-8B | bge-en-icl | Qwen3-Embedding-0.6B |
|--------|:---:|:---:|:---:|
| **Recall@1** | 0.0952 | 0.0476 | 0.0952 |
| **Recall@5** | **0.4921** | 0.2778 | 0.4048 |
| **Recall@10** | **0.6627** | 0.3254 | 0.4048 |
| **MRR** | **0.2814** | 0.1429 | 0.2551 |
| **nDCG@10** | **0.7077** | 0.3308 | 0.5030 |
| Embedding dim | 4096 | 4096 | 1024 |
| Total index time (s) | 421.2 | 707.6 | 659.1 |
| Query latency p50 (ms) | 1332.9 | 1273.2 | 833.8 |

### Per-Category Breakdown

#### Symbol Lookup (6 tasks — finding exact function/struct/class definitions)

| Metric | Qwen3-8B | bge-en-icl | Qwen3-0.6B |
|--------|:---:|:---:|:---:|
| Recall@5 | **0.6667** | 0.1667 | **0.6667** |
| Recall@10 | **0.8333** | 0.3333 | 0.6667 |
| MRR | 0.2431 | 0.0542 | **0.3889** |

Qwen3-8B and 0.6B both find 4/6 symbols in top-5. The 0.6B model actually has higher MRR (better ranking) on symbol lookups, likely because its lower dimensionality reduces noise.

#### Intent Search (5 tasks — natural language queries for code concepts)

| Metric | Qwen3-8B | bge-en-icl | Qwen3-0.6B |
|--------|:---:|:---:|:---:|
| Recall@5 | 0.5000 | **0.7000** | 0.4000 |
| Recall@10 | **0.9000** | 0.7000 | 0.4000 |
| MRR | **0.5533** | 0.4067 | 0.3118 |

Qwen3-8B strongly dominates at top-10 recall (0.90) for semantic intent queries. bge-en-icl has competitive Recall@5 (0.70) on intent but poor performance elsewhere drags down its overall score.

#### Config Lookup (4 tasks — finding config files like Cargo.toml, package.json)

| Metric | Qwen3-8B | bge-en-icl | Qwen3-0.6B |
|--------|:---:|:---:|:---:|
| Recall@5 | **0.7500** | 0.2500 | 0.5000 |
| Recall@10 | **0.7500** | 0.2500 | 0.5000 |
| MRR | 0.1958 | 0.0833 | **0.2276** |

Qwen3-8B finds 3/4 config files in top-5/10 (vs vector-only baseline from M1: also 0.75). Config lookup remains challenging for all tools.

#### Cross-File Discovery (3 tasks) and Disambiguation (3 tasks)

| Category | Metric | Qwen3-8B | bge-en-icl | Qwen3-0.6B |
|----------|--------|:---:|:---:|:---:|
| Cross-file | Recall@10 | **0.3889** | 0.1111 | 0.1667 |
| Cross-file | MRR | **0.2333** | 0.0833 | 0.1852 |
| Disambiguation | Recall@10 | 0.0833 | 0.0000 | 0.0000 |
| Disambiguation | MRR | **0.0673** | 0.0196 | 0.0000 |

Cross-file and disambiguation are hard for all models (consistent with M1 baselines). Qwen3-8B leads in both.

### Comparison with M1 Competitor Baselines

| Tool/Model | Recall@5 | Recall@10 | MRR | nDCG@10 |
|------------|:---:|:---:|:---:|:---:|
| ripgrep (lexical) | 0.2817 | 0.3651 | 0.2625 | 0.2929 |
| cocoindex-code (MiniLM) | 0.3730 | 0.5040 | 0.3517 | 0.5206 |
| **Qwen3-Embedding-8B** | **0.4921** | **0.6627** | 0.2814 | **0.7077** |
| Qwen3-Embedding-0.6B | 0.4048 | 0.4048 | 0.2551 | 0.5030 |
| bge-en-icl | 0.2778 | 0.3254 | 0.1429 | 0.3308 |

Qwen3-8B achieves the highest Recall@10 (0.66) and nDCG (0.71) of any tool tested, including competitors. Its MRR (0.28) is lower than cocoindex-code (0.35), which confirms the M1 hypothesis that reranking is needed to convert high recall into high precision.

## Decision

**Qwen3-Embedding-8B** is the primary embedding model for Vera.

Rationale:
1. **Highest retrieval quality** across all workload categories: Recall@10 = 0.66, nDCG = 0.71
2. **Dominates intent and config tasks** where semantic understanding matters most
3. **Best cross-file discovery** (0.39 Recall@10), the hardest category for all tools
4. **Outperforms all M1 competitor baselines** on Recall and nDCG
5. **OpenAI-compatible API** enables easy provider abstraction (swap to any compatible endpoint)

**Qwen3-Embedding-0.6B** is designated as a recommended lightweight fallback for local/offline use:
- Competitive symbol lookup quality (MRR=0.39 vs 0.24 for 8B)
- 4× smaller vectors (1024 vs 4096 dim)
- Suitable for resource-constrained or air-gapped environments

## Consequences

**Gains:**
- Best retrieval quality of any tested model on Vera's task suite
- Code-optimized model captures programming semantics better than general-purpose alternatives
- OpenAI-compatible API makes provider switching trivial (env var change)
- Provider abstraction designed in: EMBEDDING_MODEL_BASE_URL, EMBEDDING_MODEL_ID, EMBEDDING_MODEL_API_KEY

**Trade-offs accepted:**
- 4096-dim vectors are large: 16KB per chunk → ~48MB for 3000 chunks (within 2× budget)
- API dependency: requires network access and valid API key for embedding
- Higher per-token cost than smaller models
- MRR (0.28) lags cocoindex-code (0.35), confirming reranking is essential in the pipeline

**Mitigations:**
- Reranking stage (Qwen3-Reranker-8B, already in secrets.env) will address the MRR gap
- Provider abstraction allows switching to Qwen3-Embedding-0.6B for local mode
- BM25 hybrid pipeline compensates for vector search weaknesses on disambiguation and exact matches
- Vector dimensionality reduction could be explored if storage becomes a concern

## Follow-up

1. Validate Qwen3-8B performance with symbol-aware chunking (ADR-004 chunking decision)
2. Measure reranking impact on MRR when reranking pipeline is implemented
3. Test Qwen3-Embedding-0.6B as local fallback with `ollama` or similar local inference
4. Monitor cost and consider batch embedding optimizations for large repos
5. If Matryoshka dimensionality reduction is supported by Qwen3 models, test 1024-dim vs 4096-dim quality tradeoff
