# Vera Reference Hypotheses

This file reframes ideas from `vera-planning.md` as **non-binding hypotheses**.
It exists to preserve useful prior thinking without forcing Droid to treat those ideas as decisions.

Nothing here is fixed.
Everything here may be confirmed, modified, combined, or rejected.

## Product and workflow hypotheses

These are plausible directions worth evaluating:

- Vera may be a codebase indexing and semantic / hybrid search tool for coding agents.
- CLI may be the primary interface.
- MCP may be a secondary interface.
- local-first indexing and storage may be a strong default.

## Priority hypotheses

Earlier thinking suggested these priorities, which should be tested against real mission outcomes:

1. retrieval quality and answer usefulness,
2. performance,
3. compatibility, reliability, portability, and implementation simplicity.

## Retrieval philosophy hypotheses

These ideas surfaced repeatedly and are worth evaluating:

- retrieval / reranking-first design may be better than graph-first design,
- strong chunking and metadata may matter more than deep semantic graphs,
- graph-lite relationships may be useful while graph-heavy semantics may be too costly or fragile,
- Vera may not need LSP to be compelling,
- Vera may not need SCIP / LSIF as a core dependency,
- compact context capsules may work better for agents than verbose snippet dumps.

## Ranking and chunking hypotheses

Candidate directions from prior discussion:

- symbol-aware chunks may beat purely file-based chunks,
- sliding AST windows may be useful as fallback or recall support rather than the main strategy,
- standard RRF may be a strong simple fusion default,
- weighted RRF may or may not justify its added complexity,
- symbol-first ranking may beat chunk-first ranking in some cases, or a hybrid may win overall.

## Implementation hypotheses

Candidate implementation directions worth testing:

- Rust may be a strong fit for a local CLI-oriented tool,
- LanceDB may be a useful local backend,
- a mixed approach could still win if it measurably improves iteration speed or capability,
- simple architecture may beat richer architecture if the extra complexity does not deliver real gains.

## Model and provider hypotheses

Candidate provider and model directions from prior thinking:

- remote / OpenAI-compatible provider mode may maximize quality,
- local-only mode may need a different optimization target focused on balanced quality and speed,
- Qwen-oriented embeddings and reranking may be a strong primary path,
- Jina-based local fallback paths may be worth testing as experimental alternatives,
- provider abstraction should probably be explicit and testable.

## Language support hypotheses

Earlier planning leaned toward wide language support with different depth tiers.
That suggests these candidate ideas:

- wide support may be strategically important,
- equal depth across languages may not be necessary,
- richer structure-aware support may matter most for a smaller top tier,
- fallback support for long-tail languages may still need to be genuinely useful.

## Open-question inventory from earlier notes

These remain useful prompts for exploration:

1. exact internal index schema,
2. exact retrieval pipeline,
3. primary ranking unit,
4. chunking specification,
5. Tree-sitter language profile design,
6. embedding and reranker provider abstraction,
7. local model strategy,
8. CLI design,
9. MCP design,
10. skill / agent integration design,
11. evaluation framework,
12. incremental indexing and update behavior,
13. ignore and exclusion policy,
14. multi-repo / workspace strategy,
15. storage versioning and migration policy.

## Suggested early investigation order

The earlier notes implied that these might be especially high-leverage early decisions:

1. interface contract,
2. index schema,
3. retrieval pipeline,
4. chunking strategy,
5. agent integration model,
6. evaluation plan.

Use that ordering only if it still makes sense after planning.
