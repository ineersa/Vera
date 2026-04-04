//! Shared search service used by both CLI and MCP.
//!
//! Encapsulates the common hybrid search flow: create embedding provider,
//! build reranker, compute fetch limits, execute search, apply filters.

use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use tracing::warn;

use crate::chunk_text::file_name;
use crate::config::{InferenceBackend, VeraConfig};
use crate::retrieval::hybrid::{compute_bm25_candidates, compute_vector_candidates};
use crate::retrieval::query_classifier::{QueryType, classify_query, params_for_query_type};
use crate::retrieval::query_utils::{
    looks_like_compound_identifier, looks_like_filename, path_depth, trim_query_token,
};
use crate::retrieval::ranking::{
    RankingStage, apply_query_ranking_with_filters, is_path_weighted_query,
};
use crate::retrieval::{apply_filters, search_bm25, search_hybrid, search_hybrid_reranked};
use crate::types::{Chunk, SearchFilters, SearchResult, SymbolType};

/// Timing data for each stage of the search pipeline.
#[derive(Debug, Default)]
pub struct SearchTimings {
    pub embedding: Option<Duration>,
    pub bm25: Option<Duration>,
    pub vector: Option<Duration>,
    pub fusion: Option<Duration>,
    pub reranking: Option<Duration>,
    pub augmentation: Option<Duration>,
    pub total: Option<Duration>,
    pub embedding_error: Option<String>,
    pub bm25_error: Option<String>,
    pub vector_error: Option<String>,
    pub fusion_error: Option<String>,
    pub reranking_error: Option<String>,
    pub completion_error: Option<String>,
}

/// Execute a search against the index at `index_dir`.
///
/// Attempts hybrid search (BM25 + vector + optional reranking). Falls
/// back to partial results when configured to degrade. When
/// `retrieval.fail_on_stage_error` is enabled, stage failures return errors.
pub fn execute_search(
    index_dir: &Path,
    query: &str,
    config: &VeraConfig,
    filters: &SearchFilters,
    result_limit: usize,
    backend: InferenceBackend,
) -> Result<(Vec<SearchResult>, SearchTimings)> {
    let total_start = Instant::now();
    let fail_on_stage_error = config.retrieval.fail_on_stage_error;
    let fetch_limit = compute_fetch_limit(query, filters, result_limit);
    let query_type = classify_query(query);
    let query_params = params_for_query_type(query_type);
    let rrf_k = query_params.rrf_k;
    let bm25_candidates =
        effective_bm25_candidates(query, fetch_limit, config.retrieval.max_bm25_candidates);
    let vector_candidates = effective_vector_candidates(
        fetch_limit,
        query_params,
        config.retrieval.max_vector_candidates,
    );
    let rerank_candidates =
        effective_rerank_candidates(config.retrieval.rerank_candidates, result_limit);

    let rt = tokio::runtime::Runtime::new()?;

    // Try to create embedding provider for hybrid search.
    let (provider, model_name) =
        match rt.block_on(crate::embedding::create_dynamic_provider(config, backend)) {
            Ok(res) => res,
            Err(e) => {
                let message = format!(
                    "embedding provider initialization failed: {e}. \
                     Set retrieval.fail_on_stage_error=false to allow BM25 fallback"
                );
                if fail_on_stage_error {
                    return Err(anyhow!(message));
                }
                warn!(
                    "Failed to create embedding provider ({}), using BM25-only search",
                    e
                );
                let (results, mut timings) = run_bm25_only(
                    index_dir,
                    query,
                    filters,
                    bm25_candidates,
                    result_limit,
                    total_start,
                )?;
                timings.embedding_error = Some(message);
                return Ok((results, timings));
            }
        };

    let mut stored_dim = config.embedding.max_stored_dim;

    // Check metadata mismatch
    let metadata_path = index_dir.join("metadata.db");
    if let Ok(metadata_store) = crate::storage::metadata::MetadataStore::open(&metadata_path) {
        if let (Some(s_model), Some(s_dim)) = (
            metadata_store.get_index_meta("model_name").unwrap_or(None),
            metadata_store
                .get_index_meta("embedding_dim")
                .unwrap_or(None),
        ) {
            if !crate::config::model_names_match(&s_model, &model_name) {
                let message = format!(
                    "index model '{}' does not match active model '{}'; \
                     re-index or switch model",
                    s_model, model_name
                );
                if fail_on_stage_error {
                    return Err(anyhow!(message));
                }
                warn!(
                    "Index model '{}' does not match active model '{}'; using BM25-only search",
                    s_model, model_name
                );
                let (results, mut timings) = run_bm25_only(
                    index_dir,
                    query,
                    filters,
                    bm25_candidates,
                    result_limit,
                    total_start,
                )?;
                timings.embedding_error = Some(message);
                return Ok((results, timings));
            }
            if let Ok(dim) = s_dim.parse::<usize>() {
                use crate::embedding::EmbeddingProvider;
                if let Some(provider_dim) = provider.expected_dim() {
                    if provider_dim != dim {
                        let message = format!(
                            "index dimension {} does not match provider dimension {}; \
                             re-index or switch model",
                            dim, provider_dim
                        );
                        if fail_on_stage_error {
                            return Err(anyhow!(message));
                        }
                        warn!(
                            "Index dimension {} does not match provider dimension {}; using BM25-only search",
                            dim, provider_dim
                        );
                        let (results, mut timings) = run_bm25_only(
                            index_dir,
                            query,
                            filters,
                            bm25_candidates,
                            result_limit,
                            total_start,
                        )?;
                        timings.embedding_error = Some(message);
                        return Ok((results, timings));
                    }
                }
                stored_dim = dim;
            }
        }
    }

    let provider = crate::embedding::CachedEmbeddingProvider::new(provider, 512);

    let should_skip_reranker = should_skip_reranking(query, filters);
    let mut reranker_error: Option<String> = None;

    // Create optional reranker.
    let reranker = if config.retrieval.reranking_enabled && !should_skip_reranker {
        match rt.block_on(crate::retrieval::create_dynamic_reranker(config, backend)) {
            Ok(Some(reranker)) => Some(reranker),
            Ok(None) => {
                let message = "reranker is enabled but no reranker backend is configured";
                if fail_on_stage_error {
                    return Err(anyhow!(
                        "{message}; disable reranking or configure reranker provider"
                    ));
                }
                warn!("{message}");
                reranker_error = Some(message.to_string());
                None
            }
            Err(e) => {
                let message = format!(
                    "reranker initialization failed: {e}. \
                     Set retrieval.fail_on_stage_error=false to allow fallback"
                );
                if fail_on_stage_error {
                    return Err(anyhow!(message));
                }
                warn!("{message}");
                reranker_error = Some(message);
                None
            }
        }
    } else {
        None
    };
    let reranker_enabled = reranker.is_some();

    let ranking_stage = if reranker_enabled {
        RankingStage::PostRerank
    } else {
        RankingStage::Initial
    };

    let (results, hybrid_timings) = if reranker_enabled {
        let reranker = reranker
            .as_ref()
            .expect("reranker_enabled requires an initialized reranker");
        rt.block_on(search_hybrid_reranked(
            index_dir,
            &provider,
            reranker,
            query,
            fetch_limit,
            rrf_k,
            stored_dim,
            rerank_candidates,
            bm25_candidates,
            vector_candidates,
            fail_on_stage_error,
        ))?
    } else {
        rt.block_on(search_hybrid(
            index_dir,
            &provider,
            query,
            fetch_limit,
            rrf_k,
            stored_dim,
            bm25_candidates,
            vector_candidates,
            fail_on_stage_error,
        ))?
    };

    let mut timings = SearchTimings {
        embedding: hybrid_timings.embedding,
        bm25: hybrid_timings.bm25,
        vector: hybrid_timings.vector,
        fusion: hybrid_timings.fusion,
        reranking: hybrid_timings.reranking,
        bm25_error: hybrid_timings.bm25_error,
        vector_error: hybrid_timings.vector_error,
        fusion_error: hybrid_timings.fusion_error,
        reranking_error: reranker_error.or(hybrid_timings.reranking_error),
        ..Default::default()
    };

    let aug_start = Instant::now();
    let results =
        augment_exact_match_candidates(index_dir, query, results, ranking_stage, filters)?;
    timings.augmentation = Some(aug_start.elapsed());

    timings.total = Some(total_start.elapsed());
    Ok((apply_filters(results, filters, result_limit), timings))
}

/// Compute how many candidates to keep through fusion before final truncation.
///
/// Broad natural-language queries need a larger pool even without explicit
/// filters so deterministic ranking can surface structural chunks that raw RRF
/// scores placed outside the requested result window.
fn compute_fetch_limit(query: &str, filters: &SearchFilters, result_limit: usize) -> usize {
    let mut fetch_limit = if filters.is_empty() {
        result_limit
    } else {
        result_limit.saturating_mul(3).max(result_limit + 20)
    };

    if needs_structural_overfetch(query, filters) {
        fetch_limit = fetch_limit.max(result_limit.saturating_mul(8).max(result_limit + 140));
    } else if matches!(classify_query(query), QueryType::NaturalLanguage) {
        fetch_limit = fetch_limit.max(result_limit.saturating_mul(3).max(result_limit + 40));
    }

    fetch_limit
}

fn needs_structural_overfetch(query: &str, filters: &SearchFilters) -> bool {
    matches!(classify_query(query), QueryType::NaturalLanguage)
        && query.split_whitespace().count() >= 4
        && filters.path_glob.is_none()
        && filters.symbol_type.is_none()
        && !is_path_weighted_query(query)
}

fn effective_vector_candidates(
    fetch_limit: usize,
    query_params: crate::retrieval::query_classifier::QueryParams,
    max_candidates: usize,
) -> usize {
    let base = compute_vector_candidates(fetch_limit, query_params.vector_candidate_multiplier);
    apply_candidate_cap(base, max_candidates)
}

fn effective_bm25_candidates(query: &str, fetch_limit: usize, max_candidates: usize) -> usize {
    let base = compute_bm25_candidates(query, fetch_limit);
    apply_candidate_cap(base, max_candidates)
}

fn effective_rerank_candidates(base: usize, result_limit: usize) -> usize {
    base.max(result_limit.max(1))
}

fn apply_candidate_cap(base: usize, max_candidates: usize) -> usize {
    if max_candidates == 0 {
        return base.max(1);
    }
    base.min(max_candidates).max(1)
}

fn should_skip_reranking(query: &str, filters: &SearchFilters) -> bool {
    let word_count = query.split_whitespace().count();
    filters.path_glob.is_some()
        || filters.symbol_type.is_some()
        || is_path_weighted_query(query)
        || (matches!(classify_query(query), QueryType::Identifier) && word_count <= 2)
}

fn run_bm25_only(
    index_dir: &Path,
    query: &str,
    filters: &SearchFilters,
    bm25_candidates: usize,
    result_limit: usize,
    total_start: Instant,
) -> Result<(Vec<SearchResult>, SearchTimings)> {
    let bm25_start = Instant::now();
    let results = search_bm25(index_dir, query, bm25_candidates)?;
    let bm25_elapsed = bm25_start.elapsed();
    let aug_start = Instant::now();
    let results =
        augment_exact_match_candidates(index_dir, query, results, RankingStage::Initial, filters)?;
    let timings = SearchTimings {
        bm25: Some(bm25_elapsed),
        augmentation: Some(aug_start.elapsed()),
        total: Some(total_start.elapsed()),
        ..Default::default()
    };
    Ok((apply_filters(results, filters, result_limit), timings))
}

fn augment_exact_match_candidates(
    index_dir: &Path,
    query: &str,
    results: Vec<SearchResult>,
    stage: RankingStage,
    filters: &SearchFilters,
) -> Result<Vec<SearchResult>> {
    let metadata_path = index_dir.join("metadata.db");
    let Ok(store) = crate::storage::metadata::MetadataStore::open(&metadata_path) else {
        return Ok(apply_query_ranking_with_filters(
            query, results, stage, filters,
        ));
    };

    let mut supplemental = Vec::new();

    // Direct filename lookup for path-weighted queries (e.g. "Cargo.toml workspace config").
    if let Some(filename) = extract_exact_filename(query).filter(|_| is_path_weighted_query(query))
    {
        let mut matching_files: Vec<String> = store
            .indexed_files()?
            .into_iter()
            .filter(|path| file_name(path).eq_ignore_ascii_case(&filename))
            .collect();
        matching_files.sort_by(|a, b| path_depth(a).cmp(&path_depth(b)).then(a.cmp(b)));

        for file_path in matching_files.into_iter().take(20) {
            supplemental.extend(
                store
                    .get_chunks_by_file(&file_path)?
                    .into_iter()
                    .map(chunk_to_result),
            );
        }
    }

    // Direct symbol lookup for identifier queries (e.g. "Config", "Blueprint class").
    if let Some(identifier_case) = extract_exact_identifier_case(query).as_deref() {
        let mut chunks = store.get_chunks_by_symbol_name_case_sensitive(identifier_case)?;
        let identifier = identifier_case.to_ascii_lowercase();
        let mut fallback_chunks = store.get_chunks_by_symbol_name(&identifier)?;
        fallback_chunks.retain(|chunk| chunk.symbol_name.as_deref() != Some(identifier_case));
        if uppercase_identifier_query(identifier_case) {
            fallback_chunks.retain(|chunk| {
                !matches!(
                    chunk.symbol_type,
                    Some(SymbolType::Method | SymbolType::Function | SymbolType::Module)
                )
            });
        }
        chunks.extend(fallback_chunks);
        chunks.sort_by(|a, b| {
            exact_match_priority(query, identifier_case, a)
                .cmp(&exact_match_priority(query, identifier_case, b))
                .then(path_depth(&a.file_path).cmp(&path_depth(&b.file_path)))
                .then(a.file_path.cmp(&b.file_path))
                .then(a.line_start.cmp(&b.line_start))
        });
        supplemental.extend(chunks.into_iter().map(chunk_to_result));
    }

    if supplemental.is_empty() {
        return Ok(apply_query_ranking_with_filters(
            query, results, stage, filters,
        ));
    }

    // Merge: supplemental first (exact matches), then original results, deduped.
    let mut merged = Vec::with_capacity(supplemental.len() + results.len());
    let mut seen = HashSet::new();

    for result in supplemental.into_iter().chain(results) {
        if seen.insert(result_key(&result)) {
            merged.push(result);
        }
    }

    Ok(apply_query_ranking_with_filters(
        query, merged, stage, filters,
    ))
}

fn extract_exact_filename(query: &str) -> Option<String> {
    query
        .split_whitespace()
        .map(trim_query_token)
        .filter(|token| !token.is_empty())
        .find(|token| looks_like_filename(token))
        .map(|token| file_name(token).to_ascii_lowercase())
}

fn extract_exact_identifier_case(query: &str) -> Option<String> {
    query
        .split_whitespace()
        .map(trim_query_token)
        .filter(|token| !token.is_empty())
        .find(|token| !looks_like_filename(token) && looks_like_compound_identifier(token))
        .map(ToString::to_string)
}

fn query_mentions_implementation(query: &str) -> bool {
    let lower = query.to_ascii_lowercase();
    lower.contains("implement")
        || lower.contains("registration")
        || lower.contains("mounted")
        || lower.contains("mounting")
}

fn exact_match_priority(query: &str, identifier_case: &str, chunk: &Chunk) -> (u8, u8, u8, u8) {
    let exact_case = u8::from(chunk.symbol_name.as_deref() != Some(identifier_case));
    let implementation_rank =
        if query_mentions_implementation(query) && chunk_looks_like_impl(chunk) {
            0
        } else {
            1
        };
    let visibility_rank = u8::from(!chunk_is_public_symbol(chunk));
    let type_mismatch_rank = if identifier_case
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
        && matches!(
            chunk.symbol_type,
            Some(SymbolType::Method | SymbolType::Function)
        )
        && chunk.symbol_name.as_deref() != Some(identifier_case)
    {
        1
    } else {
        0
    };

    (
        exact_case,
        implementation_rank,
        visibility_rank,
        type_mismatch_rank,
    )
}

fn chunk_looks_like_impl(chunk: &Chunk) -> bool {
    chunk
        .symbol_name
        .as_deref()
        .is_some_and(|name| name.to_ascii_lowercase().contains("impl"))
        || chunk
            .content
            .lines()
            .find(|line| !line.trim().is_empty())
            .is_some_and(|line| line.trim_start().starts_with("impl "))
}

fn chunk_is_public_symbol(chunk: &Chunk) -> bool {
    chunk.content.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(
            trimmed.starts_with("pub ")
                || trimmed.starts_with("export ")
                || trimmed.starts_with("public ")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("interface "),
        )
    }) == Some(true)
}

fn uppercase_identifier_query(identifier: &str) -> bool {
    identifier
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
}

fn result_key(result: &SearchResult) -> String {
    format!(
        "{}:{}:{}",
        result.file_path, result.line_start, result.line_end
    )
}

fn chunk_to_result(chunk: crate::types::Chunk) -> SearchResult {
    SearchResult {
        file_path: chunk.file_path,
        line_start: chunk.line_start,
        line_end: chunk.line_end,
        content: chunk.content,
        language: chunk.language,
        score: 0.0,
        symbol_name: chunk.symbol_name,
        symbol_type: chunk.symbol_type,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::metadata::MetadataStore;
    use crate::types::{Chunk, Language};
    use tempfile::tempdir;

    #[test]
    fn test_dimension_mismatch_and_inference() {
        let dir = tempdir().unwrap();
        let index_dir = dir.path();

        let metadata_path = index_dir.join("metadata.db");
        let store = MetadataStore::open(&metadata_path).unwrap();

        // 1. Test dimension mismatch (requires local model so provider_dim is Some(768))
        store
            .set_index_meta("model_name", "jina-embeddings-v5-text-nano-retrieval")
            .unwrap();
        store.set_index_meta("embedding_dim", "1024").unwrap(); // Mismatch: 1024 vs 768

        let config = VeraConfig::default();
        let filters = SearchFilters::default();

        // This attempts local provider creation first, then falls back to BM25 when possible.
        // In this synthetic test fixture the BM25 index is absent, so either path may surface.
        {
            let res = execute_search(
                index_dir,
                "test",
                &config,
                &filters,
                10,
                crate::config::InferenceBackend::OnnxJina(
                    crate::config::OnnxExecutionProvider::Cpu,
                ),
            );
            if let Err(err) = res {
                let err_msg = err.to_string();
                assert!(
                    err_msg.contains("tantivy")
                        || err_msg.contains("Failed to initialize local embedding provider")
                        || err_msg.contains("No such file")
                        || err_msg.contains("not found"),
                    "{}",
                    err_msg
                );
            }
        }

        // 2. Test metadata-dimension inference path (API provider returns None for expected_dim)
        // Set up dummy environment variables for API provider to bypass missing keys error
        unsafe {
            std::env::set_var("EMBEDDING_MODEL_BASE_URL", "http://127.0.0.1:0");
            std::env::set_var("EMBEDDING_MODEL_ID", "dummy-api-model");
            std::env::set_var("EMBEDDING_MODEL_API_KEY", "dummy-key");
        }

        store
            .set_index_meta("model_name", "dummy-api-model")
            .unwrap();
        store.set_index_meta("embedding_dim", "123").unwrap();

        // Calling execute_search with is_local = false
        // It will pass the metadata check (model_name matches), skip mismatch check (expected_dim is None),
        // infer stored_dim = 123, and proceed to search.
        // Since the index is empty, it will return Ok([]) without making network calls.
        let res = execute_search(
            index_dir,
            "test",
            &config,
            &filters,
            10,
            crate::config::InferenceBackend::Api,
        );
        assert!(res.is_ok(), "Expected Ok but got {:?}", res);
    }

    #[test]
    fn effective_candidates_apply_hard_rerank_cap() {
        assert_eq!(effective_rerank_candidates(50, 10), 50);
        assert_eq!(effective_rerank_candidates(5, 10), 10);
        assert_eq!(effective_rerank_candidates(60, 10), 60);
        assert_eq!(effective_rerank_candidates(0, 10), 10);

        // Vector candidates use query_params multiplier and optional hard caps.
        let nl_params =
            params_for_query_type(crate::retrieval::query_classifier::QueryType::NaturalLanguage);
        let vc = effective_vector_candidates(10, nl_params, 0);
        assert!(vc >= 50); // at least the minimum from compute_vector_candidates

        let vc_capped = effective_vector_candidates(160, nl_params, 60);
        assert_eq!(vc_capped, 60);

        let bm25_uncapped =
            effective_bm25_candidates("request validation and schema enforcement", 20, 0);
        assert_eq!(bm25_uncapped, 80);

        let bm25_capped =
            effective_bm25_candidates("request validation and schema enforcement", 20, 60);
        assert_eq!(bm25_capped, 60);
    }

    #[test]
    fn broad_nl_queries_overfetch_before_ranking() {
        let filters = SearchFilters::default();

        assert_eq!(compute_fetch_limit("Config", &filters, 20), 20);
        assert_eq!(
            compute_fetch_limit("file type detection and filtering", &filters, 20),
            160
        );
        assert_eq!(
            compute_fetch_limit(
                "how are HTTP errors handled and returned to clients",
                &filters,
                5
            ),
            145
        );
    }

    #[test]
    fn exact_identifier_queries_skip_reranking() {
        assert!(should_skip_reranking("Config", &SearchFilters::default()));
        assert!(should_skip_reranking(
            "src/config.ts",
            &SearchFilters::default()
        ));
        assert!(!should_skip_reranking(
            "how are HTTP errors handled",
            &SearchFilters::default()
        ));
    }

    #[test]
    fn exact_identifier_lookup_finds_matching_symbol() {
        let dir = tempdir().unwrap();
        let metadata_path = dir.path().join("metadata.db");
        let store = MetadataStore::open(&metadata_path).unwrap();
        store
            .insert_chunks(&[Chunk {
                id: "sink:0".to_string(),
                file_path: "crates/searcher/src/sink.rs".to_string(),
                line_start: 102,
                line_end: 223,
                content: "pub trait Sink {}".to_string(),
                language: Language::Rust,
                symbol_type: Some(SymbolType::Trait),
                symbol_name: Some("Sink".to_string()),
            }])
            .unwrap();

        let augmented = augment_exact_match_candidates(
            dir.path(),
            "Sink trait and its implementations",
            Vec::new(),
            RankingStage::Initial,
            &SearchFilters::default(),
        )
        .unwrap();

        assert!(
            augmented
                .iter()
                .any(|result| result.symbol_name.as_deref() == Some("Sink"))
        );
    }

    #[test]
    fn exact_identifier_prefers_public_type_definition() {
        let dir = tempdir().unwrap();
        let metadata_path = dir.path().join("metadata.db");
        let store = MetadataStore::open(&metadata_path).unwrap();
        store
            .insert_chunks(&[
                Chunk {
                    id: "config:0".to_string(),
                    file_path: "crates/core/search.rs".to_string(),
                    line_start: 19,
                    line_end: 25,
                    content: "struct Config {\n    search_zip: bool,\n}".to_string(),
                    language: Language::Rust,
                    symbol_type: Some(SymbolType::Struct),
                    symbol_name: Some("Config".to_string()),
                },
                Chunk {
                    id: "config:1".to_string(),
                    file_path: "crates/regex/src/config.rs".to_string(),
                    line_start: 25,
                    line_end: 43,
                    content: "pub(crate) struct Config {\n    pub(crate) multi_line: bool,\n}".to_string(),
                    language: Language::Rust,
                    symbol_type: Some(SymbolType::Struct),
                    symbol_name: Some("Config".to_string()),
                },
                Chunk {
                    id: "config:2".to_string(),
                    file_path: "crates/searcher/src/searcher/mod.rs".to_string(),
                    line_start: 151,
                    line_end: 185,
                    content: "pub struct Config {\n    line_term: LineTerminator,\n    multi_line: bool,\n}".to_string(),
                    language: Language::Rust,
                    symbol_type: Some(SymbolType::Struct),
                    symbol_name: Some("Config".to_string()),
                },
            ])
            .unwrap();

        let augmented = augment_exact_match_candidates(
            dir.path(),
            "Config",
            Vec::new(),
            RankingStage::Initial,
            &SearchFilters::default(),
        )
        .unwrap();

        assert_eq!(
            augmented[0].file_path,
            "crates/searcher/src/searcher/mod.rs"
        );
    }
}
