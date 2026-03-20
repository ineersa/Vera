//! Source code parsing using tree-sitter.
//!
//! This module is responsible for:
//! - Loading tree-sitter grammars for supported languages
//! - Parsing source files into ASTs
//! - Extracting symbol-level chunks (functions, classes, structs, etc.)
//! - Tier 0 fallback chunking for unsupported languages

#[cfg(test)]
mod tests {
    #[test]
    fn module_loads() {
        // Placeholder: will be replaced with real parsing tests.
    }
}
