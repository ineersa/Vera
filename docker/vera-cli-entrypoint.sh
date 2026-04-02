#!/bin/sh
set -eu

if [ "$#" -eq 0 ]; then
  set -- help
fi

if [ "$1" = "mcp" ]; then
  echo "Error: MCP mode is disabled in this CLI-only Docker image." >&2
  echo "Run Vera CLI commands such as: search, index, update, overview, stats." >&2
  exit 64
fi

exec /usr/local/bin/vera "$@"
