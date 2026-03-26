# Install And First Run

Vera is designed to be used through the CLI, with the Vera skill installed into the user's coding agent.

## Preferred Flow

Install Vera and the skill in one step:

```sh
npx -y @vera-ai/cli install
bunx @vera-ai/cli install
uvx vera-ai install
```

Then bootstrap Vera, index the current repository, and search:

```sh
vera setup
vera index .
vera search "authentication logic"
```

The wrapper install downloads the correct Vera binary, installs a persistent `vera` command, and runs `vera agent install`.

## Built-In Local Models

`vera setup` is the default path. It downloads the local ONNX models into `~/.vera/models/` and the ONNX Runtime library into `~/.vera/lib/`.

```sh
vera setup                        # CPU inference (default)
vera setup --onnx-jina-cuda       # NVIDIA GPU (CUDA 12+)
vera setup --onnx-jina-rocm       # AMD GPU (Linux, ROCm)
vera setup --onnx-jina-directml   # DirectX 12 GPU (Windows)
```

Notes:

- Vera keeps the repo index local in `.vera/`
- `vera setup` only chooses the built-in local model backend
- GPU flags download the matching ONNX Runtime build automatically
- `vera doctor` will tell you if ONNX Runtime or model setup is incomplete

You can configure and index in one step:

```sh
vera setup --index .
```

## OpenAI-Compatible Endpoints

Use `vera setup --api` when you already have embedding credentials or want to point Vera at a local OpenAI-compatible server.

Set these first:

```sh
export EMBEDDING_MODEL_BASE_URL=https://your-embedding-api/v1
export EMBEDDING_MODEL_ID=your-embedding-model
export EMBEDDING_MODEL_API_KEY=your-api-key
```

Optional reranker:

```sh
export RERANKER_MODEL_BASE_URL=https://your-reranker-api/v1
export RERANKER_MODEL_ID=your-reranker-model
export RERANKER_MODEL_API_KEY=your-api-key
```

Then persist them:

```sh
vera setup --api
```

If those endpoints are local, the full setup stays local. If they are remote, only the model calls leave your machine.

## Manual Skill Management

If `vera` is already on `PATH`, you can install or refresh the skill manually:

```sh
vera agent install
vera agent status --scope all
```

Useful variants:

```sh
vera agent install --client codex
vera agent install --scope project
vera agent remove --client claude
```

## Optional MCP Path

Use MCP only when the client explicitly requires it:

```sh
npx -y @vera-ai/cli mcp
bunx @vera-ai/cli mcp
uvx vera-ai mcp
```

If Vera is already installed locally:

```sh
vera mcp
```

## Diagnostics

Use these when setup fails or results look wrong:

```sh
vera doctor
vera config
vera stats
```
