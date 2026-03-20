//! Vera CLI — code indexing and retrieval for AI coding agents.
//!
//! Usage:
//!   vera index <path>    Index a codebase
//!   vera search <query>  Search the index
//!   vera update <path>   Incrementally update the index
//!   vera stats            Show index statistics

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "vera",
    about = "Evidence-backed code indexing & retrieval for AI coding agents",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output results as JSON (machine-readable).
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Index a codebase for search.
    Index {
        /// Path to the directory to index.
        path: String,
    },

    /// Search the indexed codebase.
    Search {
        /// The search query.
        query: String,

        /// Filter by programming language (case-insensitive).
        #[arg(long)]
        lang: Option<String>,

        /// Filter by file path glob pattern.
        #[arg(long)]
        path: Option<String>,

        /// Maximum number of results to return.
        #[arg(long, short = 'n')]
        limit: Option<usize>,
    },

    /// Incrementally update the index for changed files.
    Update {
        /// Path to the directory to update.
        path: String,
    },

    /// Show index statistics.
    Stats,
}

fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber (logs go to stderr).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("VERA_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Index { path } => {
            tracing::info!(path = %path, "indexing");
            eprintln!("vera index: not yet implemented (path: {path})");
        }
        Commands::Search {
            query,
            lang,
            path,
            limit,
        } => {
            tracing::info!(query = %query, "searching");
            let _ = (lang, path, limit); // Will be used in implementation
            eprintln!("vera search: not yet implemented (query: {query})");
        }
        Commands::Update { path } => {
            tracing::info!(path = %path, "updating");
            eprintln!("vera update: not yet implemented (path: {path})");
        }
        Commands::Stats => {
            tracing::info!("showing stats");
            eprintln!("vera stats: not yet implemented");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_index_command() {
        let cli = Cli::parse_from(["vera", "index", "/tmp/repo"]);
        assert!(matches!(cli.command, Commands::Index { path } if path == "/tmp/repo"));
    }

    #[test]
    fn cli_parses_search_command() {
        let cli = Cli::parse_from(["vera", "search", "find auth"]);
        assert!(matches!(cli.command, Commands::Search { query, .. } if query == "find auth"));
    }

    #[test]
    fn cli_parses_search_with_filters() {
        let cli = Cli::parse_from([
            "vera",
            "search",
            "find auth",
            "--lang",
            "rust",
            "--limit",
            "5",
        ]);
        match cli.command {
            Commands::Search {
                query, lang, limit, ..
            } => {
                assert_eq!(query, "find auth");
                assert_eq!(lang, Some("rust".to_string()));
                assert_eq!(limit, Some(5));
            }
            _ => panic!("expected Search command"),
        }
    }

    #[test]
    fn cli_parses_update_command() {
        let cli = Cli::parse_from(["vera", "update", "/tmp/repo"]);
        assert!(matches!(cli.command, Commands::Update { path } if path == "/tmp/repo"));
    }

    #[test]
    fn cli_parses_stats_command() {
        let cli = Cli::parse_from(["vera", "stats"]);
        assert!(matches!(cli.command, Commands::Stats));
    }

    #[test]
    fn cli_parses_json_flag() {
        let cli = Cli::parse_from(["vera", "--json", "stats"]);
        assert!(cli.json);
    }
}
