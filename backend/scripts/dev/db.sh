#!/usr/bin/env bash
# Local Postgres lifecycle helper for fitai-api dev. Wraps docker-compose
# and sqlx-cli. Requires: docker (colima), sqlx-cli (`cargo install sqlx-cli`).

set -euo pipefail

cmd="${1:-help}"
backend_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$backend_dir"

wait_healthy() {
    echo "[db.sh] waiting for postgres to be healthy..."
    for _ in $(seq 1 30); do
        if [[ "$(docker inspect -f '{{.State.Health.Status}}' fitai-postgres 2>/dev/null)" == "healthy" ]]; then
            echo "[db.sh] postgres healthy."
            return 0
        fi
        sleep 1
    done
    echo "[db.sh] FAIL: postgres did not become healthy in 30 s"
    return 1
}

case "$cmd" in
    up)
        docker compose up -d postgres
        wait_healthy
        DATABASE_URL="${DATABASE_URL:-postgres://fitai:dev@localhost:5432/fitai}" \
            sqlx migrate run
        ;;
    down)
        docker compose down
        ;;
    reset)
        docker compose down -v
        docker compose up -d postgres
        wait_healthy
        DATABASE_URL="${DATABASE_URL:-postgres://fitai:dev@localhost:5432/fitai}" \
            sqlx migrate run
        ;;
    migrate)
        DATABASE_URL="${DATABASE_URL:-postgres://fitai:dev@localhost:5432/fitai}" \
            sqlx migrate run
        ;;
    help|*)
        cat <<USAGE
db.sh — local Postgres lifecycle.

Usage:  scripts/dev/db.sh <command>

Commands:
  up        Bring up Postgres (creates volume on first run), wait healthy, run migrations.
  down      Stop and remove the container (volume preserved).
  reset     down -v + up (drops the volume; clears all data).
  migrate   Run pending sqlx migrations against the running DB.
  help      This message.
USAGE
        ;;
esac
