#!/usr/bin/env bash
set -euo pipefail

# Usage: ./scripts/stop-postgres.sh [workdir]
# Only stops the db service if a marker file indicates this tool/script started it.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

WD="${1:-$REPO_ROOT}"
MARKER="$WD/.heimdall_db_started"

pushd "$WD" >/dev/null || { echo "workdir not found: $WD" >&2; exit 2; }

if command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
    COMPOSE=(docker compose)
elif command -v docker-compose >/dev/null 2>&1; then
    COMPOSE=(docker-compose)
else
    echo "docker compose or docker-compose not found in PATH" >&2
    popd >/dev/null
    exit 1
fi

if [ ! -f "$MARKER" ]; then
    echo "Marker file not found at $MARKER; will not stop a DB started externally."
    popd >/dev/null
    exit 0
fi

ID=$(cat "$MARKER" 2>/dev/null || true)
if [ -n "$ID" ]; then
    RUNNING=$(docker inspect -f '{{.State.Running}}' "$ID" 2>/dev/null || echo "false")
    if [ "$RUNNING" != "true" ]; then
        echo "Container $ID is not running; removing stale marker"
        rm -f "$MARKER"
        popd >/dev/null
        exit 0
    fi
fi

echo "Stopping Postgres+AGE 'db' service started by this script..."
"${COMPOSE[@]}" stop db || true
"${COMPOSE[@]}" rm -f db || true
rm -f "$MARKER"
echo "Stopped db and removed marker $MARKER"

popd >/dev/null
