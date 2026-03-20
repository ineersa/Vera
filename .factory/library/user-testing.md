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

(To be populated during Milestone 1 eval harness setup)
