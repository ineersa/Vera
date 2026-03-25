# Vera

Vera is a local-first code indexing and search tool for source trees. It combines lexical search, vector search, and reranking to return ranked code results with file paths, line ranges, symbol metadata, and JSON output that is easy to consume from scripts, editors, and coding agents. This is the spiritual successor to my old tool, Pampax, which I forked from someone else's vibeslop project to fix up. I learned a lot, and realized I could make something way better if I build it from the ground up, so here we are. Every design decision and feature is very intentional, I wanted this to be the very best tool for what it does, so I did the research and testing. I don't think anything else comes close. 

## Highlights

- Fully local; connect to any OpenAI-Compatible API (local or cloud) to use any Embedding/Reranker model you want, or automatically download Jina models to run locally via ONNX. 
- Hybrid retrieval: BM25, vector similarity, Reciprocal Rank Fusion, and optional reranking
- Tree-sitter parsing across 60+ languages
- Symbol-aware chunks for functions, methods, classes, structs, and other code units
- Structured JSON output for automation and tool integration
- CLI-first agent workflow with installable Vera skill support instead of MCP-only integration
- Optional MCP server for editor and assistant workflows

## Why Vera?

I'll keep it short and simple. No silly unfounded AI drivel like "Save 80% token usage!" (although tools like this do help cut down LLM token usage). 

- Better than grep when the query is about intent, not exact text. Vera combines lexical and semantic retrieval, so queries like `"authentication logic"` or `"where request validation happens"` work without knowing the exact symbol name first. This tool is meant to supplement tools like grep, not replace them. The SKILL.md instructs the agent when and how use each tool most effectively.
- No cloud required, everything is local, with the freedom to use any model via API, or to automatically download Jina models and run them locally via ONNX.
- Built for coding agents, and devs both. Use just the CLI yourself, integrate it however you want, or install it for your AI agent with our SKILL.md file. There's even an option to use Vera as an MCP server. 
- Strong ranking quality on the public benchmark snapshot. Vera hybrid reaches `0.6009` MRR@10 and `0.7549` Recall@10 across mixed workloads, outperforming the listed non-Vera baselines in this repository's benchmark set. Probably the highest quality and accuracy from any tool like this. 
- Very fast and efficient. Incremental by default. After the initial index, `vera update .` only reprocesses changed files instead of rebuilding everything.
- Reranker support. Most tools like this don't support reranker for some reason. This was the primary reason I made the Pampax fork pre-ceding Vera. Reranking makes a big difference, even a smaller, weaker one. Especially for codebase indexing (see https://github.com/lemon07r/vecdb-bench).   

## Installation

### Agent-First Quick Start

Install the `vera` binary, then install the Vera skill for your coding agents, then configure Vera:

```bash
vera agent install
vera setup --local
vera index .
vera search "authentication logic"
```

Use `vera doctor` if local setup fails.

### Prebuilt binaries

Releases are published on [GitHub Releases](https://github.com/lemon07r/Vera/releases).

| Platform | Target | Archive |
|----------|--------|---------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| Linux aarch64 | `aarch64-unknown-linux-gnu` | `.tar.gz` |
| macOS x86_64 | `x86_64-apple-darwin` | `.tar.gz` |
| macOS aarch64 | `aarch64-apple-darwin` | `.tar.gz` |
| Windows x86_64 | `x86_64-pc-windows-msvc` | `.zip` |

```bash
curl -sL https://github.com/lemon07r/Vera/releases/latest/download/vera-x86_64-unknown-linux-gnu.tar.gz | tar xz
chmod +x vera-x86_64-unknown-linux-gnu/vera
cp vera-x86_64-unknown-linux-gnu/vera ~/.local/bin/
vera --version
```

### Build from source

Rust 1.85 or newer is required.

```bash
git clone https://github.com/lemon07r/Vera.git
cd Vera
cargo build --release
cp target/release/vera ~/.local/bin/
```

## Configuration

Vera supports two execution modes.

### API mode

Set an embedding endpoint. A reranker is optional but improves result quality, then persist that setup:

```bash
export EMBEDDING_MODEL_BASE_URL=https://your-embedding-api/v1
export EMBEDDING_MODEL_ID=your-embedding-model
export EMBEDDING_MODEL_API_KEY=your-api-key

export RERANKER_MODEL_BASE_URL=https://your-reranker-api/v1
export RERANKER_MODEL_ID=your-reranker-model
export RERANKER_MODEL_API_KEY=your-api-key

vera setup --api
```

### Local mode

Local mode is the recommended default when you want Vera to stay self-contained on your machine. Vera already stores persistent config under `~/.vera/`, but --local mode downloads local model assets to `~/.vera/models/`, keeps each repository index in that repo's own `.vera/` directory, and uses ONNX Runtime for on-device inference.

What you get in local mode:

- No hosted Vera service dependency
- Local repo index on disk in `.vera/`
- Local model cache under `~/.vera/models/`
- A good default path for private repos and offline-ish workflows once models are cached

```bash
vera setup --local
vera index .
vera search "authentication logic"
```

## Quick Start

Install the Vera skill into supported coding agent directories:

```bash
vera agent install
vera agent status --scope all
```

Index a repository:

```bash
vera index .
```

Search it:

```bash
vera search "parse_config"
vera search "authentication logic"
vera search "error handling" --lang rust
vera search "routes" --path "src/**/*.ts"
vera search "handler" --type function --limit 5 --json
```

Update after code changes:

```bash
vera update .
```

Inspect the index:

```bash
vera doctor
vera stats
vera config
```

Vera writes its index to a local `.vera/` directory in the indexed project root.

Sample JSON search result:

```json
[
  {
    "file_path": "src/auth/login.rs",
    "line_start": 42,
    "line_end": 68,
    "content": "pub fn authenticate(credentials: &Credentials) -> Result<Token> { ... }",
    "language": "rust",
    "score": 0.847,
    "symbol_name": "authenticate",
    "symbol_type": "function"
  }
]
```

## MCP

MCP is supported, but it is optional. The preferred integration path for coding agents is `vera agent install` plus direct CLI usage.

Start the MCP server with:

```bash
vera mcp
```

The server exposes:

- `search_code`
- `index_project`
- `update_project`
- `get_stats`

For CLI-focused agent guidance, see [skills/vera/SKILL.md](skills/vera/SKILL.md). For the optional MCP note, see [skills/vera/references/mcp.md](skills/vera/references/mcp.md).

## Benchmark Snapshot

The benchmark suite in this repository covers 17 tasks across three open-source codebases (`ripgrep`, `flask`, and `fastify`) and five workload categories: symbol lookup, intent search, cross-file discovery, config lookup, and disambiguation.

| Metric | ripgrep | cocoindex-code | vector-only | Vera hybrid |
|--------|---------|----------------|-------------|-------------|
| Recall@5 | 0.2817 | 0.3730 | 0.4921 | **0.6961** |
| Recall@10 | 0.3651 | 0.5040 | 0.6627 | **0.7549** |
| MRR@10 | 0.2625 | 0.3517 | 0.2814 | **0.6009** |
| nDCG@10 | 0.2929 | 0.5206 | 0.7077 | **0.8008** |

Additional performance notes from the same benchmark set:

- `vera search` in BM25-only mode measured `3.5 ms` p95 latency
- API-backed hybrid search measured `6749 ms` p95 latency and is dominated by remote model calls
- Indexing `ripgrep` (about 175K LOC) completed in `65.1 s`
- Incremental updates complete in a few seconds for small changes

More detail:

- Public benchmark summary: [docs/benchmarks.md](docs/benchmarks.md)
- Indexing performance note: [benchmarks/indexing-performance.md](benchmarks/indexing-performance.md)
- Reproduction guide: [benchmarks/reports/reproduction-guide.md](benchmarks/reports/reproduction-guide.md)

## Supported Languages

Vera supports 60+ languages and file formats, including Rust, Python, TypeScript, JavaScript, Go, Java, C, C++, SQL, Terraform, Protobuf, HTML, CSS, Vue, Dockerfile, Astro, TOML, YAML, JSON, and Markdown.
