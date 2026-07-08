#!/usr/bin/env bash
# One-shot local dev: Postgres → API → seed demo user → flutter run.
#
# Usage:
#   scripts/dev/run.sh              # full stack + flutter (debug test account ready)
#   scripts/dev/run.sh backend      # postgres + API only (keeps running in foreground)
#   scripts/dev/run.sh seed         # ensure demo@fitai.app exists
#   scripts/dev/run.sh flutter      # flutter only (assumes API is up)
#   scripts/dev/run.sh stop         # stop background API + optional postgres down
#
# Environment / flags:
#   TARGET=android     API_BASE_URL=http://10.0.2.2:8080 (Android emulator)
#   TARGET=local       default — localhost:8080
#   TEST_LOGIN_EMAIL / TEST_LOGIN_PASSWORD — override DevLogin defaults
#   SKIP_BUILD=1       skip `cargo build` warm-up before starting API
#   POSTGRES_DOWN=1    with `stop`, also run docker compose down

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BACKEND_DIR="$REPO_ROOT/backend"
MOBILE_DIR="$REPO_ROOT/mobile"
PID_FILE="${TMPDIR:-/tmp}/fitai-dev-api.pid"
LOG_FILE="${TMPDIR:-/tmp}/fitai-dev-api.log"

DEV_EMAIL="${TEST_LOGIN_EMAIL:-demo@fitai.app}"
DEV_PASSWORD="${TEST_LOGIN_PASSWORD:-demo1234}"
DATABASE_URL="${DATABASE_URL:-postgres://fitai:dev@localhost:5432/fitai}"
JWT_SECRET="${JWT_SECRET:-dev-only-secret-replace-in-production}"
PORT="${PORT:-8080}"
TARGET="${TARGET:-local}"
PHOTO_STORE_ROOT="${PHOTO_STORE_ROOT:-$BACKEND_DIR/data/photos}"

case "$TARGET" in
local)
  API_BASE_URL="${API_BASE_URL:-http://localhost:${PORT}}"
  ;;
android)
  API_BASE_URL="${API_BASE_URL:-http://10.0.2.2:${PORT}}"
  ;;
*)
  echo "[dev] unknown TARGET=$TARGET (use local or android)" >&2
  exit 1
  ;;
esac

API_URL="http://127.0.0.1:${PORT}"

log() {
  echo "[dev] $*"
}

wait_postgres() {
  log "waiting for postgres..."
  for _ in $(seq 1 30); do
    if [[ "$(docker inspect -f '{{.State.Health.Status}}' fitai-postgres 2>/dev/null || echo missing)" == "healthy" ]]; then
      log "postgres healthy"
      return 0
    fi
    sleep 1
  done
  log "FAIL: postgres not healthy after 30s (is Docker running?)"
  return 1
}

postgres_up() {
  if [[ "$(docker inspect -f '{{.State.Health.Status}}' fitai-postgres 2>/dev/null || echo missing)" == "healthy" ]]; then
    log "postgres already running"
    return 0
  fi
  log "starting postgres (docker compose)..."
  (cd "$BACKEND_DIR" && docker compose up -d postgres)
  wait_postgres
}

api_running() {
  curl -sf "$API_URL/health" >/dev/null 2>&1
}

wait_api() {
  log "waiting for API at $API_URL ..."
  for i in $(seq 1 120); do
    if api_running; then
      log "API ready (${i}s)"
      return 0
    fi
    sleep 1
  done
  log "FAIL: API did not respond. Tail: $LOG_FILE"
  tail -20 "$LOG_FILE" 2>/dev/null || true
  return 1
}

start_api_background() {
  if api_running; then
    log "API already up at $API_URL"
    return 0
  fi

  mkdir -p "$PHOTO_STORE_ROOT"

  if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
    log "warming cargo build (first run can take a minute)..."
    (cd "$BACKEND_DIR" && cargo build -p fitai-api --locked) >>"$LOG_FILE" 2>&1
  fi

  log "starting API in background (log: $LOG_FILE)..."
  (
    cd "$BACKEND_DIR"
    export DATABASE_URL JWT_SECRET PORT PHOTO_STORE_ROOT RUST_LOG="${RUST_LOG:-info}"
    exec cargo run -p fitai-api --locked
  ) >>"$LOG_FILE" 2>&1 &
  echo $! >"$PID_FILE"
  wait_api
}

start_api_foreground() {
  mkdir -p "$PHOTO_STORE_ROOT"
  log "starting API in foreground on port $PORT..."
  cd "$BACKEND_DIR"
  export DATABASE_URL JWT_SECRET PORT PHOTO_STORE_ROOT RUST_LOG="${RUST_LOG:-info}"
  exec cargo run -p fitai-api --locked
}

stop_api() {
  if [[ -f "$PID_FILE" ]]; then
    local pid
    pid="$(cat "$PID_FILE")"
    if kill -0 "$pid" 2>/dev/null; then
      log "stopping API (pid $pid)..."
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
    rm -f "$PID_FILE"
  fi
  if api_running; then
    log "note: something is still listening on $API_URL (not our pid file?)"
  fi
}

seed_user() {
  if ! api_running; then
    log "API not up — start it first"
    return 1
  fi
  local code
  code="$(
    curl -s -o /dev/null -w '%{http_code}' -X POST "$API_URL/auth/register" \
      -H 'Content-Type: application/json' \
      -d "{\"email\":\"${DEV_EMAIL}\",\"password\":\"${DEV_PASSWORD}\"}"
  )"
  case "$code" in
  201)
    log "registered test user $DEV_EMAIL"
    ;;
  409)
    log "test user $DEV_EMAIL already exists"
    ;;
  *)
    log "FAIL: register returned HTTP $code for $DEV_EMAIL"
    return 1
    ;;
  esac
}

run_flutter() {
  if ! command -v flutter >/dev/null 2>&1; then
    log "FAIL: flutter not on PATH"
    return 1
  fi
  log "flutter run → API $API_BASE_URL, test account $DEV_EMAIL"
  cd "$MOBILE_DIR"
  exec flutter run \
    --dart-define=API_BASE_URL="$API_BASE_URL" \
    --dart-define=TEST_LOGIN_EMAIL="$DEV_EMAIL" \
    --dart-define=TEST_LOGIN_PASSWORD="$DEV_PASSWORD" \
    "${FLUTTER_ARGS[@]}"
}

cmd="${1:-all}"
shift || true

case "$cmd" in
all)
  postgres_up
  start_api_background
  seed_user
  run_flutter
  ;;
backend | api)
  postgres_up
  start_api_foreground
  ;;
seed)
  seed_user
  ;;
flutter | app)
  run_flutter
  ;;
stop)
  stop_api
  if [[ "${POSTGRES_DOWN:-0}" == "1" ]]; then
    log "stopping postgres..."
    (cd "$BACKEND_DIR" && docker compose down)
  fi
  ;;
status)
  if [[ "$(docker inspect -f '{{.State.Health.Status}}' fitai-postgres 2>/dev/null || echo missing)" == "healthy" ]]; then
    log "postgres: up"
  else
    log "postgres: down"
  fi
  if api_running; then
    log "API: up ($API_URL)"
  else
    log "API: down"
  fi
  if [[ -f "$PID_FILE" ]]; then
    log "API pid file: $(cat "$PID_FILE")"
  fi
  log "test user: $DEV_EMAIL"
  log "flutter API_BASE_URL ($TARGET): $API_BASE_URL"
  ;;
help | -h | --help)
  cat <<'USAGE'
One-shot local dev: Postgres → API → seed demo user → flutter run.

Usage:
  scripts/dev/run.sh              # full stack + flutter (debug test account ready)
  scripts/dev/run.sh backend      # postgres + API only (foreground)
  scripts/dev/run.sh seed         # ensure demo@fitai.app exists
  scripts/dev/run.sh flutter      # flutter only (assumes API is up)
  scripts/dev/run.sh stop         # stop background API
  scripts/dev/run.sh status       # postgres / API / config snapshot

Environment:
  TARGET=android     use http://10.0.2.2:8080 (Android emulator)
  TARGET=local       default — http://localhost:8080
  TEST_LOGIN_EMAIL / TEST_LOGIN_PASSWORD   override DevLogin defaults
  SKIP_BUILD=1       skip cargo warm-up before API start
  POSTGRES_DOWN=1    with stop, also docker compose down
  FLUTTER_ARGS       extra args passed to flutter run (bash array)

Examples:
  scripts/dev/run.sh
  TARGET=android scripts/dev/run.sh
  scripts/dev/run.sh backend          # API in foreground, no flutter
  SKIP_BUILD=1 scripts/dev/run.sh seed
USAGE
  ;;
*)
  echo "[dev] unknown command: $cmd (try: all, backend, seed, flutter, stop, status)" >&2
  exit 1
  ;;
esac
