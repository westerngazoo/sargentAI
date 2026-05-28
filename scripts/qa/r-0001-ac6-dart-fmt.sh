#!/usr/bin/env bash
# R-0001 AC6 — Dart formatting clean.
#
# Verifies: `dart format --set-exit-if-changed .` exits 0.
# Pre-implementation red state: `cd mobile` fails (no mobile/ dir).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "[r-0001-ac6] dart format --set-exit-if-changed ."

cd "$REPO_ROOT/mobile"
dart format --set-exit-if-changed .

echo "[r-0001-ac6] PASS"
