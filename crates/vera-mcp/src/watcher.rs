//! Background file watcher for automatic index updates in MCP mode.
//!
//! Watches a project directory for file changes and triggers incremental
//! index updates after a debounce period. This keeps the index fresh
//! without requiring manual update calls.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use anyhow::Context;
use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};
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
    start_watching_internal(repo_path, false)
}

/// Start watching with progress logs printed to stderr.
///
/// Intended for `vera watch` CLI mode where users expect visible activity.
pub fn start_watching_with_progress(repo_path: &Path) -> Result<WatchHandle, String> {
    start_watching_internal(repo_path, true)
}

fn start_watching_internal(repo_path: &Path, progress_logs: bool) -> Result<WatchHandle, String> {
    let repo_path = repo_path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve path: {e}"))?;

    let idx_dir = vera_core::indexing::index_dir(&repo_path);
    if !idx_dir.exists() {
        return Err("No index found. Run search_code first to auto-index.".to_string());
    }

    let updating = Arc::new(AtomicBool::new(false));
    let updating_clone = updating.clone();
    let watcher_started_at = SystemTime::now();
    let seen_mtimes = Arc::new(Mutex::new(HashMap::<PathBuf, SystemTime>::new()));
    let seen_mtimes_clone = seen_mtimes.clone();
    let repo_clone = repo_path.clone();
    let ignored_dirs = vec![repo_path.join(".vera"), repo_path.join(".git")];

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

            let relevant_changes = collect_relevant_changes(&events, &repo_clone, &ignored_dirs);
            if relevant_changes.is_empty() {
                return;
            }

            let material_changes = {
                let mut seen = seen_mtimes_clone
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                filter_material_changes(&relevant_changes, watcher_started_at, &mut seen)
            };
            if material_changes.is_empty() {
                return;
            }

            let indexable_changes = match filter_indexable_changes(&repo_clone, &material_changes) {
                Ok(paths) => paths,
                Err(error) => {
                    warn!(error = %error, "failed to apply index ignore rules for watcher changes");
                    material_changes.clone()
                }
            };

            if indexable_changes.is_empty() {
                if progress_logs {
                    eprintln!("[watch] changes ignored by indexing rules");
                }
                return;
            }

            if progress_logs {
                eprintln!(
                    "[watch] detected {} file change(s): {}",
                    indexable_changes.len(),
                    format_change_preview(&indexable_changes, &repo_clone, 5)
                );
            }

            // Skip if already updating.
            if updating_clone
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
                .is_err()
            {
                debug!("Skipping auto-update: previous update still running");
                if progress_logs {
                    eprintln!(
                        "[watch] update already running, changes will be picked up on next cycle"
                    );
                }
                return;
            }

            let repo = repo_clone.clone();
            let flag = updating_clone.clone();
            let changed_count = indexable_changes.len();

            std::thread::spawn(move || {
                run_incremental_update(&repo, &flag, progress_logs, changed_count);
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
fn run_incremental_update(
    repo_path: &Path,
    updating: &AtomicBool,
    progress_logs: bool,
    changed_count: usize,
) {
    debug!(path = %repo_path.display(), "Auto-update triggered by file changes");
    if progress_logs {
        eprintln!("[watch] processing {} changed file(s)...", changed_count);
    }

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
                if progress_logs {
                    eprintln!(
                        "[watch] index updated: modified={}, added={}, deleted={}",
                        summary.files_modified, summary.files_added, summary.files_deleted
                    );
                }
            } else {
                debug!("Auto-update: no changes detected");
                if progress_logs {
                    eprintln!("[watch] no index changes detected");
                }
            }
        }
        Err(e) => {
            warn!(error = %e, "Auto-update failed");
            if progress_logs {
                eprintln!("[watch] auto-update failed: {e}");
            }
        }
    }

    updating.store(false, Ordering::SeqCst);
}

fn collect_relevant_changes(
    events: &[notify_debouncer_mini::DebouncedEvent],
    repo_path: &Path,
    ignored_dirs: &[PathBuf],
) -> Vec<PathBuf> {
    let mut unique = std::collections::BTreeSet::new();
    for event in events {
        if event.kind != DebouncedEventKind::Any {
            continue;
        }
        if ignored_dirs.iter().any(|dir| event.path.starts_with(dir)) {
            continue;
        }
        if event.path == repo_path {
            continue;
        }
        unique.insert(event.path.clone());
    }
    unique.into_iter().collect()
}

fn filter_material_changes(
    changed_paths: &[PathBuf],
    watcher_started_at: SystemTime,
    seen_mtimes: &mut HashMap<PathBuf, SystemTime>,
) -> Vec<PathBuf> {
    let mut material = Vec::new();

    for path in changed_paths {
        if !path.exists() {
            seen_mtimes.remove(path);
            material.push(path.clone());
            continue;
        }

        let metadata = match std::fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(_) => {
                material.push(path.clone());
                continue;
            }
        };

        if metadata.is_dir() {
            continue;
        }
        if !metadata.is_file() {
            continue;
        }

        let modified = match metadata.modified() {
            Ok(modified) => modified,
            Err(_) => {
                material.push(path.clone());
                continue;
            }
        };

        match seen_mtimes.get(path) {
            Some(previous) if *previous >= modified => {}
            Some(_) => {
                seen_mtimes.insert(path.clone(), modified);
                material.push(path.clone());
            }
            None => {
                // Seed baseline mtimes for pre-existing files so startup/access
                // notifications don't trigger a full incremental update cycle.
                seen_mtimes.insert(path.clone(), modified);
                if modified > watcher_started_at {
                    material.push(path.clone());
                }
            }
        }
    }

    material
}

fn filter_indexable_changes(
    repo_path: &Path,
    changed_paths: &[PathBuf],
) -> anyhow::Result<Vec<PathBuf>> {
    let runtime = load_runtime_settings()?;
    let discovery = vera_core::discovery::discover_files(repo_path, &runtime.config.indexing)
        .with_context(|| {
            format!(
                "failed to apply discovery rules for {}",
                repo_path.display()
            )
        })?;

    let discoverable: HashSet<String> = discovery
        .files
        .into_iter()
        .map(|file| normalize_relative_path(&file.relative_path))
        .collect();
    let indexed = load_indexed_paths(repo_path)?;

    Ok(filter_indexable_changes_with_sets(
        repo_path,
        changed_paths,
        &discoverable,
        &indexed,
    ))
}

fn load_indexed_paths(repo_path: &Path) -> anyhow::Result<HashSet<String>> {
    let metadata_path = vera_core::indexing::index_dir(repo_path).join("metadata.db");
    if !metadata_path.exists() {
        return Ok(HashSet::new());
    }

    let store = vera_core::storage::metadata::MetadataStore::open(&metadata_path)
        .with_context(|| format!("failed to open metadata store: {}", metadata_path.display()))?;
    let indexed = store
        .indexed_files()
        .with_context(|| format!("failed to read indexed files: {}", metadata_path.display()))?;

    Ok(indexed
        .into_iter()
        .map(|path| normalize_relative_path(&path))
        .collect())
}

fn filter_indexable_changes_with_sets(
    repo_path: &Path,
    changed_paths: &[PathBuf],
    discoverable: &HashSet<String>,
    indexed: &HashSet<String>,
) -> Vec<PathBuf> {
    changed_paths
        .iter()
        .filter(|path| {
            let Some(relative) = relative_path(repo_path, path) else {
                return false;
            };

            if path.exists() {
                if path.is_file() {
                    return discoverable.contains(&relative);
                }
                if path.is_dir() {
                    let prefix = format!("{relative}/");
                    return discoverable.iter().any(|entry| entry.starts_with(&prefix))
                        || indexed.iter().any(|entry| entry.starts_with(&prefix));
                }
                return false;
            }

            if indexed.contains(&relative) {
                return true;
            }
            let prefix = format!("{relative}/");
            indexed.iter().any(|entry| entry.starts_with(&prefix))
        })
        .cloned()
        .collect()
}

fn relative_path(repo_path: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(repo_path).ok()?;
    let as_str = rel.to_string_lossy();
    if as_str.is_empty() || as_str == "." {
        return None;
    }
    Some(normalize_relative_path(&as_str))
}

fn normalize_relative_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn format_change_preview(paths: &[PathBuf], repo_path: &Path, max_items: usize) -> String {
    let mut shown: Vec<String> = paths
        .iter()
        .take(max_items)
        .map(|path| {
            path.strip_prefix(repo_path)
                .unwrap_or(path)
                .display()
                .to_string()
        })
        .collect();

    if paths.len() > max_items {
        shown.push(format!("+{} more", paths.len() - max_items));
    }

    shown.join(", ")
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
    use std::fs;

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

    #[test]
    fn collect_relevant_changes_ignores_vera_directory() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo = temp.path().to_path_buf();
        let ignored_dirs = vec![repo.join(".vera"), repo.join(".git")];

        let events = vec![
            notify_debouncer_mini::DebouncedEvent {
                path: repo.join(".vera/index.db"),
                kind: notify_debouncer_mini::DebouncedEventKind::Any,
            },
            notify_debouncer_mini::DebouncedEvent {
                path: repo.join(".git/HEAD"),
                kind: notify_debouncer_mini::DebouncedEventKind::Any,
            },
            notify_debouncer_mini::DebouncedEvent {
                path: repo.join("src/main.rs"),
                kind: notify_debouncer_mini::DebouncedEventKind::Any,
            },
        ];

        let relevant = collect_relevant_changes(&events, &repo, &ignored_dirs);
        assert_eq!(relevant, vec![repo.join("src/main.rs")]);
    }

    #[test]
    fn format_change_preview_is_relative_and_capped() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo = temp.path().to_path_buf();

        let paths = vec![
            repo.join("src/a.rs"),
            repo.join("src/b.rs"),
            repo.join("src/c.rs"),
        ];

        let preview = format_change_preview(&paths, &repo, 2);
        assert!(preview.contains("src/a.rs"));
        assert!(preview.contains("src/b.rs"));
        assert!(preview.contains("+1 more"));
    }

    #[test]
    fn filter_indexable_changes_with_sets_respects_discovery_and_deletions() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo = temp.path().to_path_buf();

        fs::create_dir_all(repo.join("src")).unwrap();
        fs::create_dir_all(repo.join("vendor")).unwrap();
        fs::write(repo.join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(repo.join("vendor/generated.rs"), "pub fn generated() {}\n").unwrap();

        let deleted_file = repo.join("src/deleted.rs");

        let changed = vec![
            repo.join("src/main.rs"),
            repo.join("vendor/generated.rs"),
            deleted_file.clone(),
        ];

        let discoverable = HashSet::from(["src/main.rs".to_string()]);
        let indexed = HashSet::from(["src/deleted.rs".to_string()]);

        let filtered = filter_indexable_changes_with_sets(&repo, &changed, &discoverable, &indexed);

        assert_eq!(filtered, vec![repo.join("src/main.rs"), deleted_file]);
    }

    #[test]
    fn filter_indexable_changes_with_sets_accepts_directory_delete_for_indexed_children() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo = temp.path().to_path_buf();

        let deleted_dir = repo.join("src/legacy");
        let changed = vec![deleted_dir.clone()];
        let discoverable = HashSet::new();
        let indexed = HashSet::from(["src/legacy/old.rs".to_string()]);

        let filtered = filter_indexable_changes_with_sets(&repo, &changed, &discoverable, &indexed);

        assert_eq!(filtered, vec![deleted_dir]);
    }

    #[test]
    fn filter_material_changes_ignores_existing_files_older_than_start() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo = temp.path().to_path_buf();

        fs::create_dir_all(repo.join("src")).unwrap();
        let file = repo.join("src/main.rs");
        fs::write(&file, "fn main() {}\n").unwrap();

        let watcher_started_at = SystemTime::now() + Duration::from_secs(1);
        let mut seen = HashMap::new();
        let filtered =
            filter_material_changes(std::slice::from_ref(&file), watcher_started_at, &mut seen);

        assert!(filtered.is_empty());
        assert!(seen.contains_key(&file));
    }

    #[test]
    fn filter_material_changes_skips_repeated_same_mtime_events() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo = temp.path().to_path_buf();

        fs::create_dir_all(repo.join("src")).unwrap();
        let file = repo.join("src/main.rs");
        fs::write(&file, "fn main() {}\n").unwrap();

        let mut seen = HashMap::new();
        let first = filter_material_changes(
            std::slice::from_ref(&file),
            SystemTime::UNIX_EPOCH,
            &mut seen,
        );
        let second = filter_material_changes(
            std::slice::from_ref(&file),
            SystemTime::UNIX_EPOCH,
            &mut seen,
        );

        assert_eq!(first, vec![file]);
        assert!(second.is_empty());
    }

    #[test]
    fn filter_material_changes_ignores_existing_directories_but_keeps_deletions() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo = temp.path().to_path_buf();

        let existing_dir = repo.join("src");
        fs::create_dir_all(&existing_dir).unwrap();
        let deleted_file = repo.join("src/deleted.rs");

        let mut seen = HashMap::new();
        let filtered = filter_material_changes(
            &[existing_dir, deleted_file.clone()],
            SystemTime::UNIX_EPOCH,
            &mut seen,
        );

        assert_eq!(filtered, vec![deleted_file]);
    }
}
