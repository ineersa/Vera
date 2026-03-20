# Vera Competitive Landscape Report
**Date:** March 20, 2026  
**Scope:** Code indexing/retrieval tools for AI coding agents

---

## 1. Tool-by-Tool Comparison

### 1.1 Comparison Table

| Feature | **cocoindex-code** | **grepai** | **SymDex** | **Zoekt** | **codemogger** | **Aider (repomap)** |
|---|---|---|---|---|---|---|
| **Language** | Python (Rust engine) | Go (94% C for tree-sitter) | Python | Go | TypeScript/Bun | Python |
| **Stars** | 1.1k | 1.5k | 121 | 1.5k | 284 | N/A (part of aider) |
| **License** | Apache 2.0 | MIT | MIT | Apache 2.0 | MIT | Apache 2.0 |
| **Storage** | Embedded SQLite + sqlite-vec | GOB files + FTS | SQLite + sqlite-vec | Custom binary shards | SQLite (Turso) + FTS + vector | In-memory graph |
| **Retrieval** | Semantic (vector) | Semantic (vector) | Semantic + symbol + text | Trigram + regexp + BM25 | Hybrid (vector + FTS) | Graph-ranked structural map |
| **AST/Tree-sitter** | ✅ AST-based chunking | ✅ (regex + tree-sitter) | ✅ tree-sitter | ❌ (ctags for symbols) | ✅ tree-sitter WASM | ✅ tree-sitter |
| **Embedding** | Local SentenceTransformers default; LiteLLM for 100+ providers | Ollama/LM Studio/OpenAI | Local sentence-transformers | N/A (lexical) | Local all-MiniLM-L6-v2 | N/A (no embeddings) |
| **Reranking** | ❌ | ❌ | ❌ | Symbol-based ranking signals | ❌ | Graph PageRank-like |
| **Call Graph** | ❌ | ✅ (callers/callees) | ✅ (symbol-level) | ❌ | ❌ | ✅ (def/ref graph) |
| **Interface** | CLI + MCP + Skill | CLI + MCP | CLI + MCP (20 tools) | CLI + Web UI + gRPC API | CLI + MCP + Library/SDK | Embedded in aider |
| **Incremental** | ✅ (changed files only) | ✅ (file watcher daemon) | ✅ (SHA-256 per file) | ✅ | ✅ (SHA-256 per file) | ✅ |
| **Languages** | ~30 | 20+ (Go, Python, JS/TS, Rust, Java, C#, C/C++, Ruby, F#, etc.) | 13 (Python, JS, TS, Go, Rust, Java, PHP, C#, C/C++, Elixir, Ruby, Vue) | Universal (any text) | 13 (Rust, C, C++, Go, Python, Zig, Java, Scala, JS, TS, TSX, PHP, Ruby) | 17+ via tree-sitter |
| **Key Claims** | 70% token savings | "reduced Claude Code tokens by X%" | 97% fewer tokens per lookup | Fast substring/regexp at scale | Keyword 25-370x faster than ripgrep | N/A |
| **Local-first** | ✅ | ✅ (100% local) | ✅ (zero infra) | ✅ (self-hosted) | ✅ | ✅ |

### 1.2 Detailed Tool Profiles

#### cocoindex-code (cocoindex-io)
- **Architecture:** Python CLI wrapping a Rust-based CocoIndex data transformation engine. Uses tree-sitter for AST-based chunking. Runs a background daemon process for index management.
- **Storage:** Embedded SQLite with sqlite-vec extension for vector similarity search. One DB per project under `.cocoindex_code/`.
- **Retrieval:** Pure semantic vector search over AST chunks. No hybrid BM25/keyword search. No reranking.
- **Strengths:** Extremely easy setup (1-min, zero config), background daemon handles indexing transparently, wide embedding provider support via LiteLLM (100+ providers), Skill-based agent integration (recommended path), active development.
- **Weaknesses:** No keyword/lexical search fallback, no reranking stage, no call graph analysis, Python runtime dependency. Default embedding model (all-MiniLM-L6-v2) is general-purpose, not code-optimized.

#### grepai
- **Architecture:** Go binary with compiled tree-sitter grammars. Uses regex-based fast extraction plus optional tree-sitter precise mode. Background daemon with file watcher.
- **Storage:** GOB (Go Binary) files for symbol stores, FTS for text. Per-project `.grepai/` directory. "RPG" (Ranked Property Graph) system for hierarchical code understanding.
- **Retrieval:** Semantic vector search over symbol embeddings. Call graph tracing (callers/callees). RPG-based enriched search results.
- **Strengths:** 100% local, privacy-first, rich call graph analysis, git worktree support, `.grepaiignore` support, strong community adoption (1.5k stars), comprehensive MCP integration, hierarchical code understanding via RPG system, F# and other less-common language support.
- **Weaknesses:** No hybrid (keyword + semantic) fusion search, RPG system adds complexity, GOB storage format less queryable than SQL, no reranking.

#### SymDex
- **Architecture:** Python, tree-sitter-based parser, MCP-first design with 20 tools. Byte-precise symbol extraction.
- **Storage:** SQLite + sqlite-vec. One `.db` file per repo. Zero infrastructure.
- **Retrieval:** Three modes: symbol search (by name, byte offsets), semantic search (embedding similarity), text search (regex/literal). Also call graph and HTTP route indexing.
- **Strengths:** Byte-precise extraction (agents read exact bytes, not whole files), rich MCP tool surface (20 tools), HTTP route indexing (Flask/FastAPI/Django/Express), cross-repo registry, Mermaid diagram generation, circular dependency detection, full CLI alongside MCP.
- **Weaknesses:** Newer project (121 stars), 13 languages only, no reranking, no hybrid fusion ranking, no BM25-like scoring, relies on generic embedding model (MiniLM).

#### Zoekt (Sourcegraph)
- **Architecture:** Go. Battle-tested trigram-based code search engine. Used internally by Sourcegraph for searching millions of repos. Custom binary shard format.
- **Storage:** Custom on-disk index shards with trigram posting lists. Designed for fast substring and regexp matching.
- **Retrieval:** Trigram-based lexical search with regexp support. BM25 scoring available via JSON API. Symbol-aware ranking using ctags. Boolean query language (AND, OR, NOT). Supports `file:`, `lang:`, `repo:` filters.
- **Strengths:** Extremely fast and proven at massive scale (100B+ documents at GitHub-scale), rich query language, symbol-aware ranking, streaming results, gRPC API, active maintenance by Sourcegraph + GitLab contributors. The gold standard for lexical code search.
- **Weaknesses:** No semantic/vector search, no natural language queries, requires ctags for symbol info (not tree-sitter), heavyweight for single-project local use, no MCP interface, no embedding support.

#### codemogger
- **Architecture:** TypeScript/Bun. Tree-sitter via WASM for AST chunking. Turso (embedded SQLite) with FTS + vector extensions. Designed as a library-first tool.
- **Storage:** SQLite (Turso) with int8 quantized vectors (vector8, 3.9x smaller than float32). FTS for keyword search. Single `.db` file per project.
- **Retrieval:** True hybrid: semantic (vector cosine similarity) + keyword (FTS with weighted fields). No reranking.
- **Strengths:** Library-first design (SDK with pluggable embedding function), hybrid search out of the box, impressive benchmarks (keyword 25-370x faster than ripgrep), quantized embeddings for compact storage, clean architecture.
- **Weaknesses:** Small project (284 stars), TypeScript/Bun dependency, only 13 languages, limited MCP surface (3 tools), no reranking, no call graph.

#### Aider (Repository Map)
- **Architecture:** Not an indexer per se. Uses tree-sitter to build a structural map of the repository showing key symbols (functions, classes) with their signatures. Graph-based ranking using PageRank-like algorithm to identify most important symbols.
- **Retrieval:** No vector search. Produces a compact, ranked text summary of the repo structure that fits within token budget. The LLM uses this map to understand codebase layout and request specific files.
- **Strengths:** Elegant and simple — no vectors, no database, no embeddings. The map itself IS the context. Graph ranking identifies most relevant symbols. Extremely token-efficient (configurable via `--map-tokens`, default 1k tokens).
- **Weaknesses:** Not a search tool — cannot answer arbitrary queries. Static map, not interactive search. Requires the LLM to interpret the map and request files. No semantic search capability.

---

## 2. Major AI Coding Tool Approaches

### 2.1 Claude Code: Agentic Search (No RAG)

**Key insight:** Boris Cherny (Claude Code creator at Anthropic) confirmed that early Claude Code used RAG + a local vector DB, but they switched to **agentic search** because it was "overwhelmingly better."

**How it works:**
- Claude Code uses **tool calls** to search: `grep`, `ripgrep`, `find`, `cat`, file reading
- The agent iteratively searches, reads results, refines queries, and reads more files
- No pre-built index, no embeddings, no vector database
- The LLM's reasoning ability drives the search strategy

**Why Anthropic chose this:**
1. **Staleness:** Vector indexes go stale as code changes; agent search always reads live files
2. **Reliability:** No index corruption, no embedding model drift, no version mismatches
3. **Security:** No data stored in vectors that could leak; code stays on disk
4. **Simplicity:** No infrastructure to maintain
5. **Adaptability:** The agent can change search strategy based on what it finds

**Criticisms (from Milvus blog and community):**
- Token-intensive: reading many files to find the right one burns tokens
- Slow for large codebases: sequential file reading is inherently slower than indexed lookup
- Precision issues: grep/ripgrep can't do semantic/conceptual search
- Counter-argument: agents with 200k+ context windows can afford the extra tokens

**Bottom line:** Claude Code proves agentic search works for the claude.ai/claude-code use case where a powerful model with large context and cheap tool calls can brute-force search. But this is expensive, slow, and may not work for less capable models or cost-sensitive deployments.

### 2.2 Cursor: Full RAG Pipeline

**How it works (from TDS article and Cursor docs):**
1. **Chunking:** Tree-sitter AST-based chunking — code split along semantic boundaries (functions, classes, blocks)
2. **Embedding:** Custom-trained embedding model (proprietary)
3. **Privacy:** File path obfuscation on client side before transmission
4. **Storage:** Turbopuffer (serverless vector DB backed by object storage). Embeddings cached in AWS by chunk hash.
5. **Retrieval:** Semantic search via vector similarity + hybrid approach with regex/grep fallback
6. **Incremental updates:** Merkle tree of file hashes — only changed files re-embedded. Sync every ~5 minutes.

**Key metrics from Cursor blog (Jan 2026):**
- Semantic search improved response accuracy by **12.5% on average**
- Code changes were more likely to be retained by users

**Design choices worth noting:**
- Source code NEVER stored remotely — only embeddings + masked metadata
- Turbopuffer for fast semantic search at scale
- Merkle trees for efficient change detection (like git)
- Hybrid semantic + lexical approach

### 2.3 OpenCode: Structural Navigation (No Semantic Search)

- Uses text search (ripgrep), file glob matching, and LSP-based navigation
- No embedding-based semantic search
- Strong structural awareness but limited semantic retrieval
- Relies on agent tool use patterns similar to Claude Code

---

## 3. Common Architectural Patterns Observed

### 3.1 Universal Patterns
1. **Tree-sitter is the de facto standard** for code parsing. Every modern tool uses it (cocoindex-code, grepai, SymDex, codemogger, Aider, Cursor). Zoekt is the exception (uses ctags).
2. **SQLite as the storage backbone.** SymDex, codemogger, and cocoindex-code all use SQLite + vector extensions. Zero-infra, single-file, portable.
3. **Local-first design.** Every tool emphasizes no Docker, no server, no API keys for core functionality.
4. **MCP as the agent interface.** All new tools (2025-2026) support MCP as a primary or secondary interface for AI agent integration.
5. **Incremental indexing via content hashing.** SHA-256 or similar per-file hashing to skip unchanged files.

### 3.2 Divergent Approaches
1. **Retrieval strategy:** Pure semantic (cocoindex-code) vs. pure lexical (Zoekt) vs. hybrid (codemogger, Cursor) vs. no index (Claude Code)
2. **Embedding model choice:** Generic (MiniLM) vs. code-optimized (Voyage Code 3, CodeRankEmbed) vs. custom (Cursor)
3. **Graph vs. flat retrieval:** grepai and SymDex build call graphs; others don't
4. **Byte-precise vs. chunk-based:** SymDex returns exact byte ranges; others return whole chunks
5. **CLI-first vs. MCP-first:** cocoindex-code prefers CLI + Skill; SymDex is MCP-first with 20 tools

### 3.3 Emerging Patterns
1. **Skill-based integration** (cocoindex-code) — teaching agents WHEN and HOW to use the tool, not just exposing tools
2. **Background daemons** for transparent index maintenance (cocoindex-code, grepai)
3. **RPG/hierarchical code understanding** (grepai) — grouping symbols into categories/areas for navigation
4. **Quantized embeddings** (codemogger) — int8 vectors for 3.9x storage reduction with minimal quality loss

---

## 4. The "Agentic Search vs Indexed Search" Debate

### Summary

| Aspect | Agentic Search (Claude Code) | Indexed Search (Cursor, cocoindex-code, etc.) |
|---|---|---|
| **Freshness** | Always reads live files — zero staleness | Index can be stale (mitigated by incremental updates) |
| **Token cost** | High — reads many files to find the right one | Low — returns precise results from index |
| **Latency** | Slow for large codebases (sequential file reads) | Fast — pre-computed index, sub-second queries |
| **Semantic capability** | Limited to what grep/ripgrep can match | Full semantic search via embeddings |
| **Infrastructure** | Zero (just file system) | Requires index storage + embedding model |
| **Model dependency** | Needs powerful reasoning model (expensive) | Works with any model that can read results |
| **Reliability** | No moving parts to break | Index corruption, embedding model drift possible |
| **Scale** | Degrades on very large codebases | Designed for large codebases |

### The Nuanced View
The debate is a **false dichotomy**. The best approach is likely **hybrid:**
- Use indexed search for fast, precise retrieval (semantic + keyword)
- Let agents use grep/ripgrep as a fallback for exact matches and fresh searches
- Use the index to provide context maps (like Aider's repomap) alongside search
- Let the agent decide which tool to use based on the query type

**Vera's opportunity:** Build a tool that combines the best of both worlds — fast indexed search (semantic + keyword + reranking) for most queries, while being designed so agents can also fall back to agentic file reading when needed.

---

## 5. Key Technical Decisions the Landscape Suggests

### 5.1 Storage
- **SQLite + vector extension** is the clear winner for local-first tools. Used by cocoindex-code, SymDex, codemogger.
- **LanceDB** (Vera's consideration) is a viable alternative — columnar, embedded, Rust-native, but less proven in this space.
- Avoid Turbopuffer/cloud unless building a hosted product (Cursor's approach).
- **Recommendation:** SQLite (proven, portable) or LanceDB (Rust-native, modern). LanceDB's Rust bindings may align better with Vera's Rust direction.

### 5.2 Chunking
- **AST-aware chunking via tree-sitter** is universally adopted. The debate is about granularity:
  - **Function/class level** (cocoindex-code, codemogger, SymDex) — most common
  - **Symbol-level with byte precision** (SymDex) — most token-efficient
  - **Sliding AST windows** — useful as fallback for long functions
- **cAST paper (arxiv 2506.15655):** Academic work confirms AST-based chunking significantly outperforms naive chunking for code retrieval
- **Best practice:** Symbol-aware chunks as primary, with splitting for items >150 lines (codemogger's approach). Metadata (file path, line range, language, symbol type) stored alongside.

### 5.3 Embedding Models
Based on Modal's comparison and MTEB leaderboard (as of March 2026):

| Model | Params | Context | Code-optimized | Open | Best for |
|---|---|---|---|---|---|
| **VoyageCode3** | Unknown | 32K | ✅ Yes | API only | Best quality if API OK |
| **CodeRankEmbed** | 137M | 8192 | ✅ Yes | MIT | Best open small model for code |
| **Nomic Embed Code** | 7B | 2048 | ✅ Yes | Apache 2.0 | Best open large model |
| **Qwen3-Embedding-8B** | 8B | 32K | Partially | Apache 2.0 | Multilingual + code |
| **Jina Code v2** | 137M | 8192 | ✅ Yes | Apache 2.0 | Fast inference |
| **all-MiniLM-L6-v2** | 22M | 256 | ❌ No | Apache 2.0 | Quick demo/default |
| **OpenAI text-embedding-3-large** | Unknown | 8191 | Partially | API only | General quality |
| **Cohere embed-v4.0** | Unknown | 128K | Partially | API only | Long context |

- **Key insight:** Most current tools use all-MiniLM-L6-v2 as default — a general-purpose model with only 256 token context. This is a **significant weakness** in the current landscape. Code-optimized models like CodeRankEmbed and VoyageCode3 deliver meaningfully better results.
- **Vera's opportunity:** Default to a code-optimized model (CodeRankEmbed at 137M for local, VoyageCode3 or Qwen3-Embedding for remote) instead of generic MiniLM. This alone could be a differentiator.

### 5.4 Retrieval & Ranking

**BM25 vs Vector Search for Code:**
- **GitHub's experience** (from ZenML analysis): Chose BM25 over vector search for 100B+ document scale due to latency and cost. Vector search better for semantic queries but BM25 better for exact identifier lookup.
- **Hybrid is the consensus:** Both academic literature and practitioner experience converge on hybrid (BM25 + vector) with fusion as the best approach for code.
- **Fusion method:** RRF (Reciprocal Rank Fusion) is the standard simple approach. No tool in the landscape currently uses weighted RRF or learned fusion.
- **Reranking:** **No tool in the landscape uses reranking.** This is a massive gap. Cross-encoder rerankers (BGE reranker, Qwen3-Reranker) can dramatically improve precision over bi-encoder embedding search.

**Vera's opportunity:** A hybrid retrieval pipeline (BM25 + vector) with a reranking stage would be **unique** in this landscape. No existing tool does this.

### 5.5 Interface Design
- **CLI is primary for agent integration** — Claude Code, Codex, etc. all prefer CLI tools
- **MCP is secondary but growing** — universal agent protocol
- **Skill-based integration** (cocoindex-code) is the newest pattern — teaching agents when to use the tool
- **Library/SDK** (codemogger) — important for embedding into other tools

---

## 6. Gaps in the Current Landscape That Vera Could Fill

### Gap 1: No Hybrid Search + Reranking
**None** of the current tools combine BM25 + vector search + reranking. This is the proven state-of-the-art in information retrieval, yet every code search tool does either pure semantic OR pure lexical. Vera with hybrid retrieval + reranking would be the first.

### Gap 2: Code-Optimized Embeddings as Default
Every tool defaults to all-MiniLM-L6-v2 (general text, 22M params, 256 token context). Vera defaulting to a code-optimized embedding model (CodeRankEmbed, Qwen3-Embedding) would immediately improve retrieval quality.

### Gap 3: Compact, Agent-Optimized Output
Most tools return raw code chunks. None produce "context capsules" — structured, compact outputs with:
- Symbol metadata (type, signature, docstring)
- Relationship context (what calls this? what does this call?)
- Relevance explanation
- Suggested follow-up queries

### Gap 4: Rust-Native Performance
Every tool is Python, Go, or TypeScript. No Rust-native code indexer exists (cocoindex-code's engine is Rust, but the tool layer is Python). A pure Rust tool would offer:
- Fastest possible indexing and search
- Single binary distribution
- Best tree-sitter integration (tree-sitter is natively Rust/C)
- No runtime dependencies

### Gap 5: Wide Language Support with Depth Tiers
Most tools support 13-30 languages with uniform (shallow) depth. No tool explicitly tiers its language support to provide deeper structural understanding for popular languages while maintaining broad fallback coverage.

### Gap 6: Evaluation/Benchmarking
No tool provides standardized evaluation of retrieval quality. codemogger's benchmarks are the best available but only measure speed, not retrieval relevance. Vera shipping with a retrieval quality evaluation suite would build credibility and enable evidence-based development.

### Gap 7: Graph-Lite Enrichment Without Full Graph Complexity
grepai's RPG system is the most advanced but adds significant complexity. SymDex has call graphs but they're simple. No tool hits the sweet spot of "graph-lite" — lightweight relationship metadata (imports, containment, call adjacency) without a full graph database.

---

## 7. Landscape Summary & Strategic Recommendations for Vera

### The landscape in one sentence:
The code indexing space is crowded with AST+embedding tools that do pure semantic search over MiniLM embeddings stored in SQLite, exposed via MCP — but none combine hybrid retrieval, reranking, code-optimized embeddings, or Rust-native performance.

### What "better" looks like for Vera:
1. **Hybrid retrieval pipeline:** BM25 (for exact identifier lookup) + vector search (for semantic/conceptual queries) + RRF fusion + reranking (for precision)
2. **Code-optimized embeddings:** Default to CodeRankEmbed (local) or Qwen3-Embedding/VoyageCode3 (remote) instead of generic MiniLM
3. **Reranking stage:** First tool to add cross-encoder reranking — dramatic precision improvement over bi-encoder alone
4. **Compact agent-friendly outputs:** Context capsules, not raw code dumps
5. **Rust-native performance:** Single binary, fastest indexing, best tree-sitter integration
6. **Evidence-based:** Ship with evaluation harness and published benchmarks vs. competitors
7. **Tiered language support:** Deep structure-aware retrieval for top 20 languages, graceful fallback for everything else

### Risk:
The space is moving fast. cocoindex-code had 1.1k stars in ~1 month. grepai hit 1.5k. Any of these tools could add hybrid search or reranking. Vera's advantage must come from doing **all of it well** in a cohesive, performant package — not from any single feature.
