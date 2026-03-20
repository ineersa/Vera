# Vera Mission Packet

This is the clean handoff version for Droid.
It is intentionally exploratory: prior Vera direction is reference material, not a fixed spec.

## Use these files

- `vera_factory_mission_brief.md`
- `vera_agent_guardrails.md`
- `vera_reference_hypotheses.md` (optional but useful)
- `vera-planning.md` as raw historical notes

## Paste-ready kickoff

```text
Read these files before doing anything else:

- vera_factory_mission_brief.md
- vera_agent_guardrails.md
- vera_reference_hypotheses.md
- vera-planning.md

Important:
- vera-planning.md is reference material only.
- Nothing in the prior Vera notes is fixed.
- Any wording like "locked", "preferred", or "current recommendation" in old notes is historical only.
- Treat prior ideas as hypotheses, prompts, and candidate directions to evaluate.

This is an exploratory research + architecture + benchmarking + implementation mission for Vera.
Your job is to determine the strongest credible successor to pampax and similar code indexing / retrieval tools for coding agents, not to preserve earlier assumptions.

First, produce:
- a concise mission plan with a small number of meaningful milestones,
- validators for each milestone,
- a clear evaluation rubric or scorecard,
- the major decision areas you think matter most,
- a shortlist of serious candidate approaches worth testing,
- the first benchmarks / prototypes / baselines you want to run.

Do not jump straight into deep implementation until the plan and evaluation approach are clear.

Throughout the mission:
- treat prior ideas as optional hypotheses,
- test only serious candidate options,
- build the benchmark harness early,
- compare against pampax and meaningful baselines,
- keep the architecture compact and AI-maintainable,
- delete abandoned paths after decisions are made,
- keep outputs concise, decision-oriented, and evidence-backed.

Your final output must include:
- the chosen architecture,
- the evidence and benchmark results behind it,
- the options you rejected and why,
- major tradeoffs,
- known gaps,
- recommended next steps.
```

## Mission framing

The planning phase matters most.
Droid should spend real effort on defining features, milestones, success criteria, and validation before heavy implementation begins.
The plan should use a small number of meaningful milestones because milestone boundaries determine validation frequency.

The mission should explicitly avoid anchoring on past preferences.
A prior idea should survive only if it performs well in experiments, benchmarks, and real coding-agent workflows.

The codebase must be built to stay maintainable for future AI agents:

- small modules,
- explicit ownership,
- stable machine-readable contracts,
- deterministic validation,
- easy debugging,
- cleanup of dead experimental paths.

## Recommended usage

If you want the simplest workflow, paste the kickoff block above into Mission Mode and keep the companion files available in the repo/session.
If you want a single reference doc while planning, use this packet plus the main brief and guardrails.
