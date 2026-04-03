---
name: vera
description: Practical repository discovery skill for finding where logic lives and how code flows.
---

# Vera

Use this skill for codebase discovery only.

## When To Use

Load this skill when the user asks questions like:

- where logic is implemented
- how behavior flows across files
- who calls a function, or what a function calls
- where an exact token/import/pattern appears
- how database/auth/config/cache logic is organized

Do not load this skill for build/test/deploy/refactor work unless discovery is part of the task.

## Operating Rules

- Run Vera from repository root (the directory that owns `.vera/`).
- If the agent is inside a subdirectory, run commands with repo root as the working directory.
- Do not run probe noise (`which vera`, `command -v vera`, `vera --version`, `pwd`, `ls`) unless the user explicitly asks for install/debug diagnostics.
- Start with one focused query, then narrow with flags instead of running many broad commands.
- Use `--path` with `vera search` for scope control; avoid changing into many subdirectories.

## Command Choice

### `vera search`

Default command for conceptual discovery.

```sh
vera search "where is authentication middleware implemented"
vera search "database connection configuration" --path "src/**"
vera search "convert API errors to user-facing errors" --lang rust --limit 5
```

### `vera grep`

Use for exact strings/regex.

```sh
vera grep "TODO|FIXME" -i
vera grep "DATABASE_URL|DB_HOST" -i
vera grep "SELECT\s+.+\s+FROM\s+users" -i
```

`vera grep` currently does not support `--path`. For path-scoped searching, use `vera search ... --path "..."`.

### `vera references`

Use for call graph navigation.

```sh
vera references createUser
vera references createUser --callees
```

### `vera overview`

Use once for unfamiliar repos or architecture-orientation questions.

```sh
vera overview
```

## Practical Workflows

### General Discovery Loop

1. Optional orientation: `vera overview` (only if needed).
2. Run one targeted `vera search` for the user question.
3. Run `vera grep` only when exact syntax/token confirmation is needed.
4. Use `vera references` for caller/callee tracing.
5. Return top matches with file paths and why each match is relevant.

### Database Exploration Playbook

Use this sequence when exploring DB logic.

1. Find DB bootstrap/config:

```sh
vera search "database client initialization"
vera search "load database connection from env"
```

2. Find models, repositories, migrations, schema:

```sh
vera search "user repository" --path "src/**"
vera search "CREATE TABLE users" --path "**/*.sql"
vera search "schema migration prisma typeorm sequelize knex diesel sqlx" --path "src/**"
```

3. Trace read/write paths for a specific entity:

```sh
vera search "create user" --path "src/**"
vera search "update user" --path "src/**"
vera references createUser
vera references createUser --callees
```

4. Verify transaction and safety handling:

```sh
vera search "transaction begin commit rollback" --path "src/**"
vera grep "SELECT|INSERT|UPDATE|DELETE" -i
```

### Find Specific Things Fast

- Specific symbol: `vera grep "parse_config"`
- Specific file area: `vera search "rate limit middleware" --path "src/http/**"`
- Specific behavior: `vera search "retry on database timeout" --lang go`

## Failure Handling

- If Vera says no index in current directory, rerun from repo root.
- If still missing, run `vera index .` at repo root and retry the same query.
