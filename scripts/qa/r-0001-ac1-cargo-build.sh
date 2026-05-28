#!/usr/bin/env bash
# R-0001 AC1 — workspace builds.
#
# Verifies: `cd backend && cargo build --workspace --all-targets` exits 0.
# Pre-implementation red state: `cd backend` fails (no backend/ dir).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "[r-0001-ac1] cargo build --workspace --all-targets (cwd: $REPO_ROOT/backend)"

cd "$REPO_ROOT/backend"
cargo build --workspace --all-targets

echo "[r-0001-ac1] PASS"
