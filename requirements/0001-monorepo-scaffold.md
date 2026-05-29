# R-0001 — Monorepo scaffold and CI gates

- **Status:** Met
- **Milestone:** M1
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-05-26
- **Depends on:** none (M0 closure)
- **Realized by:** SPEC-0001 (to be written once this R is `Accepted`)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

The repository hosts a **two-stack monorepo**: a Rust backend workspace under
`/backend` running a minimal Axum HTTP service that exposes `GET /health → 200`,
and a Flutter app under `/mobile` that renders a placeholder fitAI screen.
A `Dockerfile` produces a runnable backend image. A GitHub Actions workflow
runs format-check, lint, test, and analyze gates on every push and on every
PR to `main`, and is green on the R-0001 branch.

This requirement is foundational: every later requirement assumes the
workspaces exist and the gates run.

## 2. Rationale

R-0001 is the smallest possible thing that proves the wizzielyn methodology's
merge gates ([`CLAUDE.md`](../CLAUDE.md) §5) are real. Without it the
`architect` and `qa` agents have no concrete commands to enforce, and every
later spec would re-litigate basic scaffolding choices. Solving it once, here,
unblocks every other R.

## 3. Acceptance criteria

Each criterion is observable from a checkout of the R-0001 branch with only
the standard toolchain installed.

- **AC1.** `/backend/Cargo.toml` declares a workspace with at least one crate
  (the API service). Running `cargo build --workspace --all-targets` from
  `/backend` exits 0.
- **AC2.** Running `cargo test --workspace --all-features` from `/backend`
  exits 0 and includes at least one test that boots the HTTP service
  in-process and asserts `GET /health → 200`.
- **AC3.** Running `cargo clippy --workspace --all-targets --all-features --
  -D warnings` from `/backend` exits 0 — no warnings.
- **AC4.** Running `cargo fmt --all -- --check` from `/backend` exits 0.
- **AC5.** `/mobile/pubspec.yaml` exists. Running `flutter analyze` and
  `flutter test` from `/mobile` both exit 0, with at least one widget test
  rendering the placeholder fitAI screen.
- **AC6.** Running `dart format --set-exit-if-changed .` from `/mobile`
  exits 0.
- **AC7.** `/backend/Dockerfile` builds via `docker build -t fitai-backend .`
  with no errors. Running the resulting image and issuing `GET /health`
  against the mapped port returns 200.
- **AC8.** `.github/workflows/ci.yml` runs AC1–AC6 on push to any branch and
  on PRs targeting `main`. The CI status on the R-0001 branch is green at
  the time of QA sign-off.

## 4. Constraints & non-goals

**In scope**
- The two workspaces, the placeholder endpoint, the placeholder Flutter
  screen, the Dockerfile, the CI workflow, and the `rust-toolchain.toml` /
  Flutter pin needed for reproducible builds.

**Out of scope (deferred to the requirements that own them)**
- User model, authentication, sessions, JWT — **R-0002**.
- User profile CRUD — **R-0003**.
- Any database connection, schema, or migration tooling — **R-0004 onward**.
- Release-style mobile builds (signed APK, IPA, App Bundle) — **R-0025 / R-0026**.
- Cloud deployment, registry pushes, secrets management — **R-0026**.
- Linting / formatting rules beyond the toolchain defaults; bespoke rules
  arrive when a spec needs them.

## 5. Open questions

None remaining. OQ1, OQ2, and OQ5 were settled in chat (Axum, pinned stable
toolchain, analyze + test only for mobile CI). OQ3 (fvm) and OQ4 (Swatinem +
subosito defaults) are recorded in the decision log below; once the owner
acknowledges this Draft, status flips to `Accepted` and SPEC-0001 may begin.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-05-26 | **Monorepo layout.** `/backend` (Rust workspace) + `/mobile` (Flutter app), plus the top-level methodology set. | Single PR review, single CI, simpler solo-dev workflow; matches the source-doc architecture diagram. Mirrored into `project-specifics.md` → Cross-cutting. |
| 2026-05-26 | **HTTP framework: Axum.** | Tokio-team-backed, tower-middleware ecosystem, idiomatic with `sqlx` + `serde`, most active community as of 2026. Owner-approved (OQ1). |
| 2026-05-26 | **Rust toolchain pinned in `rust-toolchain.toml`** to a specific stable version at scaffold time (exact version recorded in the spec / commit). | Reproducible builds; toolchain bumps become explicit PRs rather than CI surprises. Owner-approved (OQ2). |
| 2026-05-26 | **Mobile CI scope: `flutter analyze` + `flutter test` only.** | Fast (~1 min), no Android SDK install in CI; release-style builds belong to R-0025 / R-0026. Owner-approved (OQ5). |
| 2026-05-26 | **Flutter SDK pin: `fvm`** with the version committed to `.fvm/fvm_config.json`. | Reproducibility parity with the Rust toolchain pin; CI installs the same SDK as devs. Default accepted (OQ3). |
| 2026-05-26 | **CI caching: `Swatinem/rust-cache@v2` + `subosito/flutter-action@v2` defaults.** | Well-maintained, the conventional choice, no bespoke cache logic to maintain. Default accepted (OQ4). |

## Changelog

- _2026-05-26 — created (Draft); decisions OQ1–OQ5 recorded._
- _2026-05-27 — owner acked acceptance criteria; status → Accepted. SPEC-0001 may begin._
- _2026-05-28 — qa step 7 PASS; every AC1–AC8 has at least one passing test artifact (local run-all.sh + green CI on PR #1, both push and pull_request runs). Status → Met._
