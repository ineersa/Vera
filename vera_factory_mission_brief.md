# Vera Exploratory Mission Brief for Factory Missions

Use this as the mission charter for Droid Mission Mode.

Also load `vera_agent_guardrails.md` as the standing engineering ruleset.
Use `vera-planning.md` and any earlier Vera notes only as reference material.
They are not source of truth, not locked decisions, and not requirements.
They are a hypothesis bank, idea archive, and comparison input.

## Mission intent

Design, evaluate, prototype, and validate the strongest evidence-backed successor to pampax for codebase indexing and retrieval for coding agents.

The goal is not to preserve prior direction.
The goal is to discover the best direction.
If earlier ideas survive benchmarking and implementation pressure, keep them because they win.
If they do not, replace them.

## What is fixed

Only these mission-level goals are fixed:

1. The result should materially improve on pampax and other realistic indexing/retrieval approaches for coding-agent workflows.
2. The result must be justified with evidence, not preference alone.
3. The resulting codebase should stay clean, compact, maintainable, and easy for future AI agents to understand, debug, and extend.
4. The mission should produce both a practical implementation path and a clear written record of what was tried, what won, what lost, and why.

## What is intentionally *not* fixed

Do **not** assume any of the following are already decided unless the mission itself produces evidence for them:

- implementation language or runtime,
- storage/index backend,
- retrieval unit (file, symbol, chunk, graph element, or hybrid),
- lexical/vector/hybrid fusion strategy,
- graph-lite vs graph-heavy semantics,
- CLI vs MCP vs daemon vs mixed interface strategy,
- local-only vs remote-assisted model strategy,
- model providers, embeddings, rerankers, or fallback stacks,
- language tiering strategy,
- chunking strategy,
- metadata schema,
- incremental indexing design,
- repo layout,
- skill layout,
- any preferred architecture from prior notes.

Prior notes may still contain useful ideas.
They just are not commitments.

## How to treat prior Vera notes

Treat `vera-planning.md` and any earlier briefs as:

- candidate requirements,
- candidate architectures,
- candidate ranking ideas,
- candidate language-support plans,
- candidate interface ideas,
- candidate model/back-end choices,
- clues about the original intent behind Vera,
- prompts for experiments.

Do **not** treat them as mandatory.
The mission should explicitly separate:

- user intent that remains clearly valuable,
- assumptions that still need proof,
- ideas that should be discarded.

## Mission operating mode

This is a research + architecture + implementation + benchmarking mission.
Droid should behave like a strong technical lead running an evidence-driven exploration program, not like a coder preserving an inherited spec.

### Expected approach

1. **Plan first.**
   - Do not jump straight into coding.
   - First build a mission plan with features, milestones, validators, unknowns, and required skills.

2. **Start from workflows and success criteria.**
   - Identify the core coding-agent jobs Vera must do better than pampax and plausible alternatives.
   - Translate those jobs into benchmarkable tasks.

3. **Map the decision space.**
   - Identify the serious candidate approaches that could plausibly win.
   - Prune weak options quickly.
   - Go deep only on finalists.

4. **Use evidence to collapse uncertainty.**
   - Prefer small prototypes, benchmark spikes, ablations, and failure analysis over large speculative builds.

5. **Keep design and implementation tightly coupled to evaluation.**
   - Evals are not a final stage.
   - They are part of architecture selection.

6. **Keep the codebase agent-friendly from day one.**
   - Do not postpone maintainability until after experimentation.
   - The implementation path itself should stay clear and compact.

7. **Clean up after decisions.**
   - Temporary experiment branches are fine.
   - Dead experimental paths should not accumulate in the main architecture.

## Required mission outputs

Produce these artifact types during the mission. Exact filenames may differ.

1. A mission plan with features, milestones, validators, and explicit success criteria.
2. A benchmark/evaluation harness that is reproducible.
3. Short decision memos or ADRs for major architecture choices.
4. One or more working candidate prototypes where comparison is necessary.
5. A chosen implementation direction with evidence.
6. A benchmark report comparing Vera against pampax and other serious baselines.
7. A maintainability audit focused on long-term AI/human maintainability.
8. A concise repo-root `AGENTS.md` once concrete commands and conventions exist.
9. A final recommendation memo covering chosen path, rejected paths, open risks, and next steps.

## Required exploration dimensions

The mission should examine whichever of these materially affect outcomes.
It does not need to exhaustively test every trivial variation, but it should test every serious contender that could realistically win.

### Product and workflow questions

- What exact user and coding-agent workflows matter most?
- What counts as “better than pampax” in those workflows?
- What output form is most useful to an agent: compact capsules, snippets, symbol cards, ranked files, richer graph context, or something else?

### Architecture questions

- What runtime or language gives the best mix of quality, implementation speed, packaging, and maintainability?
- Should Vera be a single binary, a thin CLI over a local service, a library + CLI, or another shape?
- What storage/index backend is best for the actual workload?

### Retrieval questions

- What should the primary ranking unit be?
- How much structure is actually useful?
- When do symbol-aware or AST-aware methods beat simpler chunking?
- What lexical, semantic, metadata, and relationship signals meaningfully help?
- Is graph information useful enough to justify its complexity?

### Update and scaling questions

- What incremental indexing behavior is needed in practice?
- How should monorepos, workspaces, ignored files, generated files, and partial rebuilds work?
- What schema and migration strategy best supports iteration without turning into a maintenance trap?

### Interface questions

- Which interface mix best serves coding agents?
- Should CLI, MCP, or both exist in v1?
- What JSON or machine-readable contract is easiest for agents to consume and least likely to drift?

### Model/provider questions

- Is a model-assisted path necessary for the best results?
- Which embedding and reranking choices matter enough to justify complexity?
- Where is the best boundary between local operation, optional remote quality boosts, and provider abstraction?

## Evaluation requirements

Design evaluation early, before architecture hardens.

### Required evaluation dimensions

Measure at least these families where relevant:

- retrieval quality,
- usefulness for coding-agent tasks,
- indexing speed,
- update speed,
- query latency,
- storage footprint,
- robustness across languages and repo shapes,
- output compactness / token efficiency,
- implementation complexity,
- maintainability and debuggability.

### Required metric families

Use metrics appropriate to the chosen workflow set, such as:

- Recall@k,
- MRR,
- nDCG or similar ranking-quality metrics,
- p50/p95 latency,
- index build time,
- incremental update time,
- output size in chars/tokens,
- failure rate or crash rate,
- benchmark reproducibility/stability.

### Required workload categories

Include realistic coding-agent tasks such as:

1. exact symbol or definition lookup,
2. natural-language intent search,
3. cross-file implementation discovery,
4. config/build/infra lookup,
5. mixed code + docs retrieval,
6. hard disambiguation cases,
7. update-after-edit scenarios,
8. monorepo or workspace scenarios,
9. fallback-language or low-structure scenarios.

### Required baselines

Compare against the strongest reproducible baselines available, including:

- something similar to pampax (pampax is probably too buggy, maybe choose something more popular so we can include in our readme.md actual results to show our tool is better),
- a strong lexical baseline,
- a simple structure-aware baseline,
- a vector-only baseline,
- at least one additional realistic local or hybrid alternative when feasible.

If a baseline cannot be run, document why and substitute the strongest reproducible alternative.

### Required ablations

Run ablations on the highest-impact decisions where practical, for example:

- ranking unit choice,
- chunking choice,
- lexical-only vs semantic-only vs hybrid,
- reranker on/off,
- metadata-rich vs metadata-light retrieval,
- graph signals on/off if explored,
- compact outputs vs richer outputs.

## Suggested planning shape

This is a starting point, not a locked structure.
Droid may revise it if a better plan emerges.

### Milestone 1 - Planning, workflows, and benchmark harness

- Extract mission goals from the user request and prior reference notes.
- Identify candidate workflows and define measurable success criteria.
- Inventory serious architecture options worth testing.
- Build the initial benchmark and reporting harness.
- Propose the mission plan with features, milestones, validators, and needed skills.

Validation:

- A written plan exists.
- Serious candidate options are identified.
- Benchmark scaffolding exists.
- Prior assumptions are explicitly categorized as hypotheses, not constraints.

### Milestone 2 - Candidate architecture spikes

- Build small spikes for the major decision axes.
- Explore storage/backend options.
- Explore retrieval-unit/chunking options.
- Explore interface/output contract options if necessary.
- Eliminate weak directions quickly.

Validation:

- Decision space is materially reduced.
- Evidence exists for why losers lost.
- At least one promising path is clear enough for deeper implementation.

### Milestone 3 - Finalist prototype(s) and comparative evals

- Implement the strongest one or two finalist paths deeply enough for fair comparison.
- Add realistic agent-oriented query flows.
- Measure end-to-end quality, latency, update behavior, and output usefulness.

Validation:

- Finalists can be run reproducibly.
- Comparative results exist.
- The winning direction is justified with evidence.

### Milestone 4 - Hardening the winning path

- Clean up the chosen architecture.
- Remove dead experiments from the main path.
- Tighten schemas, contracts, logging, errors, tests, and validation commands.
- Add or scope the agent-facing interface(s).

Validation:

- Main path is stable and understandable.
- Contracts are documented and testable.
- Codebase remains compact and navigable.

### Milestone 5 - Final comparison and recommendation

- Re-run the most important benchmarks.
- Produce a final architecture recommendation.
- Summarize tradeoffs, known weaknesses, and next steps.
- Produce repo conventions and agent guidance.

Validation:

- Benchmark report is reproducible.
- Final decision memo is explicit.
- Repo conventions and maintainability guidance are in place.

## Decision-record expectations

For major decisions, produce a short memo or ADR with:

1. the question,
2. options considered,
3. evaluation method,
4. evidence,
5. decision,
6. consequences,
7. follow-up work.

## AI-friendly implementation expectations

The resulting codebase should be easy for future AI agents to work on.
That means:

- obvious ownership boundaries,
- small modules,
- explicit contracts,
- deterministic validation commands,
- low hidden state,
- minimal dead code,
- descriptive naming,
- shallow directory layout,
- stable machine-readable outputs,
- easy-to-trace failures.

Treat maintainability as a first-class evaluation dimension, not a cleanup pass.

## Guidance on using Factory Missions well

Because Missions work best when the plan is solid before execution, the mission should spend real effort on planning, feature decomposition, milestone structure, and validation design before heavy implementation starts. Milestones should be meaningful checkpoints because they determine validation frequency, and the plan should be concrete enough that Mission Control can manage execution without ambiguity.

Missions can also leverage existing skills, AGENTS.md, hooks, custom droids, and MCP integrations during execution, so the mission should deliberately create or refine project conventions and skills when they would improve the work.

## Stop-and-report conditions

Pause and produce an explicit decision report if any of these happen:

- the benchmark harness cannot produce trustworthy comparisons,
- the search space keeps expanding without converging,
- a candidate architecture looks impressive but is becoming hard to maintain,
- the repo starts accumulating oversized modules, duplicate pathways, or ambiguous ownership,
- a supposedly promising path underperforms simpler baselines,
- the mission needs to reframe what “better than pampax” actually means.

## Final success test

The final result should not just be “a tool that works.”
It should be the strongest evidence-backed foundation for a next-generation codebase indexing and retrieval tool for coding agents, with a codebase that future AI agents can safely extend without drowning in complexity.
