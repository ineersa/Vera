//! OpenAI-compatible completion client for query expansion.

use std::collections::HashSet;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use serde::Serialize;

const DEFAULT_TIMEOUT_SECS: u64 = 120;
const DEFAULT_MAX_ALTERNATIVES: usize = 4;
const MAX_ALLOWED_ALTERNATIVES: usize = 8;
const DEFAULT_MAX_TOKENS: u32 = 16_384;
const MIN_MAX_TOKENS: u32 = 128;
const MAX_MAX_TOKENS: u32 = 65_536;

/// OpenAI-compatible chat completion client config.
#[derive(Debug, Clone)]
pub struct CompletionClientConfig {
    /// Base URL for OpenAI-compatible API (for example `http://localhost:8080/v1`).
    pub base_url: String,
    /// Model identifier for query expansion.
    pub model_id: String,
    /// API key used for bearer auth. Local providers can ignore this value.
    pub api_key: String,
    /// Request timeout.
    pub timeout: Duration,
    /// Number of alternative queries to request.
    pub max_alternatives: usize,
    /// Token budget for completion output.
    ///
    /// Some reasoning models emit long `reasoning_content` before final output,
    /// so deep search needs a generous completion budget.
    pub max_tokens: u32,
}

impl CompletionClientConfig {
    /// Returns true when completion env vars are present.
    pub fn is_configured() -> bool {
        let has_base = std::env::var("VERA_COMPLETION_BASE_URL")
            .ok()
            .is_some_and(|value| !value.trim().is_empty());
        let has_model = std::env::var("VERA_COMPLETION_MODEL_ID")
            .ok()
            .is_some_and(|value| !value.trim().is_empty());
        has_base && has_model
    }

    /// Build completion client config from env vars.
    ///
    /// Reads:
    /// - `VERA_COMPLETION_BASE_URL`
    /// - `VERA_COMPLETION_MODEL_ID`
    /// - `VERA_COMPLETION_API_KEY` (optional, defaults to `none`)
    /// - `VERA_COMPLETION_TIMEOUT_SECS` (optional)
    /// - `VERA_COMPLETION_MAX_ALTERNATIVES` (optional)
    /// - `VERA_COMPLETION_MAX_TOKENS` (optional)
    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("VERA_COMPLETION_BASE_URL")
            .context("VERA_COMPLETION_BASE_URL not set")?
            .trim()
            .to_string();
        let model_id = std::env::var("VERA_COMPLETION_MODEL_ID")
            .context("VERA_COMPLETION_MODEL_ID not set")?
            .trim()
            .to_string();
        let api_key = std::env::var("VERA_COMPLETION_API_KEY")
            .unwrap_or_else(|_| "none".to_string())
            .trim()
            .to_string();

        if base_url.is_empty() {
            return Err(anyhow!("VERA_COMPLETION_BASE_URL is empty"));
        }
        if model_id.is_empty() {
            return Err(anyhow!("VERA_COMPLETION_MODEL_ID is empty"));
        }

        let timeout = std::env::var("VERA_COMPLETION_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_TIMEOUT_SECS);
        let max_alternatives = std::env::var("VERA_COMPLETION_MAX_ALTERNATIVES")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .map(|value| value.clamp(1, MAX_ALLOWED_ALTERNATIVES))
            .unwrap_or(DEFAULT_MAX_ALTERNATIVES);
        let max_tokens = std::env::var("VERA_COMPLETION_MAX_TOKENS")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .map(|value| value.clamp(MIN_MAX_TOKENS, MAX_MAX_TOKENS))
            .unwrap_or(DEFAULT_MAX_TOKENS);

        Ok(Self {
            base_url,
            model_id,
            api_key,
            timeout: Duration::from_secs(timeout),
            max_alternatives,
            max_tokens,
        })
    }

    fn endpoint_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

/// OpenAI-compatible chat completion client.
pub struct CompletionClient {
    client: reqwest::blocking::Client,
    config: CompletionClientConfig,
}

impl CompletionClient {
    /// Create a completion client if env vars are configured.
    pub fn from_env_if_configured() -> Result<Option<Self>> {
        if !CompletionClientConfig::is_configured() {
            return Ok(None);
        }
        Self::from_env().map(Some)
    }

    /// Create a completion client from env vars.
    pub fn from_env() -> Result<Self> {
        crate::init_tls();
        let config = CompletionClientConfig::from_env()?;
        let client = reqwest::blocking::Client::builder()
            .timeout(config.timeout)
            .build()
            .context("failed to create HTTP client for completion")?;
        Ok(Self { client, config })
    }

    /// Generate alternative code-search queries for RAG fusion.
    pub fn expand_query(&self, query: &str) -> Result<Vec<String>> {
        let prompt = format!(
            "Original query: {query}\n\nGenerate {} alternative code-search queries that keep the same intent but vary terminology and angle (implementation, API usage, symbols, related concepts).\n\nReturn ONLY a JSON array of strings.",
            self.config.max_alternatives
        );
        let request = ChatCompletionRequest {
            model: &self.config.model_id,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: "You generate query rewrites for code retrieval. Keep each query concise and faithful to the original intent. Output only the requested JSON.".to_string(),
                },
                ChatMessage {
                    role: "user",
                    content: prompt,
                },
            ],
            temperature: 0.2,
            max_tokens: self.config.max_tokens,
            response_format: ChatResponseFormat {
                kind: "json_object",
            },
            stream: false,
        };

        let response = self
            .client
            .post(self.config.endpoint_url())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .context("failed to call completion API")?;

        let status = response.status();
        if !status.is_success() {
            let message = response.text().unwrap_or_default();
            return Err(anyhow!(
                "completion API error (status {}): {}",
                status.as_u16(),
                message
            ));
        }

        let payload: serde_json::Value = response
            .json()
            .context("failed to parse completion API response")?;
        let choice = payload
            .get("choices")
            .and_then(|value| value.as_array())
            .and_then(|choices| choices.first())
            .ok_or_else(|| anyhow!("completion response did not include choices"))?;
        let finish_reason = choice
            .get("finish_reason")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        let message = choice
            .get("message")
            .ok_or_else(|| anyhow!("completion response did not include message"))?;
        let content = message
            .get("content")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();

        if content.is_empty() {
            let has_reasoning = message
                .get("reasoning_content")
                .and_then(|value| value.as_str())
                .is_some_and(|value| !value.trim().is_empty());
            if has_reasoning {
                return Err(anyhow!(
                    "completion response had empty content and only reasoning_content (finish_reason={finish_reason}); increase VERA_COMPLETION_MAX_TOKENS or switch to a non-reasoning completion model"
                ));
            }
            return Err(anyhow!(
                "completion response did not include message content"
            ));
        }

        let candidates = parse_query_candidates(content, self.config.max_alternatives)?;
        if candidates.is_empty() {
            return Err(anyhow!(
                "completion response did not contain any query candidates"
            ));
        }

        Ok(candidates)
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
    response_format: ChatResponseFormat,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: &'static str,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

fn parse_query_candidates(raw: &str, limit: usize) -> Result<Vec<String>> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw.trim()) {
        if let Some(parsed) = json_value_to_queries(&value) {
            return Ok(normalize_query_list(parsed, limit));
        }
    }

    if let Some((start, end)) = raw.find('[').zip(raw.rfind(']')) {
        let candidate = &raw[start..=end];
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(candidate) {
            if let Some(parsed) = json_value_to_queries(&value) {
                return Ok(normalize_query_list(parsed, limit));
            }
        }
    }

    let preview = raw.lines().take(3).collect::<Vec<_>>().join(" ");
    Err(anyhow!(
        "completion response is not a JSON array of query strings: {}",
        preview
    ))
}

fn json_value_to_queries(value: &serde_json::Value) -> Option<Vec<String>> {
    fn extract_array(value: &serde_json::Value) -> Option<Vec<String>> {
        let array = value.as_array()?;
        let mut items = Vec::with_capacity(array.len());
        for item in array {
            let text = item.as_str()?;
            items.push(text.to_string());
        }
        Some(items)
    }

    if let Some(parsed) = extract_array(value) {
        return Some(parsed);
    }

    let object = value.as_object()?;
    for key in ["rewrites", "queries", "candidates", "alternatives"] {
        if let Some(parsed) = object.get(key).and_then(extract_array) {
            return Some(parsed);
        }
    }

    None
}

fn normalize_query_list(candidates: Vec<String>, limit: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for candidate in candidates {
        let query = normalize_query(&candidate);
        if query.is_empty() {
            continue;
        }
        let key = query.to_ascii_lowercase();
        if seen.insert(key) {
            normalized.push(query);
        }
        if normalized.len() >= limit {
            break;
        }
    }

    normalized
}

fn normalize_query(query: &str) -> String {
    query
        .trim()
        .trim_matches('"')
        .trim_matches('`')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_query_candidates() {
        let raw = r#"["auth token refresh", "jwt expiry validation", "session middleware"]"#;
        let parsed = parse_query_candidates(raw, 4).unwrap();
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0], "auth token refresh");
    }

    #[test]
    fn parse_json_object_query_candidates() {
        let raw = r#"{"rewrites":["auth token refresh", "jwt expiry validation"]}"#;
        let parsed = parse_query_candidates(raw, 4).unwrap();
        assert_eq!(
            parsed,
            vec![
                "auth token refresh".to_string(),
                "jwt expiry validation".to_string(),
            ]
        );
    }

    #[test]
    fn parse_fenced_json_query_candidates() {
        let raw = "```json\n[\"auth\", \"token refresh\"]\n```";
        let parsed = parse_query_candidates(raw, 4).unwrap();
        assert_eq!(
            parsed,
            vec!["auth".to_string(), "token refresh".to_string()]
        );
    }

    #[test]
    fn parse_non_json_response_returns_error() {
        let raw = "1. auth token refresh\n2) jwt expiry handling\n- auth middleware";
        let error = parse_query_candidates(raw, 4).unwrap_err().to_string();
        assert!(error.contains("not a JSON array"), "{error}");
    }

    #[test]
    fn normalize_query_list_deduplicates_case_insensitive() {
        let normalized = normalize_query_list(
            vec![
                "Auth Refresh".to_string(),
                "auth refresh".to_string(),
                "AUTH REFRESH".to_string(),
            ],
            5,
        );
        assert_eq!(normalized, vec!["Auth Refresh".to_string()]);
    }
}
