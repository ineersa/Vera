# Vera Setup Guide (API mode + local llama.cpp)

This is the exact setup we are using now: Vera in `api` backend mode, with local llama.cpp servers for embeddings and reranking. `vera search --deep` can additionally use an OpenAI-compatible completion endpoint for RAG-fusion query expansion.

## 0) Prerequisites

- Embedding server running at `http://localhost:8059/v1` with model `coderankembed-q8_0.gguf`
- Reranker server running at `http://localhost:8060/v1` with model `bge-reranker-base-q8_0.gguf`
- Optional (for `vera search --deep`): completion server running at `http://localhost:8061/v1`
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
export VERA_COMPLETION_BASE_URL="http://localhost:8061/v1"
export VERA_COMPLETION_MODEL_ID="<your-completion-model>"
export VERA_COMPLETION_API_KEY="not-needed"
export VERA_COMPLETION_MAX_TOKENS="16384"
export VERA_COMPLETION_TIMEOUT_SECS="120"
export VERA_NO_UPDATE_CHECK="1"
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
vera config set indexing.max_chunk_bytes 2000
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
  "completion_api": {
    "base_url": "http://localhost:8061/v1",
    "model_id": "flash",
    "timeout_secs": 120,
    "max_tokens": 16384,
    "max_alternatives": 4
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
      "max_chunk_bytes": 2000,
      "max_chunk_tokens": 0,
      "chunk_overlap_lines": 2
    },
    "retrieval": {
      "default_limit": 10,
      "rrf_k": 60.0,
      "rerank_candidates": 50,
      "reranking_enabled": true,
      "max_rerank_batch": 20,
      "max_output_chars": 12000,
      "reranker_max_docs_per_request": 8,
      "reranker_max_document_tokens": 300
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

## 8) MCP setup (OpenCode + generic JSON snippets)

Canonical MCP command:

```bash
vera mcp
```

To force MCP to use a specific repository index, run with a fixed CWD:

```bash
bash -lc 'cd /absolute/path/to/repo && vera mcp'
```

### A) OpenCode `opencode.json`

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "vera": {
      "type": "local",
      "command": [
        "bash",
        "-lc",
        "cd /home/ineersa/projects/mate/ai && vera mcp"
      ],
      "enabled": true
    }
  }
}
```

### B) Generic MCP client JSON (`command` + `args` style)

```json
{
  "mcpServers": {
    "vera": {
      "command": "vera",
      "args": ["mcp"],
      "cwd": "/absolute/path/to/repo"
    }
  }
}
```

If you do not want to depend on `PATH`, replace `"command": "vera"` with the absolute binary path.

MCP tools exposed by Vera: `search_code`, `get_overview`, `regex_search`.

## 9) Docker (API mode, local build)

When you can't run the native binary (macOS Gatekeeper, corporate proxy blocking cargo, etc.), run Vera inside Docker.

### Build

```bash
docker compose build
# or plain docker:
docker build -t vera:local .
```

### Run MCP server (compose)

```bash
docker compose run --rm vera mcp
```

### One-off CLI commands (compose)

```bash
docker compose run --rm vera --version
docker compose run --rm vera index /workspace
docker compose run --rm vera search "authentication logic"
docker compose run --rm vera overview
```

### Run MCP server (plain docker)

```bash
docker run --rm -i \
    --add-host=host.docker.internal:host-gateway \
    -v $(pwd):/workspace \
    -v ./docker-data/vera-home:/root/.vera \
    vera:local
```

### One-time API key setup (persisted, no docker env vars needed)

Run once per `docker-data/vera-home` directory:

```bash
docker compose run --rm vera config set embedding_api.api_key not-needed
docker compose run --rm vera config set reranker_api.api_key not-needed
docker compose run --rm vera config set completion_api.api_key not-needed
```

### MCP client config (Docker)

```json
{
  "mcpServers": {
    "vera": {
      "command": "docker",
      "args": ["compose", "-f", "/absolute/path/to/docker-compose.yml", "run", "--rm", "vera"]
    }
  }
}
```

Notes:

- `host.docker.internal` routes to the host machine from inside the container so Vera can reach your llama.cpp servers
- Config and models are at `./docker-data/vera-home/` on the host, mounted to `/root/.vera` in the container — `config.json` is pre-baked with API mode and tuned settings, and keys are stored in `credentials.json`
- `docker-data/vera-home/credentials.json` is gitignored to prevent accidental credential commits
- Index data lives at `/workspace/.vera/` on the mounted project volume, so it persists too
- On macOS Docker Desktop, `host.docker.internal` works natively; on Linux it needs `--add-host` (included in compose via `extra_hosts`)

## 10) Common issues

- Docker container can't reach llama.cpp servers:
  - verify services are bound to `0.0.0.0` (not just `127.0.0.1`) so Docker can reach them via `host.docker.internal`
- `input (...) larger than max context size` during index:
  - lower `indexing.max_chunk_bytes` (or set `indexing.max_chunk_tokens`) and/or increase excludes
- Reranker context errors:
  - set `reranker.max_docs_per_request` and `reranker.max_document_chars` (or `reranker.max_document_tokens`) via `vera config set`
- `vera search --deep` behaves like normal search:
  - ensure `completion_api.base_url` and `completion_api.model_id` are set (`vera config set ...`)
- `vera search --deep` fails with `failed to generate deep-search query candidates`:
  - ensure completion model responds with JSON content (array or object containing rewrite strings) on OpenAI-compatible `/chat/completions`
  - reasoning-heavy models often need larger budgets: increase `completion_api.max_tokens` (e.g. `16384`) and `completion_api.timeout_secs`
- No `.vera` in expected repo:
  - run `vera index .` from the repo root (or pass absolute repo path)
