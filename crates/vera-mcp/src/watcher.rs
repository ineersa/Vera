//! Background file watcher for automatic index updates in MCP mode.
//!
//! Watches a project directory for file changes and triggers incremental
//! index updates after a debounce period. This keeps the index fresh
//! without requiring manual update calls.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use serde::Deserialize;
use tracing::{debug, info, warn};

/// Debounce interval: wait this long after the last file change before updating.
const DEBOUNCE_SECS: u64 = 2;

/// Handle to a running file watcher. Dropping it stops the watcher.
pub struct WatchHandle {
    _watcher: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
    /// Set to true when an update is in progress.
    updating: Arc<AtomicBool>,
}

impl WatchHandle {
    /// True if an incremental update is currently running.
    pub fn is_updating(&self) -> bool {
        self.updating.load(Ordering::Relaxed)
    }
}

/// Start watching a project directory for file changes.
///
/// When changes are detected (after debouncing), triggers an incremental
/// index update in a background thread. Returns a handle that keeps the
/// watcher alive; drop it to stop watching.
pub fn start_watching(repo_path: &Path) -> Result<WatchHandle, String> {
    let repo_path = repo_path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve path: {e}"))?;

    let idx_dir = vera_core::indexing::index_dir(&repo_path);
    if !idx_dir.exists() {
        return Err("No index found. Run search_code first to auto-index.".to_string());
    }

    let updating = Arc::new(AtomicBool::new(false));
    let updating_clone = updating.clone();
    let repo_clone = repo_path.clone();

    let mut debouncer = new_debouncer(
        Duration::from_secs(DEBOUNCE_SECS),
        move |events: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
            let events = match events {
                Ok(e) => e,
                Err(e) => {
                    warn!(error = %e, "File watcher error");
                    return;
                }
            };

            // Filter out events inside .vera/ directory.
            let has_relevant_changes = events.iter().any(|e| {
                e.kind == DebouncedEventKind::Any && !e.path.starts_with(repo_clone.join(".vera"))
            });

            if !has_relevant_changes {
                return;
            }

            // Skip if already updating.
            if updating_clone
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
                .is_err()
            {
                debug!("Skipping auto-update: previous update still running");
                return;
            }

            let repo = repo_clone.clone();
            let flag = updating_clone.clone();

            std::thread::spawn(move || {
                run_incremental_update(&repo, &flag);
            });
        },
    )
    .map_err(|e| format!("Failed to create file watcher: {e}"))?;

    debouncer
        .watcher()
        .watch(&repo_path, notify::RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch directory: {e}"))?;

    info!(path = %repo_path.display(), "Started file watcher for auto-indexing");

    Ok(WatchHandle {
        _watcher: debouncer,
        updating,
    })
}

/// Run an incremental update, resetting the flag when done.
fn run_incremental_update(repo_path: &Path, updating: &AtomicBool) {
    debug!(path = %repo_path.display(), "Auto-update triggered by file changes");

    let result = run_update_blocking(repo_path);

    match result {
        Ok(summary) => {
            let changed = summary.files_modified + summary.files_added + summary.files_deleted;
            if changed > 0 {
                info!(
                    modified = summary.files_modified,
                    added = summary.files_added,
                    deleted = summary.files_deleted,
                    "Auto-update complete"
                );
            } else {
                debug!("Auto-update: no changes detected");
            }
        }
        Err(e) => {
            warn!(error = %e, "Auto-update failed");
        }
    }

    updating.store(false, Ordering::SeqCst);
}

/// Blocking wrapper around the async update_repository.
fn run_update_blocking(
    repo_path: &Path,
) -> Result<vera_core::indexing::UpdateSummary, anyhow::Error> {
    let runtime = match load_runtime_settings() {
        Ok(settings) => settings,
        Err(err) => {
            warn!(error = %err, "Failed to load saved runtime config; using defaults");
            RuntimeSettings::default()
        }
    };

    let backend = vera_core::config::resolve_backend(runtime.backend_hint);
    let mut config = runtime.config;
    config.adjust_for_backend(backend);

    let rt = tokio::runtime::Runtime::new()?;

    let (provider, model_name) = rt.block_on(vera_core::embedding::create_dynamic_provider(
        &config, backend,
    ))?;

    rt.block_on(vera_core::indexing::update_repository(
        repo_path,
        &provider,
        &config,
        &model_name,
    ))
}

#[derive(Debug, Clone)]
struct RuntimeSettings {
    config: vera_core::config::VeraConfig,
    backend_hint: Option<vera_core::config::InferenceBackend>,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            config: vera_core::config::VeraConfig::default(),
            backend_hint: None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct StoredConfigSnapshot {
    #[serde(default)]
    local_mode: Option<bool>,
    #[serde(default)]
    backend: Option<vera_core::config::InferenceBackend>,
    #[serde(default)]
    core_config: Option<vera_core::config::VeraConfig>,
}

fn load_runtime_settings() -> anyhow::Result<RuntimeSettings> {
    let config_path = vera_core::local_models::vera_home_dir()?.join("config.json");
    load_runtime_settings_from_path(&config_path)
}

fn load_runtime_settings_from_path(path: &Path) -> anyhow::Result<RuntimeSettings> {
    if !path.exists() {
        return Ok(RuntimeSettings::default());
    }

    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read runtime config: {}", path.display()))?;
    if bytes.is_empty() {
        return Ok(RuntimeSettings::default());
    }

    let stored: StoredConfigSnapshot = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse runtime config: {}", path.display()))?;

    let backend_hint = stored.backend.or(match stored.local_mode {
        Some(true) => Some(vera_core::config::InferenceBackend::OnnxJina(
            vera_core::config::OnnxExecutionProvider::Cpu,
        )),
        Some(false) => Some(vera_core::config::InferenceBackend::Api),
        None => None,
    });

    Ok(RuntimeSettings {
        config: stored.core_config.unwrap_or_default(),
        backend_hint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_saved_core_config_values() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("config.json");

        let mut saved = vera_core::config::VeraConfig::default();
        saved.indexing.max_chunk_tokens = 512;
        saved.retrieval.max_rerank_batch = 7;

        let payload = serde_json::json!({
            "core_config": saved,
        });
        std::fs::write(&path, serde_json::to_vec(&payload).unwrap()).unwrap();

        let settings = load_runtime_settings_from_path(&path).unwrap();
        assert_eq!(settings.config.indexing.max_chunk_tokens, 512);
        assert_eq!(settings.config.retrieval.max_rerank_batch, 7);
        assert!(settings.backend_hint.is_none());
    }

    #[test]
    fn local_mode_true_maps_to_local_backend() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("config.json");

        std::fs::write(&path, br#"{"local_mode":true}"#).unwrap();

        let settings = load_runtime_settings_from_path(&path).unwrap();
        assert_eq!(
            settings.backend_hint,
            Some(vera_core::config::InferenceBackend::OnnxJina(
                vera_core::config::OnnxExecutionProvider::Cpu
            ))
        );
    }

    #[test]
    fn explicit_backend_takes_precedence_over_local_mode() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("config.json");

        std::fs::write(&path, br#"{"local_mode":true,"backend":"api"}"#).unwrap();

        let settings = load_runtime_settings_from_path(&path).unwrap();
        assert_eq!(
            settings.backend_hint,
            Some(vera_core::config::InferenceBackend::Api)
        );
    }
}
