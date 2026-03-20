#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Load API credentials if available
if [ -f "$REPO_ROOT/secrets.env" ]; then
    set -a
    source "$REPO_ROOT/secrets.env"
    set +a
    echo "[init] Loaded secrets.env"
else
    echo "[init] WARNING: secrets.env not found - embedding/reranker APIs will not work"
fi

# Ensure Rust toolchain is available
if ! command -v cargo &>/dev/null; then
    echo "[init] ERROR: cargo not found. Install Rust: https://rustup.rs"
    exit 1
fi

echo "[init] Rust $(rustc --version)"

# Install tree-sitter CLI if not present
if ! command -v tree-sitter &>/dev/null; then
    echo "[init] Installing tree-sitter CLI..."
    cargo install tree-sitter-cli 2>/dev/null || echo "[init] tree-sitter CLI install skipped (may already be building)"
fi

# Build the project if Cargo.toml exists
if [ -f "$REPO_ROOT/Cargo.toml" ]; then
    echo "[init] Building project..."
    cargo build 2>&1 | tail -5
    echo "[init] Build complete"
else
    echo "[init] No Cargo.toml found - skipping build (expected in early milestones)"
fi

# Create benchmark repos directory
mkdir -p "$REPO_ROOT/.bench/repos"

# Ensure .bench is gitignored
if ! grep -q '.bench/' "$REPO_ROOT/.gitignore" 2>/dev/null; then
    echo '.bench/' >> "$REPO_ROOT/.gitignore"
fi

echo "[init] Environment ready"
