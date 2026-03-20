Here’s an old plan I started to put together with another AI agent, for reference only. We can change anything we want.

## Locked in

**Product shape**

* Vera is a **codebase indexing + semantic/hybrid search tool for coding agents**.
* Two interfaces:

  * **CLI** as the primary/preferred interface
  * **MCP server** as a secondary interface
* Vera should be **local-first**, storing index data locally.

**Primary design priorities**

1. Accuracy and quality
2. Performance
3. Compatibility, reliability, portability, and implementation simplicity

**Model strategy**

* **Remote/OpenAI-compatible provider mode:** optimize for **max quality**
* **Local-only mode:** optimize for **balanced quality/speed**
* Native focus is on **Qwen3 embedding + reranker support**
* Optional later local fallback:

  * heavily quantized **Jina embeddings v5 nano**
  * **Jina reranker v2 base multilingual**
* Important licensing note:

  * Vera can be **MIT**
  * but the Jina local model path should be treated as **experimental / non-commercial-sensitive**, not the main story

**Backend**

* Current preferred local backend: **LanceDB**
* No issue with using it unless implementation language/runtime support becomes a serious blocker

**Implementation direction**

* Current leaning: **Rust core**
* Reasoning:

  * strong fit for local CLI tooling
  * good fit with Tree-sitter
  * good packaging/distribution story
* Concern noted:

  * slower iteration / compile-testing loops for agent-driven development
* This is still leaning, but not 100% irreversibly locked

**Retrieval philosophy**

* Vera is now clearly **retrieval/reranking-first**
* Not graph-first
* Not dependent on deep semantic graphs
* Core value should come from:

  * strong chunking
  * strong metadata
  * strong hybrid retrieval
  * strong reranking
  * compact agent-friendly outputs

**Graph stance**

* We are **not** making deep graphs a core pillar
* Graph-heavy reasoning is probably too risky for:

  * wide language support
  * correctness
  * simplicity
* If graphs exist at all, they should be **graph-lite and opportunistic**, such as:

  * file/module relationships
  * containment
  * imports/includes
  * simple adjacency
* No commitment to deep semantic call/reference graphs

**SCIP / LSIF stance**

* Not a priority
* Not part of Vera’s main product story
* May be left as a future optional advanced integration
* Vera should not depend on SCIP to be compelling

**LSP stance**

* Vera should **not require LSP**
* Vera should work well independently
* Optional enrichment can exist later, but not as a core dependency

**Fusion / ranking**

* Default recommendation: **standard RRF**
* **Weighted RRF** can be supported later as an advanced option
* Not a default unless evals prove it helps

**Chunking**

* Strong direction is:

  * **symbol-aware chunks where possible**
  * fallback to file/structural chunks where not
* **Sliding AST windows** are useful as a secondary recall tool, not the main strategy

**Output UX**

* Default should be **compact**
* Vera should return **lean context capsules**, not noisy dumps
* Full snippets / deeper expansion should be follow-up behavior

## Language support direction

You decided on a wide-support model:

### Tier 1A

Strong structure-aware retrieval for code languages:

* TypeScript
* JavaScript
* Python
* Go
* Rust
* Java
* Kotlin
* C#
* C++
* C
* PHP
* Bash/Shell
* PowerShell
* Ruby
* Swift
* Dart
* Lua
* Scala
* Zig
* Elixir

### Tier 1B

Strong structure-aware retrieval for structural/config/doc/web formats:

* HTML
* CSS
* JSON
* Markdown
* YAML
* XML
* TOML
* SQL
* HCL
* Terraform
* Dockerfile
* GraphQL
* CMake
* Proto
* SCSS
* Vue

### Tier 2A

Broader code-language expansion:

* Objective-C
* Perl
* Haskell
* Julia
* Nix
* OCaml
* Groovy
* Clojure
* Common Lisp
* Erlang
* F#
* Fortran
* MATLAB
* Nim
* D
* Fish
* Zsh
* Luau
* R
* Scheme
* Racket
* Elm
* Hack
* Hare
* V
* Vala
* WGSL
* GLSL
* HLSL
* Rego
* Prolog
* PRQL

### Tier 2B

Broader structural/config/frontend expansion:

* Svelte
* Astro
* Prisma
* Mermaid
* INI
* Nginx config
* Makefile

### Tier 0

Everything else Vera can reasonably support through Tree-sitter-backed fallback behavior

**Important support philosophy**

* Wide support does **not** mean equal depth everywhere
* Tier 1 means **best-supported structural retrieval**
* Tier 0 means fallback hybrid retrieval even when richer structure is missing

## What is strongly implied, but not fully frozen

These are close to decided, but still worth treating as open until we formalize the spec:

* **Rust** as the implementation language
* **LanceDB** as the default local index backend
* **symbol-aware retrieval** as the preferred retrieval unit where possible
* **graph-lite only**, rather than graph-heavy semantics

## What is still left to decide

These are the main open design questions now.

### 1. Exact internal index schema

We still need to define:

* what tables/collections exist
* what is stored per file
* what is stored per chunk
* what is stored per symbol
* what metadata is embedded vs stored separately
* how relationships are represented, if any

This is one of the most important remaining decisions.

### 2. Exact retrieval pipeline

We have the broad shape, but not the concrete pipeline details:

* candidate generation order
* BM25 fields
* embedding fields
* how many candidates to fetch at each stage
* rerank cutoff sizes
* when to expand context
* how compact output is assembled

### 3. Primary ranking unit

We discussed symbol-first vs chunk-first, but did not fully lock the operational design.
We still need to finalize:

* whether ranking is primarily by **symbol entity** with attached chunks
* or by **chunks with symbol metadata**
* or hybrid depending on language/file type

My current recommendation remains: **symbol-first where available, fallback to chunk-first**

### 4. Chunking spec

We still need to define exactly:

* symbol chunk boundaries
* max token/char sizes
* overlap rules
* AST window fallback rules
* special handling for very large files/symbols
* chunk context metadata format

### 5. Tree-sitter language profile design

We have tiers, but not the implementation model for profiles:

* how a language profile is represented
* what each profile must define
* how much is generic vs language-specific
* how imports/includes/sections/tags are extracted

### 6. Embedding/reranker provider abstraction

We know Vera should optimize for Qwen3, but we still need to define:

* provider config format
* OpenAI-compatible API expectations
* batching behavior
* retries/timeouts
* model capability assumptions
* local model invocation flow

### 7. Local model strategy

We have the high-level idea, but not the product decision on:

* whether local model install is in v1
* whether it is one-command official support or experimental
* which exact Jina variants to support first
* how much CPU-only performance matters before inclusion

### 8. CLI design

A major remaining area.
We still need to define:

* core commands
* command ergonomics for agents
* output modes
* JSON schemas
* error behavior
* defaults optimized for SKILL usage
* indexing/update/reindex workflow

This is extremely important because CLI is the main interface.

### 9. MCP design

We still need to decide:

* what MCP tools Vera exposes
* how much parity there is with the CLI
* whether MCP returns the same compact payload shapes
* whether MCP is intentionally thinner than CLI

### 10. SKILL.md design

This is a major remaining workstream.
We still need to determine:

* how the Vera SKILL should be structured
* how many commands/tools the agent should be taught
* how to keep context lean
* when agents should call Vera proactively
* what the ideal output contract is for agents
* how to avoid context rot/noisy retrieval

This is one of the highest-value remaining pieces.

### 11. Evaluation framework

We agreed evaluation matters, but haven’t specified:

* benchmark tasks
* offline retrieval evals
* agent-task evals
* token efficiency evals
* latency evals
* regression suite structure
* comparison baselines

This is critical if you want Vera to actually become “best in class.”

### 12. Incremental indexing / update behavior

Still open:

* file watching or not
* incremental reindex strategy
* cache invalidation
* rename handling
* partial repo updates
* branch/worktree behavior

### 13. Ignore/exclusion policy

We haven’t decided:

* default ignores
* generated files handling
* vendored dependencies
* binaries/assets
* lockfiles/minified files
* size thresholds
* opt-in overrides

### 14. Multi-repo / workspace strategy

Still undecided:

* single-repo only at first?
* monorepo-specific handling?
* multi-root workspace support?
* shared global cache?

### 15. Storage/versioning/migration strategy

Still open:

* schema versioning
* reindex triggers on schema/model changes
* embedding version compatibility
* migration vs rebuild behavior

## My view of the next highest-leverage decisions

These are the ones I’d tackle next, in order:

1. **CLI contract**
2. **index schema**
3. **retrieval pipeline**
4. **chunking specification**
5. **SKILL.md strategy**
6. **evaluation plan**

That sequence will force the rest of the architecture to become concrete quickly.
