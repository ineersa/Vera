---
name: benchmark-worker
description: |
  Runs reproducible benchmarks, competitor baselines, ablation studies, and produces
  formatted comparison reports for the Vera project. Handles Milestone 1 baseline
  benchmarks and Milestone 4 final benchmarks, ablations, and publishable results.
  Use when the task requires running benchmarks, collecting metrics, or producing
  benchmark reports. Not for research spikes or production implementation.
---

# Benchmark Worker

Run reproducible benchmarks, collect metrics, execute ablation studies,
and produce formatted comparison reports with machine-readable output.

## When to Use

- The task involves running Vera benchmarks against competitor tools
- The task involves collecting retrieval quality metrics (Recall@k, MRR, nDCG)
- The task involves collecting performance metrics (latency, index time, storage size)
- The task involves running ablation studies (hybrid vs pure, reranker on/off, etc.)
- The task involves producing comparison tables or benchmark reports
- The task is tagged for Milestone 1 baselines or Milestone 4 final benchmarks

Do **not** use for building the eval harness itself (use `research-worker`)
or implementing Vera features (use `implementation-worker`).

## Work Procedure

### Step 1: Understand the Benchmark Scope

1. Read the benchmark task description. Identify:
   - Which tools/configurations to compare
   - Which metrics to collect
   - Which test repositories to use
   - Whether this is a baseline run, a comparative run, or an ablation study
2. Read any existing benchmark results in `benchmarks/` to avoid redundant work.
3. Read relevant ADRs in `docs/adr/` to understand current architecture choices.

### Step 2: Set Up the Benchmark Environment

1. **Test repositories:** Ensure target repos are cloned into `.bench/repos/`.
   - Pin to specific commit SHAs. Record them.
   - Standard corpus: fastify (TypeScript), flask (Python), ripgrep (Rust),
     turborepo (polyglot) — adjust per task requirements.
   - If a repo is missing, clone it and record the exact SHA.

2. **Competitor tools:** Install and pin versions of any competitor tools needed.
   - Record exact versions: `grepai --version`, `zoekt version`, etc.
   - Use the same tool configuration that a real user would (fair comparison).
   - Document any configuration flags used.

3. **Vera binary:** Build Vera from the current commit.
   - `cargo build --release`
   - Record the git commit SHA: `git rev-parse HEAD`

4. **API credentials:** If benchmarks involve embedding/reranking APIs:
   - Run `source secrets.env` before benchmark execution.
   - **Never** log, print, or include API keys in benchmark output.
   - **Never** commit `secrets.env`.

5. **Machine state:** Note relevant system info for reproducibility:
   - CPU, RAM, disk type
   - Other significant processes running (ideally: minimize background load)
   - Record with: `uname -a`, `nproc`, `free -h`

### Step 3: Define Benchmark Tasks

1. For each test repository, define a set of benchmark queries covering:
   - **Exact symbol lookup:** "find the definition of `Router.route`"
   - **Intent search:** "how does authentication work"
   - **Cross-file discovery:** "what calls `database.connect`"
   - **Config lookup:** "where is the port configured"
   - **Disambiguation:** "which `parse` function handles JSON input"
2. For each query, define the expected relevant files/symbols (ground truth).
3. Store benchmark task definitions in `benchmarks/tasks/` as JSON:

```json
{
  "repo": "flask",
  "repo_sha": "abc1234",
  "query": "how does request routing work",
  "expected_files": ["src/flask/app.py", "src/flask/blueprints.py"],
  "expected_symbols": ["Flask.route", "Flask.add_url_rule"],
  "category": "intent_search"
}
```

### Step 4: Run Benchmarks

1. Run each tool against each query. For each run, collect:
   - **Retrieval quality:** Recall@5, Recall@10, Recall@20, MRR, nDCG@10
   - **Performance:** query latency (p50, p95 over 5+ runs), index time, storage size
   - **Output quality:** token count of returned context, relevance of top-k results
2. Run each timing measurement at least 3 times. Use median for reporting.
3. Save raw results as JSON in `benchmarks/results/<benchmark-name>/`:
   - `<tool>_<repo>_raw.json` — individual query results
   - `<tool>_<repo>_summary.json` — aggregated metrics
4. For ablation studies, vary exactly one parameter at a time:
   - Hybrid vs BM25-only vs vector-only
   - Reranker on vs off
   - Different embedding models
   - Graph-lite signals on vs off

### Step 5: Produce Comparison Tables

1. Build a comparison table with all tools/configurations as columns and metrics as rows.
2. Format as both Markdown (human-readable) and JSON (machine-readable).
3. Example Markdown table:

```markdown
| Metric          | ripgrep | Zoekt  | grepai | Vera (hybrid) | Vera (BM25-only) |
|-----------------|---------|--------|--------|---------------|-------------------|
| Recall@10       | 0.45    | 0.52   | 0.71   | 0.86          | 0.74              |
| MRR             | 0.38    | 0.44   | 0.63   | 0.79          | 0.67              |
| nDCG@10         | 0.41    | 0.48   | 0.67   | 0.82          | 0.70              |
| Query p50 (ms)  | 2       | 8      | 120    | 45            | 12                |
| Query p95 (ms)  | 5       | 15     | 350    | 95            | 28                |
| Index time (s)  | N/A     | 3.2    | 18.5   | 9.1           | 6.3               |
| Storage (MB)    | N/A     | 12     | 85     | 48            | 22                |
```

4. Highlight wins and tradeoffs. Note when a metric is not applicable.
5. Save tables in `benchmarks/reports/<benchmark-name>.md`.

### Step 6: Write the Benchmark Report

Create a summary report in `benchmarks/reports/<benchmark-name>.md` with:

1. **Objective:** What was measured and why.
2. **Setup:** Tools, versions, repos, commit SHAs, machine specs.
3. **Results:** Comparison tables with all metrics.
4. **Analysis:** What the numbers mean. Key takeaways.
5. **Limitations:** What wasn't measured, known confounders.
6. **Raw Data Reference:** Path to the JSON results files.

Keep the report concise. The data should speak — avoid inflating small differences.

### Step 7: Commit Results

1. Commit benchmark task definitions, raw results, and reports together.
2. Use a commit message like: `bench: <what was benchmarked> (<repos>)`.
   - Example: `bench: hybrid vs BM25-only ablation (flask, ripgrep, fastify)`
3. Verify no secrets are staged: `git diff --cached --name-only | grep secrets`.
4. Verify no large binary files are staged (test repo clones should be in gitignored `.bench/`).

## Example Handoff

```json
{
  "feature": "Milestone 4: Final benchmark suite — Vera vs competitors",
  "status": "complete",
  "work_done": [
    "Pinned test repos: flask@a3b1c2d, fastify@e4f5a6b, ripgrep@c7d8e9f, turborepo@1a2b3c4",
    "Installed competitors: grepai 0.4.2, zoekt 3.7.1, ripgrep 15.1.0",
    "Built Vera from commit f9e8d7c (cargo build --release)",
    "Defined 40 benchmark queries across 4 repos (10 per repo, covering all 5 categories)",
    "Ran all tools against all queries, 5 repetitions for timing",
    "Collected Recall@5/10/20, MRR, nDCG@10, query latency, index time, storage size",
    "Vera hybrid: Recall@10 = 0.86, MRR = 0.79, query p95 = 95ms",
    "Best competitor (grepai): Recall@10 = 0.71, MRR = 0.63, query p95 = 350ms",
    "Produced comparison tables in Markdown and JSON",
    "Wrote benchmark report with analysis and limitations",
    "Committed all results and report"
  ],
  "files_created": [
    "benchmarks/tasks/final-suite-queries.json",
    "benchmarks/results/final-suite/vera_hybrid_flask_raw.json",
    "benchmarks/results/final-suite/vera_hybrid_flask_summary.json",
    "benchmarks/results/final-suite/grepai_flask_raw.json",
    "benchmarks/results/final-suite/grepai_flask_summary.json",
    "benchmarks/results/final-suite/comparison_all.json",
    "benchmarks/reports/final-benchmark-suite.md"
  ],
  "commits": [
    "c4d5e6f bench: final benchmark suite — Vera vs grepai, Zoekt, ripgrep (4 repos, 40 queries)"
  ],
  "key_findings": [
    "Vera hybrid retrieval outperforms all competitors on Recall@10 (+21% vs grepai)",
    "Vera query latency is 3.7x faster than grepai at p95",
    "Vera index size is 44% smaller than grepai",
    "Reranker adds ~40ms latency but improves Recall@10 by 8 points"
  ],
  "open_questions": [
    "Turborepo polyglot results are noisier than single-language repos — may need more queries",
    "Zoekt was not tested with custom scoring — could improve its numbers slightly"
  ]
}
```

## When to Return to Orchestrator

Return when:

- All benchmark runs completed successfully with reproducible results
- Raw data is saved in machine-readable JSON format
- Comparison tables and report are written
- Results are committed with no secrets exposed
- Limitations are clearly documented

Escalate back early if:

- A competitor tool fails to install or produces errors on the test corpus
- Results are unexpectedly close and more queries or repos are needed to differentiate
- Vera crashes or produces errors during benchmark runs (implementation bug — route to `implementation-worker`)
- The benchmark scope is significantly larger than estimated
- API rate limits prevent completing embedding/reranking benchmarks
