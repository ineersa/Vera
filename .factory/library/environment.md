# Environment

Environment variables, external dependencies, and setup notes.

**What belongs here:** Required env vars, external API keys/services, dependency quirks, platform-specific notes.
**What does NOT belong here:** Service ports/commands (use `.factory/services.yaml`).

---

## API Credentials

Stored in `secrets.env` at repo root (NEVER committed, in .gitignore).

- `EMBEDDING_MODEL_BASE_URL` - Qwen3 embedding API endpoint
- `EMBEDDING_MODEL_ID` - Qwen3 embedding model identifier
- `EMBEDDING_MODEL_API_KEY` - API key for embedding service
- `RERANKER_MODEL_BASE_URL` - Qwen3 reranker API endpoint
- `RERANKER_MODEL_ID` - Qwen3 reranker model identifier
- `RERANKER_MODEL_API_KEY` - API key for reranker service

Workers must `source secrets.env` before running tests that need embedding/reranking.
NEVER log, print, or commit API keys.

## Machine Specs

- AMD Ryzen 5 7600X3D (12 threads)
- 30GB RAM (~19GB available)
- 500GB disk free
- Arch Linux

## Toolchain

- Rust 1.94 (cargo, rustc)
- Node 25.8 (for competitor benchmarking)
- Go 1.25 (for grepai/Zoekt benchmarking)
- Python 3.14 (for cocoindex-code benchmarking)
- Docker 29.3
