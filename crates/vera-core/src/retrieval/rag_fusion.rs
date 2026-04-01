//! Deep search via RAG Fusion.
//!
//! Flow:
//! 1. Expand the user query into multiple variants using a completion model.
//! 2. Execute standard hybrid search for each query variant.
//! 3. Merge and rerank all results together with reciprocal rank fusion.
//!
//! If completion query-expansion is not configured, this module silently
//! falls back to a normal single-query search.

use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use tracing::warn;

use crate::config::{InferenceBackend, VeraConfig};
use crate::types::{SearchFilters, SearchResult};

use super::completion_client::CompletionClient;
use super::hybrid::fuse_rrf_multi;
use super::search_service::{SearchTimings, execute_search};

/// Execute deep search with query expansion + reciprocal rank fusion.
pub fn execute_rag_fusion_search(
    index_dir: &Path,
    query: &str,
    config: &VeraConfig,
    filters: &SearchFilters,
    result_limit: usize,
    backend: InferenceBackend,
) -> Result<(Vec<SearchResult>, SearchTimings)> {
    let overall_start = Instant::now();

    let completion_client = match CompletionClient::from_env_if_configured() {
        Ok(Some(client)) => client,
        Ok(None) => {
            return execute_search(index_dir, query, config, filters, result_limit, backend);
        }
        Err(error) => {
            return Err(anyhow!(
                "failed to initialize deep-search completion client: {error}"
            ));
        }
    };

    let expanded_queries = completion_client
        .expand_query(query)
        .map_err(|error| anyhow!("failed to generate deep-search query candidates: {error}"))?;

    let queries = dedupe_queries_with_original(query, expanded_queries);
    if queries.len() <= 1 {
        return Err(anyhow!(
            "failed to generate deep-search query candidates: no additional rewrites were produced"
        ));
    }

    let mut aggregated_timings = SearchTimings::default();
    let mut per_query_results: Vec<Vec<SearchResult>> = Vec::with_capacity(queries.len());
    let per_query_limit = compute_per_query_limit(result_limit);

    for (idx, expanded_query) in queries.iter().enumerate() {
        match execute_search(
            index_dir,
            expanded_query,
            config,
            filters,
            per_query_limit,
            backend,
        ) {
            Ok((results, timings)) => {
                merge_timings(&mut aggregated_timings, &timings);
                per_query_results.push(results);
            }
            Err(error) if idx == 0 => return Err(error),
            Err(error) => {
                warn!(
                    query = %expanded_query,
                    error = %error,
                    "deep-search subquery failed; continuing with remaining queries"
                );
            }
        }
    }

    if per_query_results.is_empty() {
        return Err(anyhow!(
            "deep search failed: all generated query candidates failed"
        ));
    }

    let query_result_slices: Vec<&[SearchResult]> =
        per_query_results.iter().map(Vec::as_slice).collect();
    let fused = fuse_rrf_multi(&query_result_slices, config.retrieval.rrf_k, result_limit);

    aggregated_timings.total = Some(overall_start.elapsed());
    Ok((fused, aggregated_timings))
}

fn dedupe_queries_with_original(original: &str, alternatives: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::with_capacity(alternatives.len() + 1);
    let mut seen = HashSet::new();

    let original = normalize_query(original);
    if !original.is_empty() {
        seen.insert(original.to_ascii_lowercase());
        deduped.push(original);
    }

    for alternative in alternatives {
        let normalized = normalize_query(&alternative);
        if normalized.is_empty() {
            continue;
        }
        let key = normalized.to_ascii_lowercase();
        if seen.insert(key) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_queries_preserves_original_order() {
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
                "auth token refresh".to_string(),
                "jwt expiry handling".to_string(),
                "auth middleware".to_string(),
            ]
        );
    }

    #[test]
    fn per_query_limit_overfetches_for_fusion() {
        assert_eq!(compute_per_query_limit(5), 20);
        assert_eq!(compute_per_query_limit(20), 40);
    }

    #[test]
    fn merge_timings_sums_stage_durations() {
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
        assert_eq!(target.vector, Some(Duration::from_millis(60)));
        assert_eq!(target.fusion, Some(Duration::from_millis(80)));
        assert_eq!(target.reranking, Some(Duration::from_millis(100)));
        assert_eq!(target.augmentation, Some(Duration::from_millis(120)));
    }
}
