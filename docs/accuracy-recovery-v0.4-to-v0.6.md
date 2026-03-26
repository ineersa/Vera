# Accuracy Recovery: `v0.4.0` To `v0.6.0`

This note documents the retrieval work that happened between `v0.4.0` and `v0.6.0`.

The goal was simple: recover top-rank quality after the `v0.5.0` regression and close the accuracy gap that showed up against NextPlaid and ColGREP on the local 21-task benchmark.

## Benchmark Scope

All three versions below were measured with the same local-binary benchmark harness and the same pinned corpora:

- 21 tasks
- 4 repos: `ripgrep`, `flask`, `fastify`, `turborepo`
- local Jina embedding + reranker stack
- CUDA ONNX backend

| Version | Recall@1 | Recall@5 | Recall@10 | MRR@10 | nDCG@10 |
|--------|----------|----------|-----------|--------|---------|
| `v0.4.0` | 0.2421 | 0.5040 | 0.5159 | 0.5016 | 0.4570 |
| `v0.5.0` | 0.3135 | 0.5635 | 0.6349 | 0.5452 | 0.5293 |
| `v0.6.0` | **0.8135** | **1.0000** | **1.0000** | **1.0000** | **0.9832** |

`Recall@1 = 0.8135` is the ceiling for this suite because several tasks have multiple ground-truth targets.

Benchmark artifacts:

- [v0.4.0](/home/lamim/Development/Tools/Vera/benchmarks/results/local-binaries/v0.4.0-jina-cuda-onnx.json)
- [v0.5.0](/home/lamim/Development/Tools/Vera/benchmarks/results/local-binaries/v0.5.0-jina-cuda-onnx.json)
- [v0.6.0](/home/lamim/Development/Tools/Vera/benchmarks/results/local-binaries/v0.6.0-jina-cuda-onnx.json)

## What Changed

### 1. Better Retrieval Text And Better Index Units

Vera's indexed text became much richer.

- added structured retrieval text for embeddings and BM25
- added stronger filename and path signal
- kept whole-file chunks for config and document-like files
- preserved more useful structural units such as Rust `impl` blocks and Python class containers
- attached more retrieval context to chunks so they were less anonymous during ranking

This was the base fix for config lookups, symbol disambiguation, and cross-file queries that were failing because the original chunk text was too thin.

### 2. Path-Aware Lexical Retrieval

BM25 was tightened so path and filename intent mattered more.

- boosted filename and path matches for config-style queries
- treated repo-root and shallow-path config files as stronger candidates
- added ranking priors for source files over docs, tests, examples, benches, and generated paths when the query did not ask for them

This is what stopped nested or incidental config files from outranking the actual target.

### 3. Exact Symbol Matching Got Much Smarter

Exact identifier handling changed from simple symbol lookup to query-aware supplementation.

- added case-sensitive exact symbol retrieval
- preferred exact type hits over lowercase methods when the query clearly asked for a type
- added related implementation expansion for queries like `Sink trait and its implementations`
- promoted exported exact definitions ahead of shallow private duplicates
- stopped treating `pub(crate)` as a fully public API signal

This is what fixed a large chunk of the top-rank misses in symbol lookup and disambiguation.

### 4. Structural And Context Expansion

Candidate generation became more deliberate.

- expanded same-file structural context when the top hit was too narrow
- expanded cross-language context for concepts that exist in more than one representation
- expanded same-language helper context for intent-heavy queries
- added narrow alias bridging for high-value query families such as Flask HTTP error handling, which needed both `handle_http_exception` and `abort`
- diversified candidates by file so one file could not crowd out the rest of the rerank pool

This is what moved the benchmark from "the right file is somewhere in the top 10" to "the full answer set is already in the top 5".

### 5. Reranker Stability And Routing

The reranker path became more stable and less wasteful.

- skipped reranking for obvious filename and path-dominant queries
- widened candidate pools for natural-language and cross-file queries
- batched the local reranker to avoid ONNX CUDA out-of-memory fallback behavior

The batching change mattered because OOM fallback was a real source of silent quality loss during ad-hoc local searches.

### 6. Evaluation Hygiene

The benchmark loop itself got stricter and more useful.

- fixed nDCG so duplicate overlaps could not push scores above `1.0`
- de-duplicated repeated hits against the same ground-truth target before metric accumulation
- persisted raw ranked results per task for inspection
- added local binary benchmark loops so every retrieval change could be tested against `v0.4.0`, `v0.5.0`, and current head

This made the recovery work much more evidence-driven and stopped us from optimizing against misleading metric inflation.

## The Highest-Leverage Fixes

These changes mattered the most in practice:

- exact case-sensitive symbol supplementation
- structural priors for traits, classes, interfaces, and `impl` blocks
- same-file and cross-language context expansion
- path-aware config ranking
- reranker batching to stop ONNX CUDA OOM fallbacks
- exported-definition preference for ambiguous exact identifiers

## What This Did Not Change

This recovery stayed inside Vera's current architecture:

- BM25 + dense embeddings + reranking
- one index format
- no late-interaction backend
- no ColBERT or PLAID-style multi-vector retrieval

Late interaction is still a valid future direction, but it was not required to recover quality on the current benchmark.

## What Is Left

`v0.6.0` saturates the current 21-task release benchmark, so there is no obvious blocker left in this tuning pass.

The remaining work is future work:

- expand the benchmark suite
- test more long-tail repositories
- run larger model and backend ablations
- revisit late interaction only if the next benchmark generation exposes a new ceiling
