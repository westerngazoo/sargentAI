# QA Test Plan — R-0001 Monorepo scaffold and CI gates

- **Requirement:** [R-0001 — Monorepo scaffold and CI gates](../../requirements/0001-monorepo-scaffold.md)
- **Spec:** [SPEC-0001](../../specs/0001-monorepo-scaffold.md)
- **QA author:** qa agent (loop step 3 — Test planning)
- **Date:** 2026-05-28
- **Loop step:** 3 (test plan, pre-implementation; tests must be red)

## 1. Scope

R-0001 has eight acceptance criteria (AC1–AC8). SPEC-0001 §6 (SAC1–SAC8) maps
each one to a concrete verification command or test. This document derives the
test artifacts the qa agent will run at sign-off (step 7) and asserts the
pre-implementation TDD-red state expected today.

Two kinds of verification artifact are used:

- **Code-level tests** (Rust integration test, Dart widget test) — committed
  under `backend/crates/api/tests/` and `mobile/test/`. The implementer's
  source code makes them green; today they fail to compile (red).
- **Shell scripts** under `scripts/qa/` — one per AC whose verification is a
  shell invocation (build, lint, format-check, container probe, CI file shape).
  Today they fail because their preconditions (the workspaces, the Dockerfile,
  the CI file) don't exist yet (red).

The shell scripts are intentionally simple and idempotent: any developer or
agent can run a single script to verify one AC in isolation, or
`scripts/qa/run-all.sh` to run them all.

## 2. AC → test mapping

| AC | Verifies | Test artifact (path) | Pre-impl state (expected red) | Post-impl state (expected green) |
|----|----------|----------------------|-------------------------------|----------------------------------|
| AC1 | Workspace builds; `cargo build --workspace --all-targets` exits 0. | `scripts/qa/r-0001-ac1-cargo-build.sh` | Fails — `backend/` does not exist (`cd` errors). | Exits 0 against scaffolded workspace. |
| AC2 | At least one test boots the HTTP service in-process and asserts `GET /health → 200`. | `backend/crates/api/tests/health.rs` (two `#[tokio::test]` functions: `health_returns_ok_via_router` and `health_returns_ok_via_real_server`), driven by `cargo test --workspace --all-features --locked` in `scripts/qa/run-all.sh`. | Fails to compile — `fitai_api` crate does not exist. | Both tests pass; second one literally binds `127.0.0.1:0` and serves via `axum::serve`. |
| AC3 | Clippy clean with `-D warnings` (pedantic + unwrap/expect/panic lints active). | `scripts/qa/r-0001-ac3-cargo-clippy.sh` | Fails — no workspace. | Exits 0 with no warnings. |
| AC4 | `cargo fmt --all -- --check` exits 0. | `scripts/qa/r-0001-ac4-cargo-fmt.sh` | Fails — no workspace. | Exits 0. |
| AC5 | `flutter analyze` + `flutter test` both exit 0; at least one widget test renders the placeholder. | `mobile/test/home_screen_test.dart`, driven by `flutter analyze && flutter test` in `scripts/qa/run-all.sh`. | Fails — no `mobile/` package, no `fitai` package import target. | `renders fitAI placeholder` passes; analyze clean. |
| AC6 | `dart format --set-exit-if-changed .` exits 0. | `scripts/qa/r-0001-ac6-dart-fmt.sh` | Fails — no `mobile/` dir. | Exits 0. |
| AC7 | `docker build` succeeds and the running image responds 200 on `/health`. | `scripts/qa/r-0001-ac7-docker-health.sh` (builds image, runs container on ephemeral host port, polls `/health` with retry budget, cleans up via `trap`). | Fails — no `backend/Dockerfile`. | Container starts, `/health` returns 200, cleanup succeeds. |
| AC8 | `.github/workflows/ci.yml` exists and contains the three required jobs (`rust`, `mobile`, `docker`). "Branch is green" portion is verified by manual `gh pr checks` at sign-off. | `scripts/qa/r-0001-ac8-ci-status.sh` for file shape; manual `gh pr checks $BRANCH` for branch-green. | Fails — file does not exist. | Script exits 0; sign-off run records `gh pr checks` green. |

## 3. Per-AC narrative

### AC1 — workspace builds

The shell script just invokes `cargo build --workspace --all-targets` from
`/backend`. `--all-targets` is load-bearing: it covers libs, bins, tests,
benches, and examples so an integration test that doesn't compile fails AC1,
not only AC2.

- **Edge cases considered:** none beyond "command exits 0" — AC1 is binary.
- **Deliberately not tested:** release-profile build, cross-compilation, MSRV
  drift. The toolchain pin in `rust-toolchain.toml` removes the MSRV variable.

### AC2 — `GET /health → 200` proved by a real boot

Two tests, both written here per SPEC-0001 §3.7:

1. `health_returns_ok_via_router` — the router via `tower::ServiceExt::oneshot`.
   Fast, no port binding. Guards regressions cheaply on every CI run.
   Additionally asserts the response body is empty (boundary on body shape).
2. `health_returns_ok_via_real_server` — binds `127.0.0.1:0`, spawns
   `axum::serve`, issues a real `reqwest::get`. This is the test that satisfies
   AC2's literal "boots the HTTP service" wording (SPEC-0001 §2.3, Architect
   Finding #4).

- **Edge cases considered:** ephemeral port (`:0`) avoids the port-collision
  failure mode that would mask a real bug; the test aborts the server task
  before returning so test leakage is bounded; the body-empty assertion in
  test 1 catches an accidental `(StatusCode::OK, "ok")` body that would still
  satisfy AC2's "returns 200" wording but violate the spec's minimal shape.
- **Deliberately not tested:** HTTPS, alternative HTTP methods on `/health`
  (only `GET` is routed; an unrouted method returns 405 — out of scope for
  R-0001), graceful-shutdown behaviour (no AC asserts it). Graceful shutdown
  *is* specced (SPEC-0001 §2.4) and will be exercised indirectly when R-0026
  adds container-lifecycle tests.

### AC3 — clippy clean with `-D warnings`

SPEC-0001 strengthens the lint set to include `clippy::pedantic`,
`clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic` (Architect
Finding #5). The script invokes clippy with `--locked` (Architect Finding #6)
so a stale `Cargo.lock` cannot silently regenerate.

- **Edge cases considered:** `--all-targets` ensures the tests themselves are
  clippy-clean (tests opt out of unwrap/expect/panic explicitly per §3.4/§3.7).
- **Deliberately not tested:** custom clippy config files, `clippy.toml`
  threshold tweaks — none are in scope for R-0001.

### AC4 — `cargo fmt --check`

One-line script. No edge cases.

### AC5 — `flutter analyze` + `flutter test`

The widget test is lifted verbatim from SPEC-0001 §3.16: `pumpWidget` of
`HomeScreen` wrapped in `MaterialApp`, then `expect(find.text('fitAI'),
findsOneWidget)`. Driven by `run-all.sh` since AC5 is two commands, not one.

- **Edge cases considered:** the test wraps `HomeScreen` in `MaterialApp`
  rather than using `FitAiApp` so the widget under test isn't coupled to the
  app shell — a future change to `app.dart` doesn't break this test for the
  wrong reason.
- **Deliberately not tested:** golden images, integration tests on a device,
  release-mode builds — all explicitly out of scope per R-0001 §4.

### AC6 — `dart format --set-exit-if-changed`

One-line script. No edge cases.

### AC7 — Docker image health probe

The script:

1. `docker build -t fitai-api:qa-$RANDOM backend`.
2. `docker run -d -p 0:8080 fitai-api:qa-$RANDOM` (ephemeral host port) and
   captures the container id.
3. Resolves the mapped host port via `docker port`.
4. Polls `curl -sf -o /dev/null -w "%{http_code}" http://127.0.0.1:$PORT/health`
   in a retry loop (15 attempts, 1 s between) — covers container-boot time and
   absorbs CI cold-start jitter without an arbitrary fixed sleep.
5. Asserts the final status is exactly `200`.
6. **Always** stops & removes the container and the image via `trap EXIT` so a
   failure halfway through still cleans up.

- **Edge cases considered:** port collision (use `0:8080` so the OS picks);
  partial cleanup (`trap EXIT` covers every exit path, including SIGINT during
  the retry loop); image-tag collisions across QA runs (per-run random tag).
- **Deliberately not tested:** image size, layer count, non-root user
  enforcement (specced in §2.5 but no AC requires it). Once R-0026 introduces
  registry push, those become its concerns.

### AC8 — CI workflow shape and branch-green

The script verifies file existence and the three required job names
(`rust:`, `mobile:`, `docker:`). It does **not** verify the branch is green —
"green at the time of QA sign-off" requires querying GitHub, which is a manual
sign-off check (qa agent runs `gh pr checks $PR_NUMBER` or visits the Actions
tab). The test plan calls this out explicitly so the gap is documented.

- **Edge cases considered:** job-name typos that would pass a substring grep
  (`rust-build`) — the script greps for the colon-anchored job header
  (`^  rust:`) under `jobs:` so partial matches don't pass.
- **Deliberately not tested:** workflow syntax validity (GitHub validates on
  push; failed validation prevents the branch from going green, which the
  manual `gh pr checks` step catches), cache key correctness, action version
  pins — covered by the workflow running successfully on the R-0001 branch.

## 4. Pre-implementation red state (today)

| Artifact | Failure mode (expected today) |
|----------|-------------------------------|
| `backend/crates/api/tests/health.rs` | `cargo` cannot compile it — there's no `backend/` workspace and no `fitai_api` crate to import. |
| `mobile/test/home_screen_test.dart` | `flutter test` cannot run it — there's no `mobile/pubspec.yaml` and no `package:fitai/screens/home_screen.dart`. |
| `scripts/qa/r-0001-ac1-cargo-build.sh` | `cd backend` exits non-zero. |
| `scripts/qa/r-0001-ac3-cargo-clippy.sh` | `cd backend` exits non-zero. |
| `scripts/qa/r-0001-ac4-cargo-fmt.sh` | `cd backend` exits non-zero. |
| `scripts/qa/r-0001-ac6-dart-fmt.sh` | `cd mobile` exits non-zero. |
| `scripts/qa/r-0001-ac7-docker-health.sh` | `docker build` exits non-zero — no `backend/Dockerfile`. |
| `scripts/qa/r-0001-ac8-ci-status.sh` | File-existence check fails — `.github/workflows/ci.yml` does not exist. |
| `scripts/qa/run-all.sh` | Aggregates all of the above; reports per-script fail. |

All of these failing is the legitimate TDD-red baseline. Implementation step
5 (SPEC-0001 §3 snippet commit) turns each one green.

## 5. Sign-off checklist (step 7)

When the implementation PR is open and the qa agent runs sign-off, it will:

1. `chmod +x /Users/goose/projects/fitAI/scripts/qa/*.sh` (one-time; can be
   left committed in the repo with the executable bit already set).
2. From the repo root, run `bash scripts/qa/run-all.sh`. Expected: every line
   reports `PASS` and the script exits 0.
3. Manually verify AC8 "branch is green":
   - `gh pr checks <PR_NUMBER>` against the R-0001 PR
   - All three required checks (`rust`, `mobile`, `docker`) must be `pass`
4. Cross-reference: every row in §2 maps to a passing artifact, every AC
   covered.
5. Produce the sign-off report per `.claude/agents/qa.md` "Sign-off report
   format" with verdict `PASS` (or `FAIL` with the failing AC ids and the
   captured output).

## 6. Operational notes for the implementer

- After cloning, run `chmod +x scripts/qa/*.sh` once if the executable bit
  didn't survive checkout (the files are written with shebangs but Git relies
  on the file-mode in the index; the qa agent set the bit at authoring time).
- `scripts/qa/run-all.sh` is the convenience driver. The per-AC scripts are
  intentionally split so the implementer can iterate AC-by-AC during step 5
  without re-running the slow ones (Docker build) each time.
- The Rust integration test and the Dart widget test are **frozen at the
  shapes in SPEC-0001 §3.7 and §3.16**. If implementation needs to change
  them, that's a spec change, not a test change — file an extension R.
