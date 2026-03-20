//! Vera MCP Server — Model Context Protocol interface for Vera.
//!
//! Exposes Vera's indexing and retrieval capabilities as MCP tools:
//! - `search_code` — search the indexed codebase
//! - `index_project` — trigger indexing of a project
//! - `update_project` — trigger incremental index update
//! - `get_stats` — retrieve index statistics

#[cfg(test)]
mod tests {
    #[test]
    fn module_loads() {
        // Placeholder: will be replaced with real MCP server tests.
    }
}
