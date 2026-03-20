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
