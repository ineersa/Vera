# User Testing

Testing surface, required tools, and resource cost classification.

**What belongs here:** How to validate Vera's user-facing behavior, testing tools, concurrency limits.

---

## Validation Surface

**Primary surface:** CLI commands (shell invocations)

| Command | What it tests |
|---------|--------------|
| `vera --version` | Binary works, version correct |
| `vera --help` | Help output, command listing |
| `vera index .` | API mode indexing |
| `vera index --local .` | Local mode with model download |
| `vera search "query"` | Search in API mode |
| `vera search --local "query"` | Search in local mode |
| `vera search --lang rust "query"` | Language-filtered search |
| `vera update .` | Incremental index update |
| `vera stats` | Index statistics display |
| `vera mcp` | MCP server (JSON-RPC stdio) |

**Tools:** Shell commands via Execute tool. No browser or TUI testing needed.

## Validation Concurrency

**Machine:** 12 cores, 30GB RAM, ~14GB available
**CLI invocations:** Lightweight (~50MB each). Max concurrent validators: **5**.
**Benchmark runs:** CPU-intensive (uses all cores). Max concurrent: **1**.
**Local inference:** ~2GB RAM for models. Max concurrent with local: **3**.

## Testing Notes

- API mode requires `secrets.env` loaded (embedding + reranker endpoints)
- Local mode requires ONNX Runtime shared library on the system
- Benchmark corpus repos must be in `.bench/repos/` (run `eval/setup-corpus.sh`)
- MCP testing: pipe JSON-RPC messages via stdin, read JSON-RPC responses from stdout
