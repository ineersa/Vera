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
            let mut split = Vec::new();
            for (start, end) in windows {
                let segment = lines[start..end].join("\n");
                if estimate_tokens(&segment) > window_tokens {
                    split.extend(split_single_line_with_overlap(
                        &segment,
                        window_tokens,
                        overlap_tokens,
                    ));
                } else {
                    split.push(segment);
                }
            }
            return split;
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
    if text.is_empty() {
        return vec![String::new()];
    }

    let window_tokens = window_tokens.max(1);
    let spans = estimated_token_spans(text);
    if spans.is_empty() {
        return vec![text.to_string()];
    }

    let mut windows = Vec::new();
    let mut start = 0usize;

    while start < spans.len() {
        let mut end = start;
        let mut used = 0usize;

        while end < spans.len() {
            let next = used.saturating_add(spans[end].weight);
            if next > window_tokens && end > start {
                break;
            }

            used = next;
            end += 1;
        }

        if end == start {
            end += 1;
        }

        let start_byte = spans[start].start;
        let end_byte = spans[end - 1].end;
        windows.push(text[start_byte..end_byte].to_string());

        if end >= spans.len() {
            break;
        }

        if overlap_tokens == 0 {
            start = end;
            continue;
        }

        let mut next_start = end;
        let mut overlap_used = 0usize;
        while next_start > start {
            let candidate = overlap_used.saturating_add(spans[next_start - 1].weight);
            if candidate > overlap_tokens && overlap_used > 0 {
                break;
            }

            overlap_used = candidate;
            next_start -= 1;

            if overlap_used >= overlap_tokens {
                break;
            }
        }

        start = if next_start <= start { end } else { next_start };
    }

    windows
}

#[derive(Debug, Clone, Copy)]
struct TokenSpan {
    start: usize,
    end: usize,
    weight: usize,
}

fn estimated_token_spans(text: &str) -> Vec<TokenSpan> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    if chars.is_empty() {
        return Vec::new();
    }

    let mut spans = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let (start, ch) = chars[i];
        let end = chars.get(i + 1).map(|(idx, _)| *idx).unwrap_or(text.len());

        if ch.is_ascii_alphanumeric() || ch == '_' {
            let mut j = i + 1;
            while j < chars.len() {
                let (_, next_ch) = chars[j];
                if next_ch.is_ascii_alphanumeric() || next_ch == '_' {
                    j += 1;
                } else {
                    break;
                }
            }

            let run_end = chars.get(j).map(|(idx, _)| *idx).unwrap_or(text.len());
            let mut cursor = start;
            while cursor < run_end {
                let segment_end = (cursor + CHARS_PER_TOKEN_ESTIMATE).min(run_end);
                spans.push(TokenSpan {
                    start: cursor,
                    end: segment_end,
                    weight: 1,
                });
                cursor = segment_end;
            }

            i = j;
            continue;
        }

        spans.push(TokenSpan {
            start,
            end,
            weight: if ch.is_whitespace() { 0 } else { 1 },
        });
        i += 1;
    }

    spans
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
            assert!(estimate_tokens(window) <= 120);
        }
    }

    #[test]
    fn split_text_with_overlap_splits_punctuation_dense_line() {
        let text = "{".repeat(2000);
        let windows = split_text_with_token_overlap(&text, 120, 20);
        assert!(windows.len() > 1);
        for window in &windows {
            assert!(estimate_tokens(window) <= 120);
        }
    }

    #[test]
    fn split_text_with_overlap_handles_oversized_line_in_multiline_text() {
        let text = format!("header\n{}\nfooter", "{".repeat(2000));
        let windows = split_text_with_token_overlap(&text, 120, 20);
        assert!(windows.len() > 2);
        for window in &windows {
            assert!(estimate_tokens(window) <= 120);
        }
    }
}
