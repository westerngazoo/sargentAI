#!/usr/bin/env bash
# R-0001 AC8 (file shape) — .github/workflows/ci.yml exists and declares the
# three required jobs: rust, mobile, docker.
#
# Note: the "CI status on the R-0001 branch is green at sign-off" portion of
# AC8 is verified manually by the qa agent at step 7 via:
#   gh pr checks <PR_NUMBER>
# This script only verifies the workflow file's static shape.
#
# Pre-implementation red state: file does not exist.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CI_FILE="$REPO_ROOT/.github/workflows/ci.yml"

echo "[r-0001-ac8] checking $CI_FILE"

if [[ ! -f "$CI_FILE" ]]; then
    echo "[r-0001-ac8] FAIL: $CI_FILE does not exist"
    exit 1
fi

missing=()
for job in rust mobile docker; do
    # Match the job header anchored at two-space indent + jobname + colon.
    # Avoids false positives from `name: rust ...` lines or substring matches.
    if ! grep -E "^[[:space:]]{2}${job}:" "$CI_FILE" >/dev/null; then
        missing+=("$job")
    fi
done

if (( ${#missing[@]} > 0 )); then
    echo "[r-0001-ac8] FAIL: ci.yml is missing required job(s): ${missing[*]}"
    echo "[r-0001-ac8] (expected pattern: '  <jobname>:' under top-level 'jobs:')"
    exit 1
fi

echo "[r-0001-ac8] ci.yml exists and declares rust, mobile, docker"
echo "[r-0001-ac8] NOTE: branch-green portion of AC8 is a manual check:"
echo "[r-0001-ac8]       gh pr checks <PR_NUMBER>"
echo "[r-0001-ac8] PASS"
