#!/usr/bin/env bash
# Run the project's test suites on a laptop, with the two things CI provides
# that a laptop does not:
#
#   1. an ONNX Runtime to link `fitai-api` against. The workspace builds `ort`
#      without `download-binaries`, and only backend/Dockerfile supplies the
#      library, so a bare `cargo test --workspace` fails to link and runs
#      nothing at all;
#   2. a Postgres for the API's `#[sqlx::test]` integration tests. Without
#      DATABASE_URL they panic, and `cargo test` stops at the first failing
#      test binary, so the suites after it never run either.
#
# Usage:
#   scripts/dev/test.sh            # Rust workspace, then Flutter
#   scripts/dev/test.sh rust       # Rust workspace only
#   scripts/dev/test.sh flutter    # Flutter only
#
# Environment:
#   ORT_LIB_LOCATION   directory holding libonnxruntime for the version in
#                      backend/Dockerfile; searched for when unset
#   DATABASE_URL       Postgres for #[sqlx::test]; defaults to the dev compose
#                      database, which is started if it is not running

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BACKEND_DIR="$REPO_ROOT/backend"
MOBILE_DIR="$REPO_ROOT/mobile"
DEV_DATABASE_URL="postgres://fitai:dev@localhost:5432/fitai"

log() { echo "[test] $*"; }
fail() { echo "[test] FAIL: $*" >&2; exit 1; }

# The ONNX Runtime version the API image links; the local library must match.
ort_version() {
  sed -n 's/^ARG ONNXRUNTIME_VERSION=//p' "$BACKEND_DIR/Dockerfile" | head -1
}

# Print a directory holding a libonnxruntime of the pinned version, or fail.
find_onnxruntime() {
  local want dir lib
  want="$(ort_version)"
  [[ -n "$want" ]] || fail "could not read ONNXRUNTIME_VERSION from backend/Dockerfile"

  if [[ -n "${ORT_LIB_LOCATION:-}" ]]; then
    ls "$ORT_LIB_LOCATION"/libonnxruntime.* >/dev/null 2>&1 \
      || fail "ORT_LIB_LOCATION=$ORT_LIB_LOCATION has no libonnxruntime in it"
    echo "$ORT_LIB_LOCATION"
    return 0
  fi

  # A static library left in ort's download cache by an earlier build that
  # had `download-binaries` on. Only used when its version is the pinned one.
  for dir in "$HOME/Library/Caches/ort.pyke.io/dfbin" "${XDG_CACHE_HOME:-$HOME/.cache}/ort.pyke.io/dfbin"; do
    [[ -d "$dir" ]] || continue
    while IFS= read -r lib; do
      # Process substitution, not a pipe: under pipefail, `grep -q` stopping
      # early would SIGPIPE `strings` and fail a match that succeeded.
      if grep -qxF "$want" < <(strings "$lib" 2>/dev/null); then
        dirname "$lib"
        return 0
      fi
    done < <(find "$dir" -name 'libonnxruntime.a' 2>/dev/null)
  done

  fail "no ONNX Runtime $want found. Set ORT_LIB_LOCATION to a directory
       holding libonnxruntime for $want (the version backend/Dockerfile pins)."
}

postgres_healthy() {
  [[ "$(docker inspect -f '{{.State.Health.Status}}' fitai-postgres 2>/dev/null || echo missing)" == "healthy" ]]
}

# Start the dev compose Postgres when the tests are pointed at it.
ensure_postgres() {
  if [[ "$DATABASE_URL" != "$DEV_DATABASE_URL" ]]; then
    log "using DATABASE_URL from the environment; not managing Postgres"
    return 0
  fi
  if postgres_healthy; then
    return 0
  fi
  log "starting postgres (docker compose)..."
  (cd "$BACKEND_DIR" && docker compose up -d postgres)
  for _ in $(seq 1 30); do
    postgres_healthy && return 0
    sleep 1
  done
  fail "postgres not healthy after 30s (is Docker running?)"
}

run_rust() {
  local ort
  # `fail` inside the substitution exits only the subshell; stop here too.
  ort="$(find_onnxruntime)" || exit 1
  export DATABASE_URL="${DATABASE_URL:-$DEV_DATABASE_URL}"
  ensure_postgres
  log "rust: ONNX Runtime from $ort"
  log "rust: cargo test --workspace --all-features --no-fail-fast"
  (cd "$BACKEND_DIR" && ORT_LIB_LOCATION="$ort" cargo test --workspace --all-features --no-fail-fast)
}

run_flutter() {
  command -v flutter >/dev/null 2>&1 || fail "flutter not on PATH"
  local want have
  want="$(cat "$MOBILE_DIR/.flutter-version" 2>/dev/null || true)"
  have="$(flutter --version 2>/dev/null | awk 'NR==1{print $2}')"
  if [[ -n "$want" && "$have" != "$want" ]]; then
    log "flutter: local SDK is $have, the repo pins $want (CI uses the pin)"
  fi
  log "flutter: flutter test"
  (cd "$MOBILE_DIR" && flutter test)
}

cmd="${1:-all}"
rust_status="skipped"
flutter_status="skipped"

case "$cmd" in
all | rust | flutter) ;;
*) fail "unknown command '$cmd' (use: all, rust, flutter)" ;;
esac

if [[ "$cmd" == "all" || "$cmd" == "rust" ]]; then
  if run_rust; then rust_status="passed"; else rust_status="FAILED"; fi
fi
if [[ "$cmd" == "all" || "$cmd" == "flutter" ]]; then
  if run_flutter; then flutter_status="passed"; else flutter_status="FAILED"; fi
fi

echo
log "rust:    $rust_status"
log "flutter: $flutter_status"
[[ "$rust_status" != "FAILED" && "$flutter_status" != "FAILED" ]]
