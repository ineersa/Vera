//! Persistent storage backends.
//!
//! This module is responsible for:
//! - SQLite database for chunk metadata
//! - sqlite-vec extension for vector storage and similarity search
//! - Tantivy index for BM25 full-text search
//! - File-level content hashing for incremental indexing

#[cfg(test)]
mod tests {
    #[test]
    fn module_loads() {
        // Placeholder: will be replaced with real storage tests.
    }
}
