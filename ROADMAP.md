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
| R-0004 | Workout log: exercises, sets, reps, weight, RPE — model + REST endpoints | SPEC-0004 | Backlog |
| R-0005 | Nutrition log: protein/carbs/fat/calories — model + REST endpoints (manual entry only; barcode scan deferred) | SPEC-0005 | Backlog |
| R-0006 | Photo session: four fixed angles, upload to S3-compatible storage, metadata in Postgres | SPEC-0006 | Backlog |

### M3 — Flutter MVP (thin client)

The user can do everything M1–M2 expose, from a phone.

| Req | Capability | Spec | Status |
|-----|------------|------|--------|
| R-0007 | Onboarding flow (body stats, goals, training history) | SPEC-0007 | Backlog |
| R-0008 | Daily workout logger UI | SPEC-0008 | Backlog |
| R-0009 | Nutrition logger UI (manual entry first) | SPEC-0009 | Backlog |
| R-0010 | Progress photo capture with fixed-angle prompts | SPEC-0010 | Backlog |
| R-0011 | Dashboard: trends, current program, weekly plan | SPEC-0011 | Backlog |

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

**R-0003 — User profile CRUD** is **Done** — the full eight-step loop
completed and it merged to `main` via PR #3 (merge commit `cdf9f9e`) on
2026-05-30: architect APPROVE on both the design (step 2) and the
implementation (step 6), `qa` sign-off PASS verifying all of AC1–AC9, and all
CI gates green (rust fmt/clippy/test/build, docker build, mobile analyze/test).
Requirement is `Met`; `SPEC-0003` is `Implemented`. With R-0001, R-0002, and
R-0003 all `Done`, **M1 — Backend skeleton, auth, profile is fully complete**.

Next per the sequencing rules is **M2 — Logging core**, beginning with
**R-0004 — Workout log** (exercises, sets, reps, weight, RPE — model + REST
endpoints), currently `Backlog`. Its dependency R-0003 is now `Done`, so R-0004
may enter **step 1 (Discuss)**: owner + Claude agree the capability and its
acceptance criteria, then write `requirements/0004-workout-log.md`.
