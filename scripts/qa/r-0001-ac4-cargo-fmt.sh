#!/usr/bin/env bash
# R-0001 AC4 — Rust formatting clean.
#
# Verifies: `cargo fmt --all -- --check` exits 0.
# Pre-implementation red state: `cd backend` fails (no backend/ dir).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "[r-0001-ac4] cargo fmt --all -- --check"

cd "$REPO_ROOT/backend"
cargo fmt --all -- --check

echo "[r-0001-ac4] PASS"
