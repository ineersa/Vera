---
name: vera
description: Repository discovery skill focused on `vera overview`, `vera search`, and `vera grep`.
---

# Vera

Use this skill for codebase discovery only.

## Load This Skill When

- The user asks where logic lives ("where is X implemented", "how does Y work").
- The user asks for architecture orientation in an unfamiliar repo.
- The user asks for exact text/pattern matches (imports, TODOs, regex).

Do not load this skill for build, test, deployment, or refactor tasks unless search/discovery is part of the request.

## Tool Selection

### `vera overview`

Use when you need fast orientation before searching.

Good for:
- first pass on unfamiliar repositories
- understanding language/directory layout
- finding likely areas to search next

```sh
vera overview
```

### `vera search`

Use for conceptual or behavioral code search.

Good for:
- "how is auth handled"
- "where do API errors get converted"
- "where is config loaded"

```sh
vera search "authentication middleware"
vera search "api error conversion" --limit 5
vera search "config loading" --lang php --path "src/**"
```

### `vera grep`

Use for exact string and regex matching.

Good for:
- exact identifiers
- import/include lines
- TODO/FIXME scans
- strict syntax patterns

```sh
vera grep "TODO|FIXME" -i
vera grep "use Symfony\\\\AI\\\\" --context 1
vera grep "function\s+handle\(" --path "src/**"
```

## Practical Search Loop

1. Run `vera overview` once if the repo is unfamiliar.
2. Use `vera search` for intent/behavior questions.
3. Use `vera grep` for exact tokens/patterns.
4. Narrow with `--lang`, `--path`, `--scope`, `--limit`.
5. Return top matches with file path and why each match is relevant.

## Notes

- Prefer `vera search` over regex for conceptual questions.
- Prefer `vera grep` over semantic search for exact syntax/token checks.
- If Vera reports no index, ask the user to run `vera index .` in the repo root.

## Copy-Paste Agent Template

For a project-level `AGENTS.md` snippet that tells agents when to load this skill, use:

- `references/agents.md`
