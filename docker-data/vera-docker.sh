#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

IMAGE="vera-vera"
WORKSPACE="${VERA_WORKSPACE:-${REPO_ROOT}}"
STATE_DIR="${REPO_ROOT}/docker-data/vera-home"

if [[ "${1:-}" == "mcp" ]]; then
  echo "Error: MCP is disabled for this Docker wrapper. Use Vera CLI commands only." >&2
  exit 64
fi

mkdir -p "${STATE_DIR}"

container_id="$(docker create \
  --add-host=host.docker.internal:host-gateway \
  -v "${WORKSPACE}:/workspace" \
  -v "${STATE_DIR}:/root/.vera" \
  "${IMAGE}" "$@")"

cleanup() {
  docker rm -f "${container_id}" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

docker start -a "${container_id}"
