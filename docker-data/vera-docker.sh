#!/usr/bin/env sh
set -eu

IMAGE="${VERA_IMAGE:-vera-vera}"
WORKSPACE="${VERA_WORKSPACE:-$PWD}"
STATE_DIR="${VERA_STATE_DIR:-$PWD/docker-data/vera-home}"

mkdir -p "${STATE_DIR}"

exec docker run --rm \
  --add-host=host.docker.internal:host-gateway \
  -e VERA_NO_UPDATE_CHECK=1 \
  -v "${WORKSPACE}:/workspace" \
  -v "${STATE_DIR}:/root/.vera" \
  -w /workspace \
  "${IMAGE}" "$@"
