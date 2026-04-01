//! Deep search via RAG Fusion.
//!
//! 1. Expand the user query into multiple variants using a completion model.
//! 2. Execute standard hybrid search for each variant.
//! 3. Merge all results with reciprocal rank fusion.
//!
//! Falls back to iterative (symbol-following) search when no completion
//! endpoint is configured.

use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use tracing::{debug, warn};

use crate::config::{InferenceBackend, VeraConfig};
use crate::retrieval::bm25::search_bm25;
use crate::types::{SearchFilters, SearchResult};

use super::completion_client::CompletionClient;
use super::hybrid::fuse_rrf_multi;
use super::search_service::{SearchTimings, execute_search};

/// Execute deep search: RAG-fusion if a completion endpoint is configured,
/// otherwise fall back to iterative symbol-following search.
pub fn execute_deep_search(
    index_dir: &Path,
    query: &str,
    config: &VeraConfig,
    filters: &SearchFilters,
    result_limit: usize,
    backend: InferenceBackend,
) -> Result<(Vec<SearchResult>, SearchTimings)> {
    let completion_client = match CompletionClient::from_env_if_configured() {
        Ok(Some(client)) => client,
        Ok(None) => {
            return super::iterative_search::execute_iterative_search(
                index_dir,
                query,
                config,
                filters,
                result_limit,
                backend,
                1,
            );
        }
        Err(e) => {
            warn!(error = %e, "completion client init failed, falling back to iterative search");
            return super::iterative_search::execute_iterative_search(
                index_dir,
                query,
                config,
                filters,
                result_limit,
                backend,
                1,
            );
        }
    };

    execute_rag_fusion(
        index_dir,
        query,
        config,
        filters,
        result_limit,
        backend,
        &completion_client,
    )
}

fn execute_rag_fusion(
    index_dir: &Path,
    query: &str,
    config: &VeraConfig,
    filters: &SearchFilters,
    result_limit: usize,
    backend: InferenceBackend,
    completion_client: &CompletionClient,
) -> Result<(Vec<SearchResult>, SearchTimings)> {
    let overall_start = Instant::now();

    // BM25 pre-filter: run a cheap keyword search to gather codebase context
    // (symbol names and file paths) that helps the LLM generate better rewrites.
    let context_hints = bm25_context_hints(index_dir, query);
    debug!(
        hints = context_hints.len(),
        "BM25 pre-filter produced context hints for query expansion"
    );

    let expanded = completion_client
        .expand_query_with_context(query, &context_hints)
        .map_err(|e| anyhow!("failed to generate deep-search query candidates: {e}"))?;

    let queries = dedupe_queries_with_original(query, expanded);
    if queries.len() <= 1 {
        return Err(anyhow!(
            "query expansion produced no additional rewrites; \
             check completion model output"
        ));
    }

    let mut aggregated_timings = SearchTimings::default();
    let mut per_query_results: Vec<Vec<SearchResult>> = Vec::with_capacity(queries.len());
    let per_query_limit = compute_per_query_limit(result_limit);

    for (idx, q) in queries.iter().enumerate() {
        match execute_search(index_dir, q, config, filters, per_query_limit, backend) {
            Ok((results, timings)) => {
                merge_timings(&mut aggregated_timings, &timings);
                per_query_results.push(results);
            }
            Err(e) if idx == 0 => return Err(e),
            Err(e) => {
                warn!(query = %q, error = %e, "deep-search subquery failed; continuing");
            }
        }
    }

    if per_query_results.is_empty() {
        return Err(anyhow!("deep search failed: all query candidates failed"));
    }

    let slices: Vec<&[SearchResult]> = per_query_results.iter().map(Vec::as_slice).collect();
    let fused = fuse_rrf_multi(&slices, config.retrieval.rrf_k, result_limit);

    aggregated_timings.total = Some(overall_start.elapsed());
    Ok((fused, aggregated_timings))
}

fn dedupe_queries_with_original(original: &str, alternatives: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::with_capacity(alternatives.len() + 1);
    let mut seen = std::collections::HashSet::new();

    let original = normalize_query(original);
    if !original.is_empty() {
        seen.insert(original.to_ascii_lowercase());
        deduped.push(original);
    }

    for alt in alternatives {
        let normalized = normalize_query(&alt);
        if normalized.is_empty() {
            continue;
        }
        if seen.insert(normalized.to_ascii_lowercase()) {
            deduped.push(normalized);
        }
    }

    deduped
}

fn normalize_query(query: &str) -> String {
    query.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn compute_per_query_limit(result_limit: usize) -> usize {
    result_limit
        .saturating_mul(2)
        .max(result_limit.saturating_add(10))
        .max(20)
}

fn merge_timings(target: &mut SearchTimings, incoming: &SearchTimings) {
    add_duration(&mut target.embedding, incoming.embedding);
    add_duration(&mut target.bm25, incoming.bm25);
    add_duration(&mut target.vector, incoming.vector);
    add_duration(&mut target.fusion, incoming.fusion);
    add_duration(&mut target.reranking, incoming.reranking);
    add_duration(&mut target.augmentation, incoming.augmentation);
}

fn add_duration(target: &mut Option<Duration>, incoming: Option<Duration>) {
    if let Some(delta) = incoming {
        *target = Some(target.unwrap_or_default() + delta);
    }
}

/// Run a quick BM25 search and extract deduplicated symbol names and file
/// paths from the top results. These hints give the LLM real identifiers
/// from the codebase so it can produce more targeted query rewrites.
const BM25_PREFILTER_LIMIT: usize = 10;
const MAX_CONTEXT_HINTS: usize = 15;

fn bm25_context_hints(index_dir: &Path, query: &str) -> Vec<String> {
    let results = match search_bm25(index_dir, query, BM25_PREFILTER_LIMIT) {
        Ok(r) => r,
        Err(e) => {
            debug!(error = %e, "BM25 pre-filter failed, continuing without context");
            return Vec::new();
        }
    };

    let mut seen = HashSet::new();
    let mut hints = Vec::new();

    for r in &results {
        if let Some(ref sym) = r.symbol_name {
            let hint = format!("symbol: {sym}");
            if seen.insert(hint.clone()) {
                hints.push(hint);
            }
        }
        let hint = format!("file: {}", r.file_path);
        if seen.insert(hint.clone()) {
            hints.push(hint);
        }
        if hints.len() >= MAX_CONTEXT_HINTS {
            break;
        }
    }

    hints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_preserves_original_first() {
        let queries = dedupe_queries_with_original(
            "auth token refresh",
            vec![
                "jwt expiry handling".to_string(),
                "auth middleware".to_string(),
                "AUTH TOKEN REFRESH".to_string(),
            ],
        );
        assert_eq!(
            queries,
            vec![
                "auth token refresh",
                "jwt expiry handling",
                "auth middleware"
            ]
        );
    }

    #[test]
    fn per_query_limit_overfetches() {
        assert_eq!(compute_per_query_limit(5), 20);
        assert_eq!(compute_per_query_limit(20), 40);
    }

    #[test]
    fn merge_timings_sums() {
        let mut target = SearchTimings::default();
        let incoming = SearchTimings {
            embedding: Some(Duration::from_millis(10)),
            bm25: Some(Duration::from_millis(20)),
            vector: Some(Duration::from_millis(30)),
            fusion: Some(Duration::from_millis(40)),
            reranking: Some(Duration::from_millis(50)),
            augmentation: Some(Duration::from_millis(60)),
            total: None,
        };
        merge_timings(&mut target, &incoming);
        merge_timings(&mut target, &incoming);
        assert_eq!(target.embedding, Some(Duration::from_millis(20)));
        assert_eq!(target.bm25, Some(Duration::from_millis(40)));
    }
}
