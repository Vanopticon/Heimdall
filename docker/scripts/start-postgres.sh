#!/usr/bin/env bash
set -euo pipefail

# Usage: ./docker/scripts/start-postgres.sh [workdir]
# If workdir is omitted the repository root (two levels up from this script)
# is used so the compose command runs from the project root where
# docker-compose.yml / docker/ files live.

# Resolve script and repository root locations so the script works from
# its new location under `docker/scripts/` even when invoked from elsewhere.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

WD="${1:-$REPO_ROOT}"
MARKER="$WD/.heimdall_db_started"

pushd "$WD" >/dev/null || { echo "workdir not found: $WD" >&2; exit 2; }

# Detect compose command (prefer `docker compose` if available)
if command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
    COMPOSE=(docker compose)
elif command -v docker-compose >/dev/null 2>&1; then
    COMPOSE=(docker-compose)
else
    echo "docker compose or docker-compose not found in PATH" >&2
    popd >/dev/null
    exit 1
fi

# Check if db container exists
ID="$(${COMPOSE[@]} ps -q db 2>/dev/null || true)"
if [ -n "$ID" ]; then
    RUNNING=$(docker inspect -f '{{.State.Running}}' "$ID" 2>/dev/null || echo "false")
    if [ "$RUNNING" = "true" ]; then
        echo "Postgres+AGE 'db' service already running (id=$ID)."
        popd >/dev/null
        exit 0
    fi
fi

echo "Starting Postgres+AGE 'db' service..."
"${COMPOSE[@]}" up -d db

# Record marker with container id
ID="$(${COMPOSE[@]} ps -q db 2>/dev/null || true)"
if [ -n "$ID" ]; then
    echo "$ID" > "$MARKER"
    echo "Started db (id=$ID). Marker created at $MARKER"
else
    echo "Started db but could not determine container id; marker not written" >&2
fi

popd >/dev/null
