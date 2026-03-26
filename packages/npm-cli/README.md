# @vera-ai/cli

Code search for AI agents. Vera indexes your codebase using tree-sitter parsing and hybrid search (BM25 + vector similarity + cross-encoder reranking), then returns ranked code snippets as structured JSON.

This package downloads and wraps the native Vera binary for your platform.

## Install

```bash
npm install -g @vera-ai/cli
```

## Usage

```bash
# Index a project
vera index .

# Search
vera search "authentication middleware"

# Local ONNX inference (no API keys needed. downloads models automatically)
vera index . --onnx-jina-cpu
vera search "error handling" --onnx-jina-cpu

# GPU acceleration (NVIDIA/AMD/DirectML)
vera index . --onnx-jina-cuda
```

## What you get

- **60+ languages** via tree-sitter AST parsing
- **Hybrid search**: BM25 keyword + vector similarity, fused with Reciprocal Rank Fusion
- **Cross-encoder reranking** for precision
- **JSON output** with file paths, line ranges, code content, and relevance scores

For full documentation, see the [GitHub repo](https://github.com/lemon07r/Vera).
