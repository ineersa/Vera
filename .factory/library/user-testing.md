# User Testing

Testing surface, resource cost classification, and validation approach.

**What belongs here:** How to test Vera's user-facing surfaces, concurrency limits, testing infrastructure.

---

## Validation Surface

**Primary surface:** CLI (terminal commands)
- `vera index <path>` - index a repository
- `vera search <query>` - search indexed code
- `vera update <path>` - incremental re-index
- `vera stats` - project statistics
- `vera config` - configuration management
- All commands support `--json` output mode

**Secondary surface:** MCP server (Milestone 3+)
- Exposed via `vera mcp` command
- Tools: search_code, index_project, update_project, get_stats

**Testing tools:** Execute tool for CLI invocation, output parsing, exit code checking.
No browser testing needed.

## Validation Concurrency

**Surface: CLI**
- Each validator runs CLI commands (~50-200MB RAM per invocation, ~500MB peak during indexing)
- Dev server: none needed (CLI tool)
- Infrastructure shared: test repo clones in `.bench/repos/`
- Max concurrent validators: **5** (19GB available / 0.5GB peak per validator * 0.7 headroom = ~26, capped at 5)
- Isolation: each validator can work on the same indexed repo since search is read-only. Index/update operations need file-level isolation.

**Surface: MCP**
- MCP server runs on port 3200
- Only one MCP server instance at a time
- Max concurrent validators for MCP surface: **1** (single server)

## Test Repositories

Benchmark corpus stored in `.bench/repos/` (gitignored). Repos pinned to specific commit SHAs for reproducibility.
Corpus repos: ripgrep (Rust), flask (Python), fastify (TypeScript), turborepo (Polyglot).
Setup: `bash eval/setup-corpus.sh` or verify with `cargo run --manifest-path eval/Cargo.toml --bin vera-eval -- verify-corpus`.

## Flow Validator Guidance: CLI

**Testing tool:** `Execute` tool for running CLI commands and checking output/exit codes.

**Isolation rules:**
- All eval-foundation testing is read-only (running the eval harness, inspecting files).
- No index/update operations that could cause write contention.
- Multiple validators can safely read `.bench/repos/` and `eval/tasks/` concurrently.
- The eval harness runs with mock adapters (no real tool invocations), so no external API calls needed.

**Boundaries:**
- Each validator should write its evidence files to its assigned evidence directory under `{missionDir}/evidence/eval-foundation/<group-id>/`.
- Each validator should write its flow report to `.factory/validation/eval-foundation/user-testing/flows/<group-id>.json`.

**Key commands:**
- Build eval harness: `cargo build --manifest-path eval/Cargo.toml`
- Run harness: `cargo run --manifest-path eval/Cargo.toml --bin vera-eval -- run`
- Run with JSON only: `cargo run --manifest-path eval/Cargo.toml --bin vera-eval -- run --json-only`
- Verify corpus: `cargo run --manifest-path eval/Cargo.toml --bin vera-eval -- verify-corpus`
- Stability check: `cargo run --manifest-path eval/Cargo.toml --bin vera-eval -- stability`

**ADR location:** `docs/adr/` — all ADRs follow a 7-section format.

## Flow Validator Guidance: File Verification

For architecture decision assertions, validators verify that ADR files exist at `docs/adr/` with the required 7-section format (Question, Options, Evaluation Method, Evidence, Decision, Consequences, Follow-up) and contain concrete evidence data.

## Flow Validator Guidance: Core Engine CLI

**Testing tool:** `Execute` tool for running CLI commands and checking output/exit codes.

**Vera binary:** `/home/lamim/Development/Tools/Vera/target/release/vera`

**API credentials:** Source secrets.env before running commands that need embedding/reranking:
```bash
set -a && source /home/lamim/Development/Tools/Vera/secrets.env && set +a
```

**Index locations:** Vera stores index in `.vera/` directory inside the repo being indexed. To search, `cd` into the indexed repo first, then run `vera search`.

**Pre-indexed repos:**
- Flask (Python): `/home/lamim/Development/Tools/Vera/.bench/repos/flask` — INDEXED
- Fastify (TypeScript): `/home/lamim/Development/Tools/Vera/.bench/repos/fastify` — INDEXED
- Ripgrep (Rust): `/home/lamim/Development/Tools/Vera/.bench/repos/ripgrep` — INDEXED
- Turborepo (Polyglot): `/home/lamim/Development/Tools/Vera/.bench/repos/turborepo` — NOT INDEXED

**Known issues:**
- Reranker API (SiliconFlow) may have connectivity issues. Vera should degrade gracefully (return unreranked results with warning). This is expected behavior for VAL-RET-012.
- Embedding API (SiliconFlow Qwen3) may occasionally timeout. Retry or accept BM25 fallback.

**Isolation rules for core-engine testing:**
- Search operations are read-only and safe to run concurrently.
- Index operations write to `.vera/` in the target repo — use separate repos or temp copies for index tests to avoid write contention.
- For index error handling tests (invalid path, empty dir, binary files, permission errors), use temporary directories to avoid interfering with other validators.
- Each validator writes evidence to its assigned evidence directory.

**Key commands:**
- Build: already built at `/home/lamim/Development/Tools/Vera/target/release/vera`
- Index: `cd <repo-dir> && set -a && source /home/lamim/Development/Tools/Vera/secrets.env && set +a && /home/lamim/Development/Tools/Vera/target/release/vera index .`
- Search: `cd <repo-dir> && set -a && source /home/lamim/Development/Tools/Vera/secrets.env && set +a && /home/lamim/Development/Tools/Vera/target/release/vera search "<query>" --json`
- Stats: `cd <repo-dir> && /home/lamim/Development/Tools/Vera/target/release/vera stats --json`

**Source sizes for reference (files only, excluding .git/.vera/node_modules):**
- Run `find <repo> -not -path '*/.git/*' -not -path '*/.vera/*' -not -path '*/node_modules/*' -type f | xargs du -sb | awk '{sum+=$1} END{print sum}'` to get source size.

## Flow Validator Guidance: Agent Integration

### Common Setup

**Vera binary:** `/home/lamim/Development/Tools/Vera/target/release/vera`

**API credentials:** Source secrets.env before running commands that need embedding/reranking:
```bash
set -a && source /home/lamim/Development/Tools/Vera/secrets.env && set +a
```

**Pre-indexed repos (read-only safe):**
- Flask (Python): `/home/lamim/Development/Tools/Vera/.bench/repos/flask` — INDEXED
- Fastify (TypeScript): `/home/lamim/Development/Tools/Vera/.bench/repos/fastify` — INDEXED
- Ripgrep (Rust): `/home/lamim/Development/Tools/Vera/.bench/repos/ripgrep` — INDEXED

**SKILL.md location:** `/home/lamim/Development/Tools/Vera/SKILL.md`
**Cargo.toml location:** `/home/lamim/Development/Tools/Vera/Cargo.toml`

### Isolation Boundaries

**CLI read-only groups** (CLI completeness, agent capsules): Use pre-indexed repos at `.bench/repos/`. Safe to run concurrently — search and stats are read-only.

**Incremental indexing group**: Use isolated repo copy at `/tmp/vera-test-incr/flask`. This group has exclusive write access. Do NOT use `.bench/repos/` for modification tests.

**Cross-area flows group**: Use isolated repo copy at `/tmp/vera-test-cross/flask`. This group has exclusive write access. Do NOT use `.bench/repos/` for modification tests.

**MCP group**: Use isolated repo copy at `/tmp/vera-test-mcp/flask`. MCP server uses stdio transport (not TCP), so invoke via pipe. Only one MCP instance at a time.

### MCP Testing Approach

The MCP server uses stdio JSON-RPC transport. Test by piping JSON-RPC messages:
```bash
# Initialize and send requests via pipe
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' | vera mcp
```

For multi-message testing, use a script that sends multiple JSON-RPC messages on stdin and reads responses from stdout. The server processes one request per line.

### Known Issues for Agent Integration

- MCP server uses stdio transport, NOT HTTP/TCP. No port needed.
- Reranker API may have connectivity issues — Vera degrades gracefully.
- Embedding API may occasionally timeout — accept BM25 fallback for search tests.
- `vera update` requires the target directory to already have a `.vera/` index directory.
