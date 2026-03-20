# Indexing Performance Benchmark Results

## Environment

- **Machine:** AMD Ryzen 5 7600X3D (12 threads), 30GB RAM
- **OS:** Arch Linux (kernel 6.19.9)
- **Rust:** 1.94.0 (release build)
- **Embedding API:** Qwen3-Embedding-8B via Nebius tokenfactory API
- **Date:** 2026-03-20

## Configuration

| Parameter               | Value |
|-------------------------|-------|
| Batch size              | 64    |
| Max concurrent requests | 8     |
| Stored vector dimension | 1024  |
| API timeout             | 60s   |
| Max retries (transient) | 3     |
| Max retries (rate limit)| 7     |

## Target Repo: ripgrep (BurntSushi/ripgrep)

| Metric            | Value     |
|-------------------|-----------|
| **Commit**        | 4519153   |
| **Total LOC**     | 175,424   |
| **Rust LOC**      | 52,266    |
| **Files parsed**  | 209       |
| **Chunks created**| 5,377     |
| **Binary skipped**| 9         |
| **Wall time**     | **59.2s** |
| **Source size**    | 23.4 MB   |
| **Index size**    | 32.4 MB   |
| **Size ratio**    | **1.38x** |

### Index Breakdown

| Component    | Size    |
|-------------|---------|
| BM25 index  | 2.7 MB  |
| Metadata DB | 5.0 MB  |
| Vector DB   | 24.7 MB |

## Performance Optimizations Applied

1. **Parallel parsing (rayon):** File reading, tree-sitter parsing, and
   chunking run concurrently across all CPU cores.
2. **Concurrent embedding API calls:** Up to 8 batches sent in parallel
   using `futures::future::join_all`.
3. **Batched storage writes:** Metadata, vectors, and BM25 index all use
   batch insert operations within transactions.
4. **Vector dimension truncation:** Qwen3-Embedding-8B produces 4096-dim
   vectors; truncated to 1024-dim for storage (Matryoshka-style). This
   reduces vector storage by 4× with minimal quality impact.
5. **Rate limit resilience:** Automatic retry with extended backoffs for
   429 and transient 400 errors from the embedding API.

## Targets vs Actuals

| Target                       | Actual     | Status |
|------------------------------|------------|--------|
| 100K+ LOC in < 60s          | 175K LOC in 59.2s | ✅ PASS |
| Index size < 2× source      | 1.38× source      | ✅ PASS |
| Concurrent processing used   | rayon + tokio      | ✅ PASS |

## Notes

- Wall-clock time is dominated by embedding API calls (~95% of total time).
  Parsing + chunking takes < 2s with rayon parallelization.
- Timing depends on API response latency and rate limits. The API endpoint
  (Nebius tokenfactory) has per-model token quotas that can cause transient
  400/429 errors under high concurrency.
- The 1024-dim truncation is configured via `max_stored_dim` in
  `EmbeddingConfig`. Set to 0 for full 4096-dim storage (increases index
  size ~4× but may improve retrieval quality for edge cases).
