#!/usr/bin/env bash
# R-0001 — convenience runner. Invokes every per-AC script + the in-line test
# commands for AC2 (cargo test) and AC5 (flutter analyze + test), then reports
# pass/fail per check at the end.
#
# Does NOT abort on the first failure; keeps going so the qa agent (and the
# implementer iterating step 5) see the full picture in one shot.

# Intentionally do NOT use `set -e` at the top level — we want to capture
# per-check exit codes. We do use `set -uo pipefail` for the rest of the safety
# net.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
QA_DIR="$REPO_ROOT/scripts/qa"

declare -a NAMES
declare -a RESULTS

run_check() {
    local name="$1"
    shift
    echo
    echo "==================== $name ===================="
    if "$@"; then
        NAMES+=("$name")
        RESULTS+=("PASS")
    else
        NAMES+=("$name")
        RESULTS+=("FAIL")
    fi
}

# AC1 — cargo build
run_check "AC1 cargo build" bash "$QA_DIR/r-0001-ac1-cargo-build.sh"

# AC2 — cargo test (runs the two health.rs integration tests)
run_check "AC2 cargo test" bash -c "cd '$REPO_ROOT/backend' && cargo test --workspace --all-features --locked"

# AC3 — clippy
run_check "AC3 cargo clippy" bash "$QA_DIR/r-0001-ac3-cargo-clippy.sh"

# AC4 — cargo fmt
run_check "AC4 cargo fmt" bash "$QA_DIR/r-0001-ac4-cargo-fmt.sh"

# AC5 — flutter analyze + flutter test
run_check "AC5 flutter analyze + test" bash -c "cd '$REPO_ROOT/mobile' && flutter analyze && flutter test"

# AC6 — dart fmt
run_check "AC6 dart fmt" bash "$QA_DIR/r-0001-ac6-dart-fmt.sh"

# AC7 — docker build + /health probe
run_check "AC7 docker /health" bash "$QA_DIR/r-0001-ac7-docker-health.sh"

# AC8 — ci.yml shape (branch-green is a separate manual check)
run_check "AC8 ci.yml shape" bash "$QA_DIR/r-0001-ac8-ci-status.sh"

echo
echo "==================== SUMMARY ===================="
overall=0
for i in "${!NAMES[@]}"; do
    printf "  %-30s  %s\n" "${NAMES[$i]}" "${RESULTS[$i]}"
    if [[ "${RESULTS[$i]}" != "PASS" ]]; then
        overall=1
    fi
done

echo
if (( overall == 0 )); then
    echo "RESULT: ALL PASS"
else
    echo "RESULT: FAIL — one or more checks did not pass"
fi
echo
echo "Reminder: AC8 also requires manual verification that the R-0001 PR"
echo "branch is green on GitHub. Run: gh pr checks <PR_NUMBER>"

exit "$overall"
