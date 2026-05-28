#!/usr/bin/env bash
# R-0001 AC3 — clippy clean with -D warnings (including pedantic + unwrap/expect/panic).
#
# Verifies: cargo clippy exits 0 with no warnings.
# Pre-implementation red state: `cd backend` fails (no backend/ dir).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "[r-0001-ac3] cargo clippy --workspace --all-targets --all-features --locked -- -D warnings"

cd "$REPO_ROOT/backend"
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

echo "[r-0001-ac3] PASS"
