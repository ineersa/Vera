# Agent Usage

Vera usage in agents should stay focused on three commands:

- `vera overview` for high-level repository orientation
- `vera search` for conceptual/behavioral code discovery
- `vera grep` for exact string or regex matching

## Quick Command Choice

- Use `vera overview` first in unfamiliar repos.
- Use `vera search` for "where/how" questions.
- Use `vera grep` for exact identifiers, imports, TODOs, and strict patterns.

## Minimal Examples

```bash
vera overview
vera search "where is auth token refresh handled" --limit 5
vera grep "TODO|FIXME" -i --context 1
```

For full skill behavior and load criteria, see `skills/vera/SKILL.md`.
