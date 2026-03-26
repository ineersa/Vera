# Architecture Decisions

These are the key technical choices made early on, each backed by spike implementations and benchmarked against the same 21-task evaluation suite.

| Area | Choice | Details |
|------|--------|---------|
| Language | Rust | [001](001-implementation-language.md) |
| Storage | SQLite + sqlite-vec + Tantivy | [002](002-storage-backend.md) |
| Embedding | Qwen3-Embedding-8B (API), Jina v5 nano (local) | [003](003-embedding-model.md) |
| Chunking | Symbol-aware via tree-sitter AST | [004](004-chunking-strategy.md) |
| Retrieval | BM25 + Vector + RRF + Reranking | Cross-cutting |

Spike code lives in `spikes/`.
