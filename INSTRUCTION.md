# Vera Setup Guide (API mode + local llama.cpp)

This is the exact setup we are using now: Vera in `api` backend mode, with local llama.cpp servers for embeddings and reranking.

## 0) Prerequisites

- Embedding server running at `http://localhost:8059/v1` with model `coderankembed-q8_0.gguf`
- Reranker server running at `http://localhost:8060/v1` with model `bge-reranker-base-q8_0.gguf`
- `coderankembed` query prefix requirement: `Represent this query for searching relevant code:`

Optional quick checks:

```bash
curl -s http://localhost:8059/v1/models
curl -s http://localhost:8060/v1/models
```

## 1) Get Vera on a new machine

You have two paths:

### A. Build from source (recommended for our custom fixes)

```bash
git clone https://github.com/ineersa/Vera.git
cd Vera
git checkout localhost
cargo build --release
```

Binary path:

```bash
./target/release/vera
```

Add it to `PATH` after build (current shell):

```bash
export PATH="$(pwd)/target/release:$PATH"
```

Persist it in your shell profile:

```bash
echo 'export PATH="/absolute/path/to/Vera/target/release:$PATH"' >> ~/.bashrc
# or: >> ~/.zshrc
source ~/.bashrc
```

### B. Download a prebuilt binary

If you publish binaries in GitHub Releases, download the one matching the target OS/arch.

Important: a Linux binary built here will **not** run on macOS.

- Linux build -> Linux only
- macOS build -> macOS only

For macOS, build on macOS (or produce a macOS target build in CI).

## 2) Shell env vars (`~/.bashrc` or `~/.zshrc`)

Add this block (this is the current working set):

```bash
# Vera
export PATH="/home/ineersa/mcp-servers/Vera/target/release:$PATH"
export EMBEDDING_MODEL_BASE_URL="http://localhost:8059/v1"
export EMBEDDING_MODEL_ID="coderankembed-q8_0.gguf"
export EMBEDDING_MODEL_API_KEY="not-needed"
export EMBEDDING_MODEL_QUERY_PREFIX="Represent this query for searching relevant code:"
export RERANKER_MODEL_BASE_URL="http://localhost:8060/v1"
export RERANKER_MODEL_ID="bge-reranker-base-q8_0.gguf"
export RERANKER_MODEL_API_KEY="not-needed"
export RERANKER_MAX_DOCS_PER_REQUEST="8"
export RERANKER_MAX_DOCUMENT_CHARS="1200"
```

Then reload shell config:

```bash
source ~/.bashrc
# or: source ~/.zshrc
```

## 3) Initial Vera setup in API mode

```bash
vera setup --api
```

This writes `~/.vera/config.json` with API backend settings.

## 4) Apply runtime tuning (important for small context reranker/embedding)

These are our current tuned values:

```bash
vera config set indexing.max_chunk_lines 80
vera config set indexing.default_excludes '[".git",".vera","node_modules","target","build","dist","__pycache__",".venv",".github","deptrac.yaml","UPGRADE.md","reference.php","vendor"]'
vera config set indexing.max_embedding_chars 2000
vera config set embedding.batch_size 4
vera config set embedding.max_concurrent_requests 1
```

## 5) Current `~/.vera/config.json` reference

If you want to mirror exactly, this is the current file:

```json
{
  "local_mode": false,
  "backend": "api",
  "embedding_api": {
    "base_url": "http://localhost:8059/v1",
    "model_id": "coderankembed-q8_0.gguf"
  },
  "reranker_api": {
    "base_url": "http://localhost:8060/v1",
    "model_id": "bge-reranker-base-q8_0.gguf"
  },
  "core_config": {
    "indexing": {
      "max_chunk_lines": 80,
      "default_excludes": [
        ".git",
        ".vera",
        "node_modules",
        "target",
        "build",
        "dist",
        "__pycache__",
        ".venv",
        ".github",
        "deptrac.yaml",
        "UPGRADE.md",
        "reference.php",
        "vendor"
      ],
      "max_file_size_bytes": 1000000,
      "extra_excludes": [],
      "no_ignore": false,
      "no_default_excludes": false,
      "max_embedding_chars": 2000
    },
    "retrieval": {
      "default_limit": 10,
      "rrf_k": 60.0,
      "rerank_candidates": 50,
      "reranking_enabled": true
    },
    "embedding": {
      "batch_size": 4,
      "max_concurrent_requests": 1,
      "timeout_secs": 60,
      "max_retries": 3,
      "max_stored_dim": 1024,
      "gpu_mem_limit_mb": 0,
      "low_vram": false
    }
  }
}
```

## 6) Index a code repository

Go to the repo root, then index:

```bash
cd ~/projects/my-repo
vera index .
```

This creates per-project index data in:

```text
<repo>/.vera/
```

Search example:

```bash
vera search "openai platform result converter"
```

## 7) Incremental updates and watch mode (CLI)

Manual incremental update:

```bash
vera update .
```

Start watcher (blocks foreground process):

```bash
vera watch .
```

How watch works:

- Requires existing index (`.vera/`) first
- Debounces file changes (~2s)
- Runs incremental update automatically
- Ignores changes inside `.vera/`
- Stop with `Ctrl+C`

## 8) OpenCode project MCP setup (optional)

Project-level `opencode.json` can force Vera MCP to run in that project directory:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "vera": {
      "type": "local",
      "command": [
        "bash",
        "-lc",
        "cd /home/ineersa/projects/mate/ai && /home/ineersa/mcp-servers/Vera/target/release/vera mcp"
      ],
      "enabled": true
    }
  }
}
```

This ensures MCP search tools use that repo's `.vera/` index.

## 9) Common issues

- `input (...) larger than max context size` during index:
  - lower `indexing.max_embedding_chars` and/or increase excludes
- Reranker context errors:
  - ensure `RERANKER_MAX_DOCS_PER_REQUEST` and `RERANKER_MAX_DOCUMENT_CHARS` are set
- No `.vera` in expected repo:
  - run `vera index .` from the repo root (or pass absolute repo path)
