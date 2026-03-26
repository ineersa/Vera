# Why Rust

Vera gets invoked hundreds of times per agent session. It needs fast tree-sitter parsing, sub-millisecond startup, and single-binary distribution. We spiked both Rust and TypeScript/Bun on identical workloads.

## Benchmarks

AMD Ryzen 5 7600X3D, 30GB RAM, Arch Linux. Rust 1.94.0, Bun 1.3.11.

### Tree-sitter parsing (parse + full AST walk)

| File | Rust (ms) | Bun (ms) | Speedup |
|------|-----------|----------|---------|
| ripgrep flags/defs.rs (7.8K LOC) | 16.9 | 29.7 | 1.76× |
| fastify hooks.test.js (3.6K LOC) | 10.4 | 17.9 | 1.72× |
| turborepo builder.rs (4.9K LOC) | 14.0 | 22.8 | 1.63× |

### CLI cold start (100 invocations)

| Runtime | Avg (ms) |
|---------|----------|
| Rust | 0.51 |
| Bun | 5.09 |
| Node | 35.52 |

### Distribution size

| Artifact | Size |
|----------|------|
| Estimated Vera binary (all grammars) | ~10–15 MB |
| Bun compiled binary | ~60–80 MB |
| TS node_modules (tree-sitter only) | 32 MB |

## Why it wins

- 1.6–1.8× faster parsing — same underlying C library, less FFI overhead
- 10× faster cold start than Bun
- Single ~10MB binary vs runtime + node_modules
- `ignore` crate (from ripgrep) gives gitignore-aware walking out of the box
- Tantivy, sqlite-vec, tree-sitter grammars all have first-class Rust support

## Trade-offs

- Slower iteration during development vs TypeScript
- Steeper contributor learning curve
- MCP server needs more boilerplate than a Node equivalent
