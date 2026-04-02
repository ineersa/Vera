//! `vera config` -- Show or set configuration values.

use anyhow::{Context, bail};

use crate::helpers::load_runtime_config;
use crate::state;

/// Run the `vera config` command.
pub fn run(args: &[String], json_output: bool) -> anyhow::Result<()> {
    let mut config = load_runtime_config()?;

    match args.first().map(|s| s.as_str()) {
        None | Some("show") => {
            if json_output {
                print_json_config(&config)?;
            } else {
                print_human_config(&config);
                print_stored_sections()?;
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

            let value = if let Some(value) = get_config_value(&config, key) {
                Some(value)
            } else {
                get_stored_config_value(key)?
            };

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
                    let handled = if set_config_value(&mut config, key, value)? {
                        state::save_runtime_config(&config)?;
                        true
                    } else {
                        set_stored_config_value(key, value)?
                    };

                    if !handled {
                        bail!(
                            "unknown configuration key: {key}\n\
                             Hint: run `vera config show` to see all available keys."
                        );
                    }

                    // Ensure in-process env reflects any saved provider settings.
                    state::apply_saved_env_force()?;

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

fn print_json_config(config: &vera_core::config::VeraConfig) -> anyhow::Result<()> {
    let stored = state::load_saved_config()?;
    let secrets = state::load_saved_secrets()?;

    let json = serde_json::json!({
        "indexing": config.indexing,
        "retrieval": config.retrieval,
        "embedding": config.embedding,
        "embedding_api": stored.embedding_api,
        "reranker_api": stored.reranker_api,
        "completion_api": stored.completion_api,
        "credentials": {
            "embedding_api_key_set": secrets.embedding_api_key.as_deref().is_some_and(|v| !v.is_empty()),
            "reranker_api_key_set": secrets.reranker_api_key.as_deref().is_some_and(|v| !v.is_empty()),
            "completion_api_key_set": secrets.completion_api_key.as_deref().is_some_and(|v| !v.is_empty())
        }
    });
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

/// Print human-readable runtime configuration.
fn print_human_config(config: &vera_core::config::VeraConfig) {
    println!("Vera Configuration");
    println!();
    println!("  Indexing:");
    println!(
        "    max_chunk_lines                 {}",
        config.indexing.max_chunk_lines
    );
    println!(
        "    max_file_size_bytes             {}",
        config.indexing.max_file_size_bytes
    );
    println!(
        "    max_chunk_bytes                 {}",
        config.indexing.max_chunk_bytes
    );
    println!(
        "    max_chunk_tokens                {}",
        config.indexing.max_chunk_tokens
    );
    println!(
        "    chunk_overlap_lines             {}",
        config.indexing.chunk_overlap_lines
    );
    println!(
        "    default_excludes                {:?}",
        config.indexing.default_excludes
    );
    println!();
    println!("  Retrieval:");
    println!(
        "    default_limit                   {}",
        config.retrieval.default_limit
    );
    println!(
        "    max_output_chars                {}",
        config.retrieval.max_output_chars
    );
    println!(
        "    rrf_k                           {}",
        config.retrieval.rrf_k
    );
    println!(
        "    rerank_candidates               {}",
        config.retrieval.rerank_candidates
    );
    println!(
        "    reranking_enabled               {}",
        config.retrieval.reranking_enabled
    );
    println!(
        "    max_rerank_batch                {}",
        config.retrieval.max_rerank_batch
    );
    println!(
        "    reranker_max_docs_per_request   {}",
        config.retrieval.reranker_max_docs_per_request
    );
    println!(
        "    reranker_max_document_tokens    {}",
        config.retrieval.reranker_max_document_tokens
    );
    println!();
    println!("  Embedding:");
    println!(
        "    batch_size                      {}",
        config.embedding.batch_size
    );
    println!(
        "    max_concurrent_requests         {}",
        config.embedding.max_concurrent_requests
    );
    println!(
        "    timeout_secs                    {}",
        config.embedding.timeout_secs
    );
    println!(
        "    max_retries                     {}",
        config.embedding.max_retries
    );
    println!(
        "    max_stored_dim                  {}",
        config.embedding.max_stored_dim
    );
}

fn print_stored_sections() -> anyhow::Result<()> {
    let stored = state::load_saved_config()?;
    let secrets = state::load_saved_secrets()?;

    println!();
    println!("  Embedding API:");
    println!(
        "    base_url                        {}",
        stored
            .embedding_api
            .as_ref()
            .map(|v| v.base_url.as_str())
            .unwrap_or("<unset>")
    );
    println!(
        "    model_id                        {}",
        stored
            .embedding_api
            .as_ref()
            .map(|v| v.model_id.as_str())
            .unwrap_or("<unset>")
    );
    println!(
        "    api_key                         {}",
        secret_status(secrets.embedding_api_key.as_deref())
    );

    println!();
    println!("  Reranker API:");
    println!(
        "    base_url                        {}",
        stored
            .reranker_api
            .as_ref()
            .map(|v| v.base_url.as_str())
            .unwrap_or("<unset>")
    );
    println!(
        "    model_id                        {}",
        stored
            .reranker_api
            .as_ref()
            .map(|v| v.model_id.as_str())
            .unwrap_or("<unset>")
    );
    println!(
        "    api_key                         {}",
        secret_status(secrets.reranker_api_key.as_deref())
    );

    println!();
    println!("  Completion API:");
    println!(
        "    base_url                        {}",
        stored
            .completion_api
            .as_ref()
            .map(|v| v.base_url.as_str())
            .unwrap_or("<unset>")
    );
    println!(
        "    model_id                        {}",
        stored
            .completion_api
            .as_ref()
            .map(|v| v.model_id.as_str())
            .unwrap_or("<unset>")
    );
    println!(
        "    timeout_secs                    {}",
        stored
            .completion_api
            .as_ref()
            .map(|v| v.timeout_secs.to_string())
            .unwrap_or_else(|| "<unset>".to_string())
    );
    println!(
        "    max_tokens                      {}",
        stored
            .completion_api
            .as_ref()
            .map(|v| v.max_tokens.to_string())
            .unwrap_or_else(|| "<unset>".to_string())
    );
    println!(
        "    max_alternatives                {}",
        stored
            .completion_api
            .as_ref()
            .map(|v| v.max_alternatives.to_string())
            .unwrap_or_else(|| "<unset>".to_string())
    );

    let completion_key = stored
        .completion_api
        .as_ref()
        .and_then(|value| value.api_key.as_deref())
        .or(secrets.completion_api_key.as_deref());
    println!(
        "    api_key                         {}",
        secret_status(completion_key)
    );

    Ok(())
}

fn secret_status(value: Option<&str>) -> &'static str {
    if value.is_some_and(|v| !v.trim().is_empty()) {
        "<set>"
    } else {
        "<unset>"
    }
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
        "indexing.max_chunk_tokens" => Some(serde_json::Value::Number(
            config.indexing.max_chunk_tokens.into(),
        )),
        "indexing.chunk_overlap_lines" => Some(serde_json::Value::Number(
            config.indexing.chunk_overlap_lines.into(),
        )),
        "indexing.default_excludes" => serde_json::to_value(&config.indexing.default_excludes).ok(),
        "retrieval.default_limit" => Some(serde_json::Value::Number(
            config.retrieval.default_limit.into(),
        )),
        "retrieval.rrf_k" => serde_json::to_value(config.retrieval.rrf_k).ok(),
        "retrieval.rerank_candidates" => Some(serde_json::Value::Number(
            config.retrieval.rerank_candidates.into(),
        )),
        "retrieval.reranking_enabled" => {
            Some(serde_json::Value::Bool(config.retrieval.reranking_enabled))
        }
        "retrieval.max_output_chars" => Some(serde_json::Value::Number(
            config.retrieval.max_output_chars.into(),
        )),
        "retrieval.max_rerank_batch" => Some(serde_json::Value::Number(
            config.retrieval.max_rerank_batch.into(),
        )),
        "retrieval.reranker_max_docs_per_request" => Some(serde_json::Value::Number(
            config.retrieval.reranker_max_docs_per_request.into(),
        )),
        "retrieval.reranker_max_document_tokens" => Some(serde_json::Value::Number(
            config.retrieval.reranker_max_document_tokens.into(),
        )),
        "reranker.max_docs_per_request" => {
            let value = if config.retrieval.reranker_max_docs_per_request > 0 {
                config.retrieval.reranker_max_docs_per_request
            } else {
                config.retrieval.max_rerank_batch
            };
            Some(serde_json::Value::Number(value.into()))
        }
        "reranker.max_document_tokens" => Some(serde_json::Value::Number(
            config.retrieval.reranker_max_document_tokens.into(),
        )),
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

fn get_stored_config_value(key: &str) -> anyhow::Result<Option<serde_json::Value>> {
    let stored = state::load_saved_config()?;
    let secrets = state::load_saved_secrets()?;

    let value = match key {
        "embedding_api.base_url" => Some(optional_string(
            stored.embedding_api.as_ref().map(|v| v.base_url.as_str()),
        )),
        "embedding_api.model_id" => Some(optional_string(
            stored.embedding_api.as_ref().map(|v| v.model_id.as_str()),
        )),
        "embedding_api.api_key" => Some(optional_string(secrets.embedding_api_key.as_deref())),
        "reranker_api.base_url" => Some(optional_string(
            stored.reranker_api.as_ref().map(|v| v.base_url.as_str()),
        )),
        "reranker_api.model_id" => Some(optional_string(
            stored.reranker_api.as_ref().map(|v| v.model_id.as_str()),
        )),
        "reranker_api.api_key" => Some(optional_string(secrets.reranker_api_key.as_deref())),
        "completion_api.base_url" | "completion.base_url" => Some(optional_string(
            stored.completion_api.as_ref().map(|v| v.base_url.as_str()),
        )),
        "completion_api.model_id" | "completion.model_id" => Some(optional_string(
            stored.completion_api.as_ref().map(|v| v.model_id.as_str()),
        )),
        "completion_api.api_key" | "completion.api_key" => {
            let key = stored
                .completion_api
                .as_ref()
                .and_then(|value| value.api_key.as_deref())
                .or(secrets.completion_api_key.as_deref());
            Some(optional_string(key))
        }
        "completion_api.timeout_secs" | "completion.timeout_secs" => Some(optional_u64(
            stored.completion_api.as_ref().map(|v| v.timeout_secs),
        )),
        "completion_api.max_tokens" | "completion.max_tokens" => Some(optional_u32(
            stored.completion_api.as_ref().map(|v| v.max_tokens),
        )),
        "completion_api.max_alternatives" | "completion.max_alternatives" => Some(optional_usize(
            stored.completion_api.as_ref().map(|v| v.max_alternatives),
        )),
        _ => None,
    };

    Ok(value)
}

fn optional_string(value: Option<&str>) -> serde_json::Value {
    match value {
        Some(value) => serde_json::Value::String(value.to_string()),
        None => serde_json::Value::Null,
    }
}

fn optional_u64(value: Option<u64>) -> serde_json::Value {
    value
        .map(|v| serde_json::Value::Number(v.into()))
        .unwrap_or(serde_json::Value::Null)
}

fn optional_u32(value: Option<u32>) -> serde_json::Value {
    value
        .map(|v| serde_json::Value::Number(v.into()))
        .unwrap_or(serde_json::Value::Null)
}

fn optional_usize(value: Option<usize>) -> serde_json::Value {
    value
        .and_then(|v| u64::try_from(v).ok())
        .map(|v| serde_json::Value::Number(v.into()))
        .unwrap_or(serde_json::Value::Null)
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
        "indexing.max_chunk_tokens" => {
            config.indexing.max_chunk_tokens = parse_value(key, value)?;
        }
        "indexing.chunk_overlap_lines" => {
            config.indexing.chunk_overlap_lines = parse_value(key, value)?;
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
        "retrieval.reranking_enabled" => {
            config.retrieval.reranking_enabled = parse_value(key, value)?;
        }
        "retrieval.max_output_chars" => {
            config.retrieval.max_output_chars = parse_value(key, value)?;
        }
        "retrieval.max_rerank_batch" => {
            config.retrieval.max_rerank_batch = parse_value(key, value)?;
        }
        "retrieval.reranker_max_docs_per_request" | "reranker.max_docs_per_request" => {
            config.retrieval.reranker_max_docs_per_request = parse_value(key, value)?;
        }
        "retrieval.reranker_max_document_tokens" | "reranker.max_document_tokens" => {
            config.retrieval.reranker_max_document_tokens = parse_value(key, value)?;
        }
        "reranker.max_document_chars" => {
            let max_chars: usize = parse_value(key, value)?;
            config.retrieval.reranker_max_document_tokens = if max_chars == 0 {
                0
            } else {
                max_chars.div_ceil(4).max(16)
            };
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

fn set_stored_config_value(key: &str, value: &str) -> anyhow::Result<bool> {
    let mut stored = state::load_saved_config()?;
    let mut secrets = state::load_saved_secrets()?;
    let mut config_changed = false;
    let mut secrets_changed = false;

    match key {
        "embedding_api.base_url" => {
            let endpoint = stored
                .embedding_api
                .get_or_insert_with(default_api_endpoint_config);
            endpoint.base_url = value.to_string();
            config_changed = true;
        }
        "embedding_api.model_id" => {
            let endpoint = stored
                .embedding_api
                .get_or_insert_with(default_api_endpoint_config);
            endpoint.model_id = value.to_string();
            config_changed = true;
        }
        "embedding_api.api_key" => {
            secrets.embedding_api_key = Some(value.to_string());
            secrets_changed = true;
        }
        "reranker_api.base_url" => {
            let endpoint = stored
                .reranker_api
                .get_or_insert_with(default_api_endpoint_config);
            endpoint.base_url = value.to_string();
            config_changed = true;
        }
        "reranker_api.model_id" => {
            let endpoint = stored
                .reranker_api
                .get_or_insert_with(default_api_endpoint_config);
            endpoint.model_id = value.to_string();
            config_changed = true;
        }
        "reranker_api.api_key" => {
            secrets.reranker_api_key = Some(value.to_string());
            secrets_changed = true;
        }
        "completion_api.base_url" | "completion.base_url" => {
            let completion = stored
                .completion_api
                .get_or_insert_with(default_completion_api_config);
            completion.base_url = value.to_string();
            config_changed = true;
        }
        "completion_api.model_id" | "completion.model_id" => {
            let completion = stored
                .completion_api
                .get_or_insert_with(default_completion_api_config);
            completion.model_id = value.to_string();
            config_changed = true;
        }
        "completion_api.api_key" | "completion.api_key" => {
            secrets.completion_api_key = Some(value.to_string());
            secrets_changed = true;
        }
        "completion_api.timeout_secs" | "completion.timeout_secs" => {
            let timeout: u64 = parse_value(key, value)?;
            if timeout == 0 {
                bail!("{key} must be greater than 0");
            }
            let completion = stored
                .completion_api
                .get_or_insert_with(default_completion_api_config);
            completion.timeout_secs = timeout;
            config_changed = true;
        }
        "completion_api.max_tokens" | "completion.max_tokens" => {
            let completion = stored
                .completion_api
                .get_or_insert_with(default_completion_api_config);
            completion.max_tokens = parse_value(key, value)?;
            config_changed = true;
        }
        "completion_api.max_alternatives" | "completion.max_alternatives" => {
            let completion = stored
                .completion_api
                .get_or_insert_with(default_completion_api_config);
            completion.max_alternatives = parse_value(key, value)?;
            config_changed = true;
        }
        _ => return Ok(false),
    }

    if config_changed {
        state::save_saved_config(&stored)?;
    }
    if secrets_changed {
        state::save_saved_secrets(&secrets)?;
    }

    Ok(true)
}

fn default_api_endpoint_config() -> state::ApiEndpointConfig {
    state::ApiEndpointConfig {
        base_url: String::new(),
        model_id: String::new(),
    }
}

fn default_completion_api_config() -> state::CompletionApiConfig {
    state::CompletionApiConfig {
        base_url: String::new(),
        model_id: String::new(),
        api_key: None,
        timeout_secs: 120,
        max_tokens: 16_384,
        max_alternatives: 4,
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
