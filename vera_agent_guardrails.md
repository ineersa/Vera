# Vera Agent Guardrails (Exploratory Mission Version)

Use this as the standing engineering ruleset during exploration and implementation.
These are engineering and maintainability guardrails, not architecture constraints.
They do **not** force a specific runtime, backend, or retrieval design.

## Core principle

Optimize the codebase for long-term correctness, clean evolution, low drift, easy debugging, and easy maintenance by both humans and AI agents.
Do not optimize only for initial implementation speed.

## What future AI agents usually struggle with

Design against these failure modes:

- giant files and giant functions,
- vague module ownership,
- duplicate logic in multiple places,
- hidden mutable state,
- silent fallbacks and implicit conventions,
- many layers of thin wrappers,
- dead experimental code left in the main path,
- unstable output schemas,
- poor error context,
- docs and commands drifting out of sync with the code.

## Scope discipline

- Change only what is needed for the current milestone.
- Keep experiments isolated and reviewable.
- Do not mix large refactors with behavior changes unless necessary.
- When both must happen together, explain why.
- Prefer a small number of understandable moving parts over many clever ones.

## Architectural discipline

- Prefer explicit layers and ownership boundaries.
- Keep discovery, parsing, chunking, indexing, retrieval, reranking, output assembly, config, interfaces, and evals separable.
- Prefer composition over inheritance.
- Prefer straightforward control flow over clever indirection.
- Keep side effects near boundaries.
- Minimize hidden shared mutable state.
- Avoid magic-heavy patterns unless clearly justified.
- Avoid framework sprawl when a direct design would work.

## Repo-shape discipline

- Keep the top-level repo layout shallow and easy to scan.
- Make it obvious where a new feature or bug fix belongs.
- Prefer one obvious home for each responsibility.
- Avoid giant `utils` or `helpers` dumping grounds.
- Avoid creating many micro-packages or micro-crates without a durable reason.

## Size budgets

- Soft target: keep most files under 300 lines.
- Hard review trigger: explain files over 500 lines.
- Soft target: keep most functions under 40 lines.
- Hard review trigger: explain functions over 80 lines.
- Split code by responsibility, not by arbitrary abstraction patterns.

## Interface and schema rules

- Public interfaces must be narrow and well named.
- Public data contracts should be typed where practical.
- Machine-readable outputs should be stable and easy for agents to parse.
- Version schemas and payloads that may evolve.
- Prefer explicit required fields over vague optional state.
- Avoid multiple competing ways to request the same behavior.

## Config rules

- Prefer explicit config over hidden conventions.
- Validate config early.
- Fail loudly on invalid config.
- Avoid silent fallback chains that hide mistakes.
- Keep provider assumptions documented close to the config model.

## Error handling and observability

- Return actionable errors with context.
- Preserve original cause chains where possible.
- Make failures easy to debug locally.
- Add diagnostics for indexing, schema, provider, and retrieval failures.
- Use structured logging where practical.
- Keep debug output informative without becoming noise.
- Make validation failures easy to inspect later.

## Testing rules

- Every major subsystem needs focused tests.
- Keep a fast smoke path for local iteration.
- Add end-to-end tests for index -> query -> result flows.
- Add regression tests for bugs once fixed.
- Add contract or golden tests for stable payloads.
- Do not mark work complete without proof.

## Benchmark and evaluation rules

- Treat the evaluation harness as production infrastructure, not a side script.
- Keep benchmark inputs, outputs, and commands versioned.
- Prefer reproducible benchmark runs.
- Record both wins and losses.
- Keep ablations so future agents know why a choice was made.
- Do not make irreversible architecture claims without comparative evidence.

## Documentation and decision hygiene

- Record major architecture decisions in short ADRs or decision memos.
- Update docs when commands, contracts, schemas, or workflows change.
- Keep docs close to code.
- Avoid duplicating the same truth in many places.
- When a doc becomes stale, fix it or delete it.
- Keep a clear summary of what was tried, what won, and why.

## Cleanup rules

- Delete dead experimental branches once a decision is made.
- Do not leave half-abandoned prototype paths in the main architecture.
- Remove unused helpers and stale abstractions.
- Keep the repo shallow and easy to navigate.
- Do not let temporary compatibility shims become permanent by neglect.

## AI-friendly coding rules

- Prefer descriptive names.
- Prefer explicit inputs and outputs.
- Avoid deep call chains.
- Avoid giant utility modules.
- Avoid hidden invariants; document them where they matter.
- Make ownership obvious at the file and module level.
- Make validation commands easy to discover.
- Make it easy to answer: what does this module own, what are its inputs, what are its outputs, and how is it verified?

## Simplicity rules

- When two designs perform similarly, choose the simpler one.
- When an abstraction saves little but hides a lot, remove it.
- When the codebase is getting harder to explain than the problem itself, simplify.
- The cleanest adequate solution beats the most impressive-looking one.

## Delivery rules

- Benchmark before making strong architecture claims.
- If an experiment fails, keep the learning and drop the code.
- If complexity is growing faster than capability, simplify.
- If ownership is getting blurry, restructure before continuing.
- If a direction keeps needing exceptions, wrappers, and special cases, reconsider it.

## Minimal completion checklist for any milestone

Before calling a milestone complete, verify:

1. the goal is clear,
2. the implementation is understandable,
3. validation exists and was run,
4. logs/errors are usable,
5. docs/decision notes were updated,
6. dead code from the milestone was removed or quarantined,
7. the change did not make the repo harder for a future AI agent to navigate.
