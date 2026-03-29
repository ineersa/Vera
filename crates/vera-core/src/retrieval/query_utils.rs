//! Shared query-parsing and path utilities used across ranking and search.

/// Count directory separators in a path.
pub(crate) fn path_depth(path: &str) -> usize {
    path.matches('/').count() + path.matches('\\').count()
}

/// Strip non-identifier punctuation from the edges of a query token.
pub(crate) fn trim_query_token(token: &str) -> &str {
    token.trim_matches(|ch: char| {
        !ch.is_ascii_alphanumeric() && !matches!(ch, '.' | '_' | '-' | '/')
    })
}

/// Check whether a token looks like a compound identifier (snake_case, CamelCase, or `::` path).
pub(crate) fn looks_like_compound_identifier(token: &str) -> bool {
    token.contains('_') || token.contains("::") || token.chars().any(|ch| ch.is_ascii_uppercase())
}

/// Check whether a (lowercased) token looks like a filename.
pub(crate) fn looks_like_filename(token: &str) -> bool {
    matches!(
        token,
        "dockerfile" | "makefile" | "cmakelists.txt" | "nginx.conf"
    ) || token.contains('.')
}
