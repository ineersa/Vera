//! `vera config` — Show or set configuration values.

use anyhow::{Context, bail};
use serde::Serialize;

use crate::helpers::load_runtime_config;
use crate::state;

const SECRET_SET_MARKER: &str = "[set]";

#[derive(Debug, Clone, Serialize)]
struct EffectiveApiConfig {
    embedding: EffectiveEmbeddingConfig,
    reranker: EffectiveEndpointConfig,
    completion: EffectiveEndpointConfig,
}

#[derive(Debug, Clone, Serialize)]
struct EffectiveEmbeddingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    query_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct EffectiveEndpointConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key: Option<String>,
}

/// Run the `vera config` command.
pub fn run(args: &[String], json_output: bool) -> anyhow::Result<()> {
    let mut config = load_runtime_config()?;
    let mut stored = state::load_saved_config()?;
    let mut secrets = state::load_saved_secrets()?;

    match args.first().map(|s| s.as_str()) {
        None | Some("show") => {
            let api = effective_api_config(&stored, &secrets);

            // Show full configuration.
            if json_output {
                print_json_config(&config, &api)?;
            } else {
                print_human_config(&config, &api);
            }
        }
        Some("get") => {
            let key = match args.get(1) {
                Some(k) => k,
                None => bail!(
                    "missing key for `vera config get`.\n\
                     Hint: use `vera config get <key>`, \
                     e.g., `vera config get retrieval.default_limit`"
                ),
            };
            let api = effective_api_config(&stored, &secrets);
            let value = get_config_value(&config, key)
                .or_else(|| get_external_config_value(&config, &api, key));
            match value {
                Some(v) => {
                    if json_output {
                        println!("{v}");
                    } else {
                        println!("{key} = {v}");
                    }
                }
                None => bail!(
                    "unknown configuration key: {key}\n\
                     Hint: run `vera config show` to see all available keys."
                ),
            }
        }
        Some("set") => {
            let key = args.get(1);
            let value = args.get(2);
            match (key, value) {
                (Some(key), Some(value)) => {
                    if set_config_value(&mut config, key, value)? {
                        state::save_runtime_config(&config)?;
                    } else if set_external_config_value(&mut stored, &mut secrets, key, value) {
                        state::persist_saved_config(&stored)?;
                        state::persist_saved_secrets(&secrets)?;
                    } else {
                        bail!(
                            "unknown configuration key: {key}\n\
                             Hint: run `vera config show` to see all available keys."
                        );
                    }

                    if json_output {
                        let result = serde_json::json!({
                            "key": key,
                            "value": value,
                            "status": "saved"
                        });
                        println!("{}", serde_json::to_string_pretty(&result).unwrap());
                    } else {
                        println!("Saved: {key} = {value}");
                    }
                }
                _ => bail!(
                    "missing key or value for `vera config set`.\n\
                     Hint: use `vera config set <key> <value>`, \
                     e.g., `vera config set retrieval.default_limit 20`"
                ),
            }
        }
        Some(unknown) => bail!(
            "unknown config subcommand: {unknown}\n\
             Hint: valid subcommands are: show, get, set.\n\
             Run `vera config --help` for details."
        ),
    }

    Ok(())
}

fn print_json_config(
    config: &vera_core::config::VeraConfig,
    api: &EffectiveApiConfig,
) -> anyhow::Result<()> {
    let mut json = serde_json::to_value(config)
        .map_err(|e| anyhow::anyhow!("failed to serialize config: {e}"))?;

    let object = json
        .as_object_mut()
        .context("failed to serialize config as object")?;
    object.insert(
        "api".to_string(),
        serde_json::to_value(api)
            .map_err(|e| anyhow::anyhow!("failed to serialize API config: {e}"))?,
    );

    let rendered = serde_json::to_string_pretty(&json)
        .map_err(|e| anyhow::anyhow!("failed to render config JSON: {e}"))?;
    println!("{rendered}");
    Ok(())
}

/// Print human-readable configuration.
fn print_human_config(config: &vera_core::config::VeraConfig, api: &EffectiveApiConfig) {
    println!("Vera Configuration");
    println!();
    println!("  Indexing:");
    println!(
        "    max_chunk_lines           {}",
        config.indexing.max_chunk_lines
    );
    println!(
        "    max_file_size_bytes       {}",
        config.indexing.max_file_size_bytes
    );
    println!(
        "    max_chunk_bytes           {}",
        config.indexing.max_chunk_bytes
    );
    println!(
        "    max_chunk_overlap_bytes   {}",
        config.indexing.max_chunk_overlap_bytes
    );
    println!(
        "    default_excludes          {:?}",
        config.indexing.default_excludes
    );
    println!();
    println!("  Retrieval:");
    println!(
        "    default_limit             {}",
        config.retrieval.default_limit
    );
    println!(
        "    max_output_chars          {}",
        config.retrieval.max_output_chars
    );
    println!(
        "    fail_on_stage_error       {}",
        config.retrieval.fail_on_stage_error
    );
    println!("    rrf_k                     {}", config.retrieval.rrf_k);
    println!(
        "    rerank_candidates         {}",
        config.retrieval.rerank_candidates
    );
    println!(
        "    max_bm25_candidates       {}",
        config.retrieval.max_bm25_candidates
    );
    println!(
        "    max_vector_candidates     {}",
        config.retrieval.max_vector_candidates
    );
    println!(
        "    reranking_enabled         {}",
        config.retrieval.reranking_enabled
    );
    println!(
        "    max_rerank_batch          {}",
        config.retrieval.max_rerank_batch
    );
    println!(
        "    max_rerank_doc_chars      {}",
        config.retrieval.max_rerank_doc_chars
    );
    println!();
    println!("  Embedding:");
    println!(
        "    batch_size                {}",
        config.embedding.batch_size
    );
    println!(
        "    max_concurrent_requests   {}",
        config.embedding.max_concurrent_requests
    );
    println!(
        "    timeout_secs              {}",
        config.embedding.timeout_secs
    );
    println!(
        "    max_retries               {}",
        config.embedding.max_retries
    );
    println!(
        "    max_stored_dim            {}",
        config.embedding.max_stored_dim
    );
    println!();
    println!("  API:");
    println!(
        "    embedding.base_url        {}",
        display_optional(&api.embedding.base_url)
    );
    println!(
        "    embedding.model_id        {}",
        display_optional(&api.embedding.model_id)
    );
    println!(
        "    embedding.api_key         {}",
        display_optional(&api.embedding.api_key)
    );
    println!(
        "    embedding.query_prefix    {}",
        display_optional(&api.embedding.query_prefix)
    );
    println!(
        "    reranker.base_url         {}",
        display_optional(&api.reranker.base_url)
    );
    println!(
        "    reranker.model_id         {}",
        display_optional(&api.reranker.model_id)
    );
    println!(
        "    reranker.api_key          {}",
        display_optional(&api.reranker.api_key)
    );
    println!(
        "    reranker.max_docs_per_request {}",
        config.retrieval.max_rerank_batch
    );
    println!(
        "    completion.base_url       {}",
        display_optional(&api.completion.base_url)
    );
    println!(
        "    completion.model_id       {}",
        display_optional(&api.completion.model_id)
    );
    println!(
        "    completion.api_key        {}",
        display_optional(&api.completion.api_key)
    );
}

fn display_optional(value: &Option<String>) -> &str {
    value
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("<unset>")
}

/// Get a configuration value by dot-notation key.
pub fn get_config_value(
    config: &vera_core::config::VeraConfig,
    key: &str,
) -> Option<serde_json::Value> {
    match key {
        "indexing.max_chunk_lines" => Some(serde_json::Value::Number(
            config.indexing.max_chunk_lines.into(),
        )),
        "indexing.max_file_size_bytes" => Some(serde_json::Value::Number(
            config.indexing.max_file_size_bytes.into(),
        )),
        "indexing.max_chunk_bytes" => Some(serde_json::Value::Number(
            config.indexing.max_chunk_bytes.into(),
        )),
        "indexing.max_chunk_overlap_bytes" => Some(serde_json::Value::Number(
            config.indexing.max_chunk_overlap_bytes.into(),
        )),
        "indexing.default_excludes" => serde_json::to_value(&config.indexing.default_excludes).ok(),
        "retrieval.default_limit" => Some(serde_json::Value::Number(
            config.retrieval.default_limit.into(),
        )),
        "retrieval.rrf_k" => serde_json::to_value(config.retrieval.rrf_k).ok(),
        "retrieval.rerank_candidates" => Some(serde_json::Value::Number(
            config.retrieval.rerank_candidates.into(),
        )),
        "retrieval.max_bm25_candidates" => Some(serde_json::Value::Number(
            config.retrieval.max_bm25_candidates.into(),
        )),
        "retrieval.max_vector_candidates" => Some(serde_json::Value::Number(
            config.retrieval.max_vector_candidates.into(),
        )),
        "retrieval.reranking_enabled" => {
            Some(serde_json::Value::Bool(config.retrieval.reranking_enabled))
        }
        "retrieval.max_rerank_batch" => Some(serde_json::Value::Number(
            config.retrieval.max_rerank_batch.into(),
        )),
        "api.reranker.max_docs_per_request"
        | "RERANKER_MAX_DOCS_PER_REQUEST"
        | "VERA_MAX_RERANK_BATCH" => Some(serde_json::Value::Number(
            config.retrieval.max_rerank_batch.into(),
        )),
        "retrieval.max_rerank_doc_chars" => Some(serde_json::Value::Number(
            config.retrieval.max_rerank_doc_chars.into(),
        )),
        "VERA_MAX_RERANK_DOC_CHARS" => Some(serde_json::Value::Number(
            config.retrieval.max_rerank_doc_chars.into(),
        )),
        "retrieval.max_output_chars" => Some(serde_json::Value::Number(
            config.retrieval.max_output_chars.into(),
        )),
        "retrieval.fail_on_stage_error" | "VERA_FAIL_ON_STAGE_ERROR" => Some(
            serde_json::Value::Bool(config.retrieval.fail_on_stage_error),
        ),
        "embedding.batch_size" => Some(serde_json::Value::Number(
            config.embedding.batch_size.into(),
        )),
        "embedding.max_concurrent_requests" => Some(serde_json::Value::Number(
            config.embedding.max_concurrent_requests.into(),
        )),
        "embedding.timeout_secs" => Some(serde_json::Value::Number(
            config.embedding.timeout_secs.into(),
        )),
        "embedding.max_retries" => Some(serde_json::Value::Number(
            config.embedding.max_retries.into(),
        )),
        "embedding.max_stored_dim" => Some(serde_json::Value::Number(
            config.embedding.max_stored_dim.into(),
        )),
        _ => None,
    }
}

fn get_external_config_value(
    config: &vera_core::config::VeraConfig,
    api: &EffectiveApiConfig,
    key: &str,
) -> Option<serde_json::Value> {
    match key {
        "api.embedding.base_url" | "EMBEDDING_MODEL_BASE_URL" => {
            serde_json::to_value(&api.embedding.base_url).ok()
        }
        "api.embedding.model_id" | "EMBEDDING_MODEL_ID" => {
            serde_json::to_value(&api.embedding.model_id).ok()
        }
        "api.embedding.api_key" | "EMBEDDING_MODEL_API_KEY" => {
            serde_json::to_value(&api.embedding.api_key).ok()
        }
        "api.embedding.query_prefix" | "EMBEDDING_QUERY_PREFIX" => {
            serde_json::to_value(&api.embedding.query_prefix).ok()
        }
        "api.reranker.base_url" | "RERANKER_MODEL_BASE_URL" => {
            serde_json::to_value(&api.reranker.base_url).ok()
        }
        "api.reranker.model_id" | "RERANKER_MODEL_ID" => {
            serde_json::to_value(&api.reranker.model_id).ok()
        }
        "api.reranker.api_key" | "RERANKER_MODEL_API_KEY" => {
            serde_json::to_value(&api.reranker.api_key).ok()
        }
        "api.reranker.max_docs_per_request"
        | "RERANKER_MAX_DOCS_PER_REQUEST"
        | "VERA_MAX_RERANK_BATCH" => Some(serde_json::Value::Number(
            config.retrieval.max_rerank_batch.into(),
        )),
        "api.completion.base_url" | "VERA_COMPLETION_BASE_URL" => {
            serde_json::to_value(&api.completion.base_url).ok()
        }
        "api.completion.model_id" | "VERA_COMPLETION_MODEL_ID" => {
            serde_json::to_value(&api.completion.model_id).ok()
        }
        "api.completion.api_key" | "VERA_COMPLETION_API_KEY" => {
            serde_json::to_value(&api.completion.api_key).ok()
        }
        _ => None,
    }
}

fn effective_api_config(
    stored: &state::StoredConfig,
    secrets: &state::StoredSecrets,
) -> EffectiveApiConfig {
    EffectiveApiConfig {
        embedding: EffectiveEmbeddingConfig {
            base_url: effective_value(
                "EMBEDDING_MODEL_BASE_URL",
                stored.embedding_api.as_ref().map(|c| c.base_url.as_str()),
            ),
            model_id: effective_value(
                "EMBEDDING_MODEL_ID",
                stored.embedding_api.as_ref().map(|c| c.model_id.as_str()),
            ),
            api_key: effective_secret_marker(
                "EMBEDDING_MODEL_API_KEY",
                secrets.embedding_api_key.as_deref(),
            ),
            query_prefix: effective_value(
                "EMBEDDING_QUERY_PREFIX",
                stored.embedding_query_prefix.as_deref(),
            ),
        },
        reranker: EffectiveEndpointConfig {
            base_url: effective_value(
                "RERANKER_MODEL_BASE_URL",
                stored.reranker_api.as_ref().map(|c| c.base_url.as_str()),
            ),
            model_id: effective_value(
                "RERANKER_MODEL_ID",
                stored.reranker_api.as_ref().map(|c| c.model_id.as_str()),
            ),
            api_key: effective_secret_marker(
                "RERANKER_MODEL_API_KEY",
                secrets.reranker_api_key.as_deref(),
            ),
        },
        completion: EffectiveEndpointConfig {
            base_url: effective_value(
                "VERA_COMPLETION_BASE_URL",
                stored.completion_api.as_ref().map(|c| c.base_url.as_str()),
            ),
            model_id: effective_value(
                "VERA_COMPLETION_MODEL_ID",
                stored.completion_api.as_ref().map(|c| c.model_id.as_str()),
            ),
            api_key: effective_secret_marker(
                "VERA_COMPLETION_API_KEY",
                secrets.completion_api_key.as_deref(),
            ),
        },
    }
}

fn effective_value(env_key: &str, fallback: Option<&str>) -> Option<String> {
    std::env::var(env_key)
        .ok()
        .and_then(|value| normalize_optional_value(&value))
        .or_else(|| fallback.and_then(normalize_optional_value))
}

fn effective_secret_marker(env_key: &str, fallback: Option<&str>) -> Option<String> {
    effective_value(env_key, fallback).map(|_| SECRET_SET_MARKER.to_string())
}

fn normalize_optional_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("none")
        || trimmed.eq_ignore_ascii_case("null")
        || trimmed.eq_ignore_ascii_case("unset")
    {
        None
    } else {
        Some(value.to_string())
    }
}

fn set_config_value(
    config: &mut vera_core::config::VeraConfig,
    key: &str,
    value: &str,
) -> anyhow::Result<bool> {
    match key {
        "indexing.max_chunk_lines" => {
            config.indexing.max_chunk_lines = parse_value(key, value)?;
        }
        "indexing.max_file_size_bytes" => {
            config.indexing.max_file_size_bytes = parse_value(key, value)?;
        }
        "indexing.max_chunk_bytes" => {
            config.indexing.max_chunk_bytes = parse_value(key, value)?;
        }
        "indexing.max_chunk_overlap_bytes" => {
            config.indexing.max_chunk_overlap_bytes = parse_value(key, value)?;
        }
        "indexing.default_excludes" => {
            config.indexing.default_excludes = serde_json::from_str(value).with_context(|| {
                format!("failed to parse {key} as JSON array of strings: {value}")
            })?;
        }
        "retrieval.default_limit" => {
            config.retrieval.default_limit = parse_value(key, value)?;
        }
        "retrieval.rrf_k" => {
            config.retrieval.rrf_k = parse_value(key, value)?;
        }
        "retrieval.rerank_candidates" => {
            config.retrieval.rerank_candidates = parse_value(key, value)?;
        }
        "retrieval.max_bm25_candidates" => {
            config.retrieval.max_bm25_candidates = parse_value(key, value)?;
        }
        "retrieval.max_vector_candidates" => {
            config.retrieval.max_vector_candidates = parse_value(key, value)?;
        }
        "retrieval.reranking_enabled" => {
            config.retrieval.reranking_enabled = parse_value(key, value)?;
        }
        "retrieval.max_rerank_batch" => {
            config.retrieval.max_rerank_batch = parse_value(key, value)?;
        }
        "api.reranker.max_docs_per_request"
        | "RERANKER_MAX_DOCS_PER_REQUEST"
        | "VERA_MAX_RERANK_BATCH" => {
            config.retrieval.max_rerank_batch = parse_value(key, value)?;
        }
        "retrieval.max_rerank_doc_chars" => {
            config.retrieval.max_rerank_doc_chars = parse_value(key, value)?;
        }
        "VERA_MAX_RERANK_DOC_CHARS" => {
            config.retrieval.max_rerank_doc_chars = parse_value(key, value)?;
        }
        "retrieval.max_output_chars" => {
            config.retrieval.max_output_chars = parse_value(key, value)?;
        }
        "retrieval.fail_on_stage_error" | "VERA_FAIL_ON_STAGE_ERROR" => {
            config.retrieval.fail_on_stage_error = parse_value(key, value)?;
        }
        "embedding.batch_size" => {
            config.embedding.batch_size = parse_value(key, value)?;
        }
        "embedding.max_concurrent_requests" => {
            config.embedding.max_concurrent_requests = parse_value(key, value)?;
        }
        "embedding.timeout_secs" => {
            config.embedding.timeout_secs = parse_value(key, value)?;
        }
        "embedding.max_retries" => {
            config.embedding.max_retries = parse_value(key, value)?;
        }
        "embedding.max_stored_dim" => {
            config.embedding.max_stored_dim = parse_value(key, value)?;
        }
        _ => return Ok(false),
    }

    Ok(true)
}

fn set_external_config_value(
    stored: &mut state::StoredConfig,
    secrets: &mut state::StoredSecrets,
    key: &str,
    value: &str,
) -> bool {
    match key {
        "api.embedding.base_url" | "EMBEDDING_MODEL_BASE_URL" => {
            set_endpoint_base_url(&mut stored.embedding_api, normalize_optional_value(value));
            true
        }
        "api.embedding.model_id" | "EMBEDDING_MODEL_ID" => {
            set_endpoint_model_id(&mut stored.embedding_api, normalize_optional_value(value));
            true
        }
        "api.embedding.api_key" | "EMBEDDING_MODEL_API_KEY" => {
            secrets.embedding_api_key = normalize_optional_value(value);
            true
        }
        "api.embedding.query_prefix" | "EMBEDDING_QUERY_PREFIX" => {
            stored.embedding_query_prefix = normalize_optional_value(value);
            true
        }
        "api.reranker.base_url" | "RERANKER_MODEL_BASE_URL" => {
            set_endpoint_base_url(&mut stored.reranker_api, normalize_optional_value(value));
            true
        }
        "api.reranker.model_id" | "RERANKER_MODEL_ID" => {
            set_endpoint_model_id(&mut stored.reranker_api, normalize_optional_value(value));
            true
        }
        "api.reranker.api_key" | "RERANKER_MODEL_API_KEY" => {
            secrets.reranker_api_key = normalize_optional_value(value);
            true
        }
        "api.completion.base_url" | "VERA_COMPLETION_BASE_URL" => {
            set_endpoint_base_url(&mut stored.completion_api, normalize_optional_value(value));
            true
        }
        "api.completion.model_id" | "VERA_COMPLETION_MODEL_ID" => {
            set_endpoint_model_id(&mut stored.completion_api, normalize_optional_value(value));
            true
        }
        "api.completion.api_key" | "VERA_COMPLETION_API_KEY" => {
            secrets.completion_api_key = normalize_optional_value(value);
            true
        }
        _ => false,
    }
}

fn set_endpoint_base_url(slot: &mut Option<state::ApiEndpointConfig>, value: Option<String>) {
    set_endpoint_field(slot, value, true);
}

fn set_endpoint_model_id(slot: &mut Option<state::ApiEndpointConfig>, value: Option<String>) {
    set_endpoint_field(slot, value, false);
}

fn set_endpoint_field(
    slot: &mut Option<state::ApiEndpointConfig>,
    value: Option<String>,
    set_base_url: bool,
) {
    match value {
        Some(value) => {
            let endpoint = slot.get_or_insert_with(state::ApiEndpointConfig::default);
            if set_base_url {
                endpoint.base_url = value;
            } else {
                endpoint.model_id = value;
            }
        }
        None => {
            if let Some(endpoint) = slot.as_mut() {
                if set_base_url {
                    endpoint.base_url.clear();
                } else {
                    endpoint.model_id.clear();
                }
                if endpoint.base_url.trim().is_empty() && endpoint.model_id.trim().is_empty() {
                    *slot = None;
                }
            }
        }
    }
}

fn parse_value<T>(key: &str, value: &str) -> anyhow::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    value
        .parse::<T>()
        .map_err(|e| anyhow::anyhow!("failed to parse {key}: {e}"))
}
