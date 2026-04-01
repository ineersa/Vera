//! `vera watch` command: watch a project directory and auto-update the index.

use anyhow::Result;

/// Run the watch command. Blocks until the process is killed (Ctrl-C).
pub fn run(path: &str, json: bool) -> Result<()> {
    let repo_path = std::path::Path::new(path)
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Failed to resolve path: {e}"))?;

    let _handle = vera_mcp::watcher::start_watching_with_progress(&repo_path)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if json {
        println!(
            "{}",
            serde_json::json!({
                "status": "watching",
                "path": repo_path.display().to_string(),
                "message": "Watching for file changes. Index will auto-update and print progress logs. Press Ctrl-C to stop."
            })
        );
    } else {
        eprintln!("[watch] started in {}", repo_path.display());
        eprintln!(
            "[watch] waiting for file changes (uses same ignore/exclude rules as indexing; Ctrl-C to stop)"
        );
    }

    // Block forever; the watcher runs in background threads.
    // The process exits on Ctrl-C (SIGINT).
    loop {
        std::thread::park();
    }
}
