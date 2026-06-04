# Roadmap

The single source of truth for what is being built and in what order — for the
project named in `project-specifics.md`. Milestones group requirements; each
requirement is realized by one or more specs. Nothing moves without passing the
requirement loop in [`CLAUDE.md`](CLAUDE.md) §4.

## Status legend

`Backlog` → `Discussing` → `Spec'd` → `In progress` → `In review` → `Done`

## Milestones

### M0 — Foundation  ·  *complete*

Adopt the methodology and prepare the repository.

| Item | Status |
|------|--------|
| Methodology files in place (`CLAUDE.md`, `requirements/`, `specs/`, agents) | Done |
| `project-specifics.md` filled in | Done |
| `.gitignore` in place | Done |
| Source brief ingested at `docs/fitness_ai_project.md` | Done |
| Roadmap (this file) seeded with M0–M8 and R-0001–R-0026 | Done |
| Toolchain confirmed against real workspaces | Done — Rust 1.95.0 + Flutter 3.44.0, pinned in R-0001 |
| First requirement (R-0001) discussed | Done — full loop completed; merged via PR #1 |

### M1 — Backend skeleton, auth, profile

The two-stack monorepo exists, CI is green, and a user can be created and
authenticated against a real backend.

| Req | Capability | Spec | Status |
|-----|------------|------|--------|
| R-0001 | Monorepo scaffold: Rust workspace under `/backend`, Flutter app under `/mobile`, Docker base image, GitHub Actions CI green | SPEC-0001 | Done |
| R-0002 | User authentication (JWT HS256, 24h, argon2id; OAuth2 deferred to its own R; Postgres + sqlx introduced) | SPEC-0002 | Done |
| R-0003 | User profile CRUD (age, height, weight, goals, body stats) | SPEC-0003 | Done |

### M2 — Logging core

Server-side persistence of every signal the model will eventually consume.

| Req | Capability | Spec | Status |
|-----|------------|------|--------|
| R-0004 | Workout log: exercises, sets, reps, weight, RPE — model + REST endpoints | SPEC-0004 | Done |
| R-0005 | Nutrition log: protein/carbs/fat/calories — model + REST endpoints (manual entry only; barcode scan deferred) | SPEC-0005 | Done |
| R-0006 | Photo session: four fixed angles, upload to S3-compatible storage, metadata in Postgres | SPEC-0006 | Backlog |

### M3 — Flutter MVP (thin client)

The user can do everything M1–M2 expose, from a phone. The milestone opens with
the app architecture + auth shell (R-0007), then each feature logger is a thin
screen on top of it.

| Req | Capability | Spec | Status |
|-----|------------|------|--------|
| R-0007 | Flutter app architecture & auth shell: register/login, JWT in secure storage, Riverpod state, configurable HTTP client, router auth-gate (no feature UI) | SPEC-0007 | Done |
| R-0008 | Onboarding flow (body stats, goals, training history) | SPEC-0008 | Backlog |
| R-0009 | Daily workout logger UI | SPEC-0009 | Backlog |
| R-0010 | Nutrition logger UI (manual entry first) | SPEC-0010 | Backlog |
| R-0011 | Dashboard: trends, current program, weekly plan | SPEC-0011 | Backlog |

> **Progress-photo capture** (fixed-angle prompts) was the former R-0010; it is
> **blocked on the photo-session backend (R-0006)** and re-homed onto that gate —
> it re-enters the backlog with a fresh id once R-0006 is `Done`, sequenced
> alongside the M6 photo pipeline.

### M4 — Archetype prior & initial program

Bootstrap personalization before any per-user logs exist.

| Req | Capability | Spec | Status |
|-----|------------|------|--------|
| R-0012 | `ArchetypeLibrary` schema + curated seed data (Mentzer, Arnold, Columbu, Yates, …) | SPEC-0012 | Backlog |
| R-0013 | Archetype-matching service: new user → closest archetype | SPEC-0013 | Backlog |
| R-0014 | Generate initial program from matched archetype | SPEC-0014 | Backlog |

### M5 — ML inference (Phase 1, `linfa`)

Move from heuristic adjustment to learned adjustment from real logs.

| Req | Capability | Spec | Status |
|-----|------------|------|--------|
| R-0015 | Time-series log aggregation (per user, per time window) | SPEC-0015 | Backlog |
| R-0016 | Response-inference model (linfa regression / trees): which inputs correlate with positive outcomes | SPEC-0016 | Backlog |
| R-0017 | Program adjustment engine: tweak volume / frequency / intensity / rest / macros | SPEC-0017 | Backlog |

### M6 — Photo pipeline & compliance

Add the visual signal and account for users who don't log consistently.

| Req | Capability | Spec | Status |
|-----|------------|------|--------|
| R-0018 | Pose-estimation pipeline (MediaPipe candidate; choice deferred to discussion) | SPEC-0018 | Backlog |
| R-0019 | Derived photo features (shoulder-width proxy, muscle-belly visibility, symmetry) fed into the main model | SPEC-0019 | Backlog |
| R-0020 | Compliance tracking: detect logging gaps, weight model confidence accordingly | SPEC-0020 | Backlog |

### M7 — Subscription & monetization

Turn it into a SaaS.

| Req | Capability | Spec | Status |
|-----|------------|------|--------|
| R-0021 | Subscription plans + billing integration (Stripe candidate) | SPEC-0021 | Backlog |
| R-0022 | Freemium feature gating (manual logging free; adaptive AI + photo analysis paid) | SPEC-0022 | Backlog |
| R-0023 | Subscription paywall UI in Flutter | SPEC-0023 | Backlog |

### M8 — Launch readiness

Everything needed to ship to the public.

| Req | Capability | Spec | Status |
|-----|------------|------|--------|
| R-0024 | Privacy policy + health-data compliance (LATAM + GDPR-adjacent rules) | SPEC-0024 | Backlog |
| R-0025 | App Store + Play Store accounts, metadata, screenshots | SPEC-0025 | Backlog |
| R-0026 | Production deployment: AWS *or* Azure, managed Postgres, S3/Blob, CI promotion | SPEC-0026 | Backlog |

## Deferred (not yet on a milestone)

Pulled from the source doc's "Open Questions / TODOs"; will be promoted to
R-files when their parent milestone is the focus.

- Barcode scan for nutrition (extension of R-0005 / R-0009)
- OAuth2 social login (extension of R-0002)
- Self-hosted VPS migration for unit-economics (Phase 2 of M8)
- GPU inference path (only if R-0016 / R-0018 demand it)
- Phase-2 ML stack: `burn` or `tch-rs` for sequential / time-series modelling

## Sequencing rules

- A requirement enters `Discussing` only when every requirement it depends on is
  `Done`.
- Requirement and spec ids are 4-digit and shared in spirit: `R-0001` is
  realized by `SPEC-0001` unless a requirement needs several specs.
- This file is updated by the orchestrator whenever a requirement changes state.

## Current focus

**R-0007 — Flutter app architecture & auth shell** is **Done** — the project's
first mobile requirement completed the eight-step loop and landed on `main`. It
turns `/mobile` from a hello-world into an authenticated client skeleton:
register/login against the R-0002 endpoints, JWT in platform secure storage, a
Riverpod session as the single source of truth, a Dio client with a 401
re-login interceptor, a `go_router` auth gate, and a build-time-configurable API
base URL — **no feature logger UI** (that is R-0008+). Architect **APPROVE**; qa
**SIGN-OFF** on AC1–AC11 with the `test/` suite green (49/49). Requirement is
`Met`; `SPEC-0007` is `Implemented`. AC10's `integration_test` is authored +
compile-checked; running it in a gate is deferred to **R-0025** (it needs the
platform folders R-0007 deliberately omits).

> Landed via a hotfix: PR #6/#8 first merged the branch at a stale commit that
> predated the test-suite fix (a `pumpShell` fake-async hang + bare-handler
> interceptor tests), turning `main` red. The hotfix restored the green 49/49
> suite, amended AC10 to match the e2e's real gate coverage, and completed the
> step-8 tracking.

Predecessors **R-0005 — Nutrition log** and **R-0004 — Workout log** are `Done`
(M2 — Logging core: workout + nutrition signals for the M5 ML engine). With
**R-0001–R-0003** `Done`, **M1** is complete.

Next focus is the first **M3 feature logger — R-0008 (Onboarding flow)** — now
unblocked: it builds directly on the R-0007 shell and the R-0003 profile
endpoints. The **M2 photo-session backend (R-0006)** remains open and gates the
later progress-photo capture screen.
