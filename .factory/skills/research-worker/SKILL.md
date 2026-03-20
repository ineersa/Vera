---
name: research-worker
description: |
  Handles Milestone 1 research tasks: architecture spikes, competitor benchmarking,
  evaluation harness building, embedding/storage experiments, and ADR writing.
  Use when the feature involves research, experimentation, prototyping, or producing
  an architecture decision record. Not for production Rust implementation or final benchmarks.
---

# Research Worker

Execute research spikes, build evaluation infrastructure, benchmark competitors,
and produce evidence-backed Architecture Decision Records (ADRs) for the Vera project.

## When to Use

- The task involves running an architecture spike (e.g., Rust vs TypeScript, LanceDB vs SQLite)
- The task involves benchmarking a competitor tool (grepai, cocoindex-code, Zoekt, ripgrep)
- The task involves building or extending the evaluation harness
- The task involves comparing embedding models or reranker configurations
- The task requires producing an ADR with experimental evidence
- The task is tagged for Milestone 1 or is explicitly research/spike work

Do **not** use this skill for production Rust implementation (use `implementation-worker`)
or final publishable benchmarks (use `benchmark-worker`).

## Work Procedure

### Step 1: Understand the Research Question

1. Read the feature description carefully. Identify the core question to answer.
2. Read `vera_reference_hypotheses.md` to see if there are prior assumptions about this topic.
3. Read any existing ADRs in `docs/adr/` that relate to this decision area.
4. Clarify success criteria: what evidence would make a clear decision possible?

### Step 2: Set Up the Experiment Environment

1. Create a working directory under `spikes/<spike-name>/` for throwaway prototype code.
2. If the experiment needs test repositories, clone them into `.bench/repos/` (this directory is gitignored).
3. If competitor tools are needed, install them locally and pin their versions.
4. If API credentials are needed (embedding models, rerankers):
   - Run `source secrets.env` before any API calls.
   - **Never** log, print, or commit API keys. Verify your code does not echo credentials.
   - **Never** commit `secrets.env`.
5. Document exact tool versions, commit SHAs of test repos, and environment details.

### Step 3: Write Test/Evaluation Code First

1. If building evaluation harness code, treat it as **production-quality infrastructure**:
   - Write it in the project's main source tree (not in `spikes/`).
   - Include proper error handling, typed data structures, and tests.
   - Follow the project's size budgets (files < 300 lines soft, < 500 hard).
2. If writing spike/prototype code:
   - Keep it in `spikes/<spike-name>/`.
   - It can be rougher, but must be **reproducible** (clear setup instructions, pinned deps).
3. Define the metrics you will collect before running anything:
   - Recall@k, MRR, nDCG for retrieval quality
   - p50/p95 latency for performance
   - Index time, storage size for resource usage
   - Token count for output efficiency

### Step 4: Run Experiments and Collect Data

1. Run each experiment variant at least once with identical inputs.
2. Record raw results in machine-readable format (JSON preferred).
3. Save results in the spike directory: `spikes/<spike-name>/results/`.
4. If results are surprising or inconsistent, re-run and investigate before concluding.
5. For API-based experiments, note rate limits and costs if relevant.

### Step 5: Analyze and Compare

1. Build a comparison table with all variants side by side.
2. Highlight the winner on each metric.
3. Note any tradeoffs (e.g., faster but less accurate, smaller but harder to maintain).
4. Check if results match or contradict prior hypotheses from `vera_reference_hypotheses.md`.

### Step 6: Write the ADR

Create the ADR in `docs/adr/NNN-<topic>.md` using this exact 7-section format:

```markdown
# ADR-NNN: <Decision Title>

## Question
What specific question are we deciding?

## Options
List each candidate option with a brief description.

## Evaluation Method
How did we test? What metrics, what corpus, what configuration?

## Evidence
Present the data: tables, measurements, benchmark results.
Include raw numbers, not just "Option A was better."

## Decision
State the chosen option clearly.

## Consequences
What are the implications? What do we gain, what do we lose or defer?

## Follow-up
What should be validated later? Any open risks?
```

- Number ADRs sequentially (check existing ADRs for the next number).
- Keep the ADR concise but include enough data to be self-contained.
- A future agent should be able to read just the ADR and understand the decision.

### Step 7: Clean Up and Commit

1. Commit evaluation harness code (production-quality) with a meaningful message.
2. Commit the ADR separately with a message like `docs: ADR-NNN <topic>`.
3. Spike code stays in `spikes/` -- commit it for reference but it will be cleaned up later.
4. Remove any temporary files, downloaded artifacts, or large binaries from the commit.
5. Verify `secrets.env` is NOT staged: run `git diff --cached --name-only | grep secrets` and confirm empty.

## Example Handoff

```json
{
  "feature": "Architecture spike: SQLite+sqlite-vec vs LanceDB storage backend",
  "status": "complete",
  "work_done": [
    "Read prior hypotheses and existing landscape analysis",
    "Created spike in spikes/storage-backend-comparison/",
    "Implemented identical index/query benchmark against both backends",
    "Used 3 test repos: fastify (TS, 45k LOC), flask (Python, 28k LOC), ripgrep (Rust, 52k LOC)",
    "Measured: index time, query latency (p50/p95), storage size, Recall@10",
    "SQLite+sqlite-vec: 12s index, 8ms/15ms query, 45MB storage, 0.82 Recall@10",
    "LanceDB: 9s index, 6ms/11ms query, 52MB storage, 0.84 Recall@10",
    "Wrote ADR-003 recommending LanceDB based on query performance and simpler embedding integration",
    "Committed eval harness improvements and ADR"
  ],
  "files_created": [
    "spikes/storage-backend-comparison/README.md",
    "spikes/storage-backend-comparison/bench.rs",
    "spikes/storage-backend-comparison/results/sqlite_results.json",
    "spikes/storage-backend-comparison/results/lancedb_results.json",
    "docs/adr/003-storage-backend.md"
  ],
  "commits": [
    "a1b2c3d feat(eval): add storage backend benchmark harness",
    "d4e5f6a docs: ADR-003 storage backend decision (LanceDB)"
  ],
  "key_finding": "LanceDB wins on query latency and developer ergonomics; SQLite wins on storage size. LanceDB chosen for simpler vector integration and better p95 tail latency.",
  "open_questions": [
    "LanceDB incremental update performance not yet tested (deferred to M3)"
  ]
}
```

## When to Return to Orchestrator

Return when:

- The research question has a clear, evidence-backed answer and an ADR is written
- The evaluation harness code is committed and tests pass
- Spike code is committed in `spikes/` with reproducibility notes
- All experimental data is saved in machine-readable format
- `secrets.env` is confirmed not committed

Escalate back early if:

- The experiment reveals the question itself is wrong or needs reframing
- Results are ambiguous and you need guidance on which tradeoff to prioritize
- You discover a blocker (missing tool, API down, insufficient test data)
- The scope of the spike is significantly larger than expected
