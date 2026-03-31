//! Lightweight token-budget helpers.
//!
//! Vera supports multiple embedding/reranker backends, many of which do not
//! expose model tokenizers directly. This module provides deterministic,
//! low-cost token estimation and token-aware windowing utilities.

/// Conservative chars-per-token estimate used for byte->token fallback.
pub const CHARS_PER_TOKEN_ESTIMATE: usize = 4;

/// Estimate token count for arbitrary text.
///
/// This is intentionally conservative for code-like content: punctuation and
/// operators count as standalone tokens while long alphanumeric spans are
/// approximated as 1 token per ~4 chars.
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    let mut tokens = 0usize;
    let mut alnum_run = 0usize;

    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            alnum_run += 1;
            continue;
        }

        if alnum_run > 0 {
            tokens += alnum_run.div_ceil(CHARS_PER_TOKEN_ESTIMATE);
            alnum_run = 0;
        }

        if ch.is_whitespace() {
            continue;
        }

        tokens += 1;
    }

    if alnum_run > 0 {
        tokens += alnum_run.div_ceil(CHARS_PER_TOKEN_ESTIMATE);
    }

    tokens.max(1)
}

/// Convert a byte budget to an approximate token budget.
pub fn token_budget_from_bytes(max_bytes: usize) -> usize {
    if max_bytes == 0 {
        0
    } else {
        max_bytes.div_ceil(CHARS_PER_TOKEN_ESTIMATE)
    }
}

/// Estimate token counts for each source line.
pub fn line_token_counts(lines: &[&str]) -> Vec<usize> {
    lines
        .iter()
        .map(|line| estimate_tokens(line).saturating_add(1)) // newline separator
        .collect()
}

/// Compute line windows that fit within `max_tokens` with token overlap.
///
/// Returns `(start, end)` ranges where `end` is exclusive.
pub fn line_windows_by_tokens(
    line_tokens: &[usize],
    max_tokens: usize,
    overlap_tokens: usize,
) -> Vec<(usize, usize)> {
    if line_tokens.is_empty() {
        return Vec::new();
    }

    let max_tokens = max_tokens.max(1);
    let mut windows = Vec::new();
    let mut start = 0usize;

    while start < line_tokens.len() {
        let mut end = start;
        let mut used = 0usize;

        while end < line_tokens.len() {
            let next = used.saturating_add(line_tokens[end]);
            if next > max_tokens && end > start {
                break;
            }

            // Ensure forward progress even when a single line exceeds the budget.
            if next > max_tokens && end == start {
                end += 1;
                break;
            }

            used = next;
            end += 1;
        }

        windows.push((start, end));

        if end >= line_tokens.len() {
            break;
        }

        if overlap_tokens == 0 {
            start = end;
            continue;
        }

        // Walk backward from end to accumulate overlap token budget.
        let mut next_start = end;
        let mut overlap_used = 0usize;
        while next_start > start {
            let candidate = overlap_used.saturating_add(line_tokens[next_start - 1]);
            if candidate > overlap_tokens && overlap_used > 0 {
                break;
            }

            overlap_used = candidate;
            next_start -= 1;

            if overlap_used >= overlap_tokens {
                break;
            }
        }

        // Never allow a zero-progress overlap loop.
        start = if next_start <= start { end } else { next_start };
    }

    windows
}

/// Split text into token-budgeted windows with overlap.
pub fn split_text_with_token_overlap(
    text: &str,
    window_tokens: usize,
    overlap_tokens: usize,
) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let window_tokens = window_tokens.max(1);
    if estimate_tokens(text) <= window_tokens {
        return vec![text.to_string()];
    }

    let lines: Vec<&str> = text.lines().collect();
    if lines.len() > 1 {
        let line_tokens = line_token_counts(&lines);
        let windows = line_windows_by_tokens(&line_tokens, window_tokens, overlap_tokens);
        if !windows.is_empty() {
            return windows
                .into_iter()
                .map(|(start, end)| lines[start..end].join("\n"))
                .collect();
        }
    }

    // Fallback for single-line/minified content.
    split_single_line_with_overlap(text, window_tokens, overlap_tokens)
}

/// Truncate text to approximately `max_tokens`.
pub fn truncate_to_token_budget(text: &str, max_tokens: usize) -> String {
    if max_tokens == 0 || estimate_tokens(text) <= max_tokens {
        return text.to_string();
    }

    // Prefer truncating at line boundaries when possible.
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() > 1 {
        let mut used = 0usize;
        let mut end = 0usize;
        while end < lines.len() {
            let line_tokens = estimate_tokens(lines[end]).saturating_add(1);
            if used.saturating_add(line_tokens) > max_tokens {
                break;
            }
            used = used.saturating_add(line_tokens);
            end += 1;
        }
        if end > 0 {
            return lines[..end].join("\n");
        }
    }

    split_single_line_with_overlap(text, max_tokens, 0)
        .into_iter()
        .next()
        .unwrap_or_default()
}

fn split_single_line_with_overlap(
    text: &str,
    window_tokens: usize,
    overlap_tokens: usize,
) -> Vec<String> {
    let window_bytes = window_tokens
        .saturating_mul(CHARS_PER_TOKEN_ESTIMATE)
        .max(1)
        .min(text.len().max(1));
    let overlap_bytes = overlap_tokens
        .saturating_mul(CHARS_PER_TOKEN_ESTIMATE)
        .min(window_bytes.saturating_sub(1));

    let mut windows = Vec::new();
    let mut start = 0usize;

    while start < text.len() {
        let mut end = (start + window_bytes).min(text.len());
        while end > start && !text.is_char_boundary(end) {
            end -= 1;
        }
        if end == start {
            end = next_char_boundary(text, start + 1);
        }

        windows.push(text[start..end].to_string());

        if end >= text.len() {
            break;
        }

        let mut next_start = end.saturating_sub(overlap_bytes);
        while next_start > 0 && !text.is_char_boundary(next_start) {
            next_start -= 1;
        }

        start = if next_start <= start { end } else { next_start };
    }

    windows
}

fn next_char_boundary(text: &str, mut idx: usize) -> usize {
    idx = idx.min(text.len());
    while idx < text.len() && !text.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_budget_converts_to_tokens() {
        assert_eq!(token_budget_from_bytes(0), 0);
        assert_eq!(token_budget_from_bytes(4), 1);
        assert_eq!(token_budget_from_bytes(5), 2);
    }

    #[test]
    fn line_windows_make_forward_progress() {
        let lines = vec![10usize, 10, 10, 10, 10];
        let windows = line_windows_by_tokens(&lines, 15, 6);
        assert!(!windows.is_empty());
        for (idx, (start, end)) in windows.iter().enumerate() {
            assert!(start < end, "window must have positive length");
            if idx > 0 {
                assert!(
                    start <= &windows[idx - 1].1,
                    "window order should be monotonic"
                );
            }
        }
    }

    #[test]
    fn split_text_with_overlap_splits_single_line() {
        let text = "x".repeat(2000);
        let windows = split_text_with_token_overlap(&text, 120, 20);
        assert!(windows.len() > 1);
        for window in &windows {
            assert!(estimate_tokens(window) <= 160);
        }
    }
}
