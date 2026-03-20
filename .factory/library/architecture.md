# Architecture

Architectural decisions, patterns, and conventions for Vera.

**What belongs here:** Decided architecture patterns, module ownership, key design decisions.
**What does NOT belong here:** Speculative ideas (those go in ADRs until decided).

---

## Decided (updated as ADRs are finalized)

(To be populated by workers after Milestone 1 spikes)

## Key Constraints

- Files under 300 lines (soft), 500 lines (hard - must explain)
- Functions under 40 lines (soft), 80 lines (hard - must explain)
- Explicit module ownership boundaries
- Side effects at boundaries only
- Composition over inheritance
- No magic-heavy patterns without justification
