# AGENTS.md Template (Copy-Paste)

Use this snippet in project-level `AGENTS.md` files.

```md
## Vera Skill Loading Rules

Load the `vera` skill when tasks are about repository discovery:

- where logic is implemented
- how behavior works across files
- exact token/pattern matching
- quick architecture orientation

After loading the skill, use only these commands unless the user asks otherwise:

- `vera overview` for architecture/layout orientation
- `vera search` for conceptual/behavioral discovery
- `vera grep` for exact string or regex matching

Tool choice policy:

- Prefer `vera search` for intent questions ("how/where is X handled").
- Prefer `vera grep` for exact matches (imports, TODOs, identifiers, regex).
- Use `vera overview` first when the repository is unfamiliar.
```
