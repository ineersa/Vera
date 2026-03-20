#!/usr/bin/env bash
# Vera Evaluation Corpus Setup Script
#
# Clones test repositories at pinned commit SHAs for deterministic evaluation.
# Idempotent: safe to re-run, skips repos already at correct SHA.
#
# Usage: ./eval/setup-corpus.sh [--force]
#
# Options:
#   --force    Re-clone repos even if they exist at the correct SHA

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CORPUS_FILE="$SCRIPT_DIR/corpus.toml"

FORCE=false
if [[ "${1:-}" == "--force" ]]; then
    FORCE=true
fi

if [[ ! -f "$CORPUS_FILE" ]]; then
    echo "ERROR: corpus.toml not found at $CORPUS_FILE"
    exit 1
fi

# Parse clone_root from corpus.toml
CLONE_ROOT=$(python3 -c "
import tomllib, os, sys
with open('$CORPUS_FILE', 'rb') as f:
    data = tomllib.load(f)
root = data.get('corpus', {}).get('clone_root', '.bench/repos')
# Make absolute relative to repo root
if not os.path.isabs(root):
    root = os.path.join('$REPO_ROOT', root)
print(root)
")

echo "=== Vera Corpus Setup ==="
echo "Corpus file: $CORPUS_FILE"
echo "Clone root:  $CLONE_ROOT"
echo ""

mkdir -p "$CLONE_ROOT"

# Parse repos from corpus.toml and clone each
python3 -c "
import tomllib, json
with open('$CORPUS_FILE', 'rb') as f:
    data = tomllib.load(f)
for repo in data.get('repos', []):
    print(json.dumps(repo))
" | while IFS= read -r repo_json; do
    NAME=$(echo "$repo_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['name'])")
    URL=$(echo "$repo_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['url'])")
    COMMIT=$(echo "$repo_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['commit'])")
    LANG=$(echo "$repo_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['language'])")

    REPO_DIR="$CLONE_ROOT/$NAME"

    echo "--- $NAME ($LANG) ---"
    echo "  URL:    $URL"
    echo "  Commit: $COMMIT"

    # Check if repo exists and is at correct SHA
    if [[ -d "$REPO_DIR/.git" ]] && [[ "$FORCE" != "true" ]]; then
        CURRENT_SHA=$(git -C "$REPO_DIR" rev-parse HEAD 2>/dev/null || echo "unknown")
        if [[ "$CURRENT_SHA" == "$COMMIT" ]]; then
            echo "  Status: Already at correct SHA, skipping"
            echo ""
            continue
        else
            echo "  Status: Exists but at $CURRENT_SHA, re-cloning..."
            rm -rf "$REPO_DIR"
        fi
    elif [[ -d "$REPO_DIR" ]]; then
        echo "  Status: Forcing re-clone..."
        rm -rf "$REPO_DIR"
    fi

    echo "  Cloning..."
    git clone --quiet "$URL" "$REPO_DIR"
    git -C "$REPO_DIR" checkout --quiet "$COMMIT"

    # Verify
    ACTUAL_SHA=$(git -C "$REPO_DIR" rev-parse HEAD)
    if [[ "$ACTUAL_SHA" == "$COMMIT" ]]; then
        echo "  Status: Cloned and verified at $COMMIT"
    else
        echo "  ERROR: SHA mismatch after checkout. Expected $COMMIT, got $ACTUAL_SHA"
        exit 1
    fi
    echo ""
done

echo "=== Corpus setup complete ==="

# Print summary
echo ""
echo "Repos cloned:"
python3 -c "
import tomllib
with open('$CORPUS_FILE', 'rb') as f:
    data = tomllib.load(f)
for repo in data.get('repos', []):
    print(f\"  {repo['name']:15s} {repo['language']:12s} {repo['commit'][:12]}\")
"
