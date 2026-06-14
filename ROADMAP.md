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
| R-0006 | Photo session: multipart upload through the API to an `ObjectStore` seam (local-fs now, S3 at R-0026), metadata in Postgres; flexible photo list w/ optional angle; owner-only byte download; cross-user → 404 | SPEC-0006 | Done |

### M3 — Flutter MVP (thin client)

The user can do everything M1–M2 expose, from a phone. The milestone opens with
the app architecture + auth shell (R-0007), then each feature logger is a thin
screen on top of it.

> **Differentiator fast-track (owner decision, 2026-06-10).** The build order is
> re-sequenced onto the path that ships the product's differentiator soonest:
> **R-0009** (live workout logger, designed as a program-aware session driver) →
> **R-0006** (photo backend) → **R-0012** (archetype library curated from famous
> athletes' documented routines + diets, with provenance; curator: Claude,
> approver: owner) → **R-0013/R-0014** (photo → pose-estimation frame features →
> archetype → proposed program + diet, user picks a target) → **R-0027**
> (earbud-guided training: TTS voice-out + earbud-button control, no speech
> recognition in v1 — the phone stays pocketed). R-0010 (nutrition UI) and
> R-0011 (dashboard) are deferred until after the chain. Consequences: parts of
> M6 pose-estimation are pulled forward into R-0013, and real device builds
> (`flutter create .`, on-device audio testing) are pulled forward from R-0025
> for R-0027. Famous names stay internal labels; user-facing archetype names are
> abstracted (likeness/legal, see M8).

| Req | Capability | Spec | Status |
|-----|------------|------|--------|
| R-0007 | Flutter app architecture & auth shell: register/login, JWT in secure storage, Riverpod state, configurable HTTP client, router auth-gate (no feature UI) | SPEC-0007 | Done |
| R-0008 | Onboarding flow: dismissible home prompt + multi-step wizard (body stats, goals, optional details) over `PUT /profile/me`; training history deferred (no backend field) | SPEC-0008 | Done |
| R-0009 | Live workout logger: program-aware in-gym session driver (start → add exercise via preset picker + free text → log sets → finish → `POST /workouts`); sessions list + delete; full edit deferred. The substrate R-0027 drives by voice | SPEC-0009 | Done |
| R-0010 | Nutrition logger UI (manual entry first) — deferred until after the fast-track chain | SPEC-0010 | Backlog |
| R-0011 | Dashboard: trends, current program, weekly plan — deferred until after the fast-track chain | SPEC-0011 | Backlog |
| R-0027 | Earbud-guided training: the app speaks the session (next exercise, sets, weight) via TTS; the earbud media button advances/confirms; background audio with the phone pocketed. v1 is voice-OUT only (no speech recognition). Depends on R-0009 + R-0014 | SPEC-0027 | Backlog |

> **Progress-photo capture** (fixed-angle prompts) was the former R-0010; it is
> **blocked on the photo-session backend (R-0006)** and re-homed onto that gate —
> it re-enters the backlog with a fresh id once R-0006 is `Done`, sequenced
> alongside the M6 photo pipeline. Note R-0006 itself is now pulled forward by
> the fast-track (it feeds photo→archetype matching, R-0013).

### M4 — Archetype prior & initial program  ·  *pulled forward (fast-track)*

Bootstrap personalization before any per-user logs exist. The archetype is the
**prior**; per-user logs drive the posterior (M5). Famous-athlete data seeds the
prior library only — it must never feed the M5 response model (genetic /
enhancement confounders).

| Req | Capability | Spec | Status |
|-----|------------|------|--------|
| R-0012 | `ArchetypeLibrary` schema + curated seed data: documented routines **and diets** of famous bodybuilders/athletes (Mentzer, Arnold, Columbu, Yates '96, Cutler, Heath, …) with frame profile, program template, diet template, and provenance (documented vs folklore). Claude curates; owner approves each record. Names internal-only | SPEC-0012 | Done |
| R-0013 | Archetype-matching service: uploaded photo → server-side pose-estimation frame features (shoulder/hip ratio, limb proportions — pulled forward from R-0018/R-0019) → closest archetype | SPEC-0013 | Done |
| R-0014 | Generate proposed program **+ diet** from the matched archetype; present 2–3 targets and the user chooses which to follow | SPEC-0014 | Backlog |

### M5 — ML inference (Phase 1, `linfa`)

Move from heuristic adjustment to learned adjustment from real logs.

| Req | Capability | Spec | Status |
|-----|------------|------|--------|
| R-0015 | Time-series log aggregation (per user, per time window) | SPEC-0015 | Backlog |
| R-0016 | Response-inference model (linfa regression / trees): which inputs correlate with positive outcomes | SPEC-0016 | Backlog |
| R-0017 | Program adjustment engine: tweak volume / frequency / intensity / rest / macros | SPEC-0017 | Backlog |

### M6 — Photo pipeline & compliance

Add the visual signal and account for users who don't log consistently.
**Note:** the archetype-matching slice of pose estimation (frame features) is
pulled forward into R-0013; R-0018/R-0019 here cover the full pipeline depth
(progress tracking over time, symmetry/muscle-belly features for the M5 model).

| Req | Capability | Spec | Status |
|-----|------------|------|--------|
| R-0018 | Pose-estimation pipeline (MediaPipe candidate; choice deferred to discussion). Frame-feature slice pulled forward into R-0013 | SPEC-0018 | Backlog |
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

**R-0012 — Archetype library** is **Done** — the heart of the differentiator,
the third fast-track requirement, completed the eight-step loop and merged via
PR #18 (squash `600b0c7`) on 2026-06-13. A curated **six-archetype library**
(Yates/Mentzer/Arnold/Columbu/Cutler/Heath — internal research labels) where the
data *is* the deliverable: each record carries a structured **frame profile**
(numeric shoulder-to-waist ratio + banded/enum descriptors + a controlled
`StructureTag` vocab — the shape R-0013's pose estimation matches against), a
**program template**, a **diet template**, and honest **provenance** (Yates/
Mentzer `documented`; Arnold/Columbu/Cutler/Heath `reconstructed` — no fabricated
precision). It ships as an **embedded typed-Rust** library in `core::archetype`
(validated once via `OnceLock`, no DB), exposed through an **authenticated read
API** (`GET /archetypes`, `GET /archetypes/:id`); the `ArchetypeResponse` DTO
**omits `internal_name` + `provenance.sources`** so famous names and research
sources never cross the wire (likeness/legal). `seed::all` discharges the
validating constructors with the single justified `expect` (architect finding 1,
option B), guarded by the SAC2 revalidation test so an invalid record fails the
build. Architect **APPROVE** on the implementation; qa **SIGN-OFF** on AC1–AC9
(39 new tests — 29 core unit + 10 api integration; 321 passing overall).
Requirement is `Met`; `SPEC-0012` is `Implemented`. This is the **prior** R-0013
matches a photo against and R-0014 instantiates a starting plan from. The
**prior-only guardrail** (the famous data must never feed the M5 response model)
is documentation + module boundary today; making it an executable lint/test is a
deferred note carried to the first M5 requirement (see R-0015).

**R-0006 — Photo-session backend** is `Done` (PR #16): session CRUD + multipart
photo upload through the API to an **`ObjectStore` seam** (`LocalObjectStore`
now, S3 at R-0026), bytes-first/row-second/compensate, cross-user **404 never
403** — the substrate R-0013's pose estimation reads. **R-0009 — Live workout
logger** is `Done` (PR #14): the live in-gym logger over the widget-independent
`SessionDriver` — the R-0027 earbud seam. **R-0008 — Onboarding flow** is `Done`
(PR #10/#11), introducing the shared `ApiException.fromDio` + `AsyncValue` shell.
**R-0007 — Flutter app shell** is `Done` (PR #6/#8 + hotfix #9). **R-0005 —
Nutrition** and **R-0004 — Workout** logs are `Done` (M2 — Logging core). With
**R-0001–R-0003** `Done`, **M1** is complete.

**The roadmap is re-sequenced onto the differentiator fast-track** (owner
decision, 2026-06-10 — see the M3 callout): live workout logger → photo backend
→ famous-athlete archetype library → photo→archetype matching + program/diet
proposal → **earbud-guided training (R-0027)**, the hands-free voice-out in-gym
experience that is the product's stated differentiator. Owner-resolved forks:
earbud v1 is **button + TTS voice-out only** (no speech recognition);
photo→archetype uses **real pose-estimation frame features** from day one;
archetype data is **Claude-curated, owner-approved**, with provenance flags and
internal-only famous names.

**R-0013 — photo→archetype matching** is **Done** — merged via PR #21 (squash
`438851a`) on 2026-06-14. An uploaded photo runs through **in-process MoveNet
Lightning** (fp32, Apache-2.0, 192 px, bundled via `include_bytes!`) using ONNX
Runtime (`ort 2.0-rc.12`); the keypoints derive a **`FrameFeatures`** (numeric
`shoulder_to_waist` ratio + optional banded clavicle/limb fields); a **weighted
nearest-neighbor** (`rank()`, weights 0.6 / 0.2 / 0.2, absent-field-skip +
renormalize, `f64::total_cmp` stable sort) over the in-memory library returns
all six archetypes ranked nearest-first at `POST /photo-sessions/:id/match`.
Wire privacy: `internal_name` / `sources` never cross the wire. Error contract:
`422 no_usable_photo`, `422 no_person_detected`, `422 degenerate_frame`, `404`
for missing/foreign sessions. Dependency-inverted `Arc<dyn PoseEstimator>` seam
mirrors R-0006's `Arc<dyn ObjectStore>` — the integration suite uses
`FakePoseEstimator`; one real-ONNX test (`pose_onnx.rs`) on a committed
public-domain fixture asserts a plausible ratio, catching silent preprocessing
regressions. Architect **APPROVE WITH NITS** (both nits fixed before PR).
QA **SIGN-OFF** AC1–AC9 (31 new tests — 11 pose unit + 11 matching unit + 8
endpoint integration + 1 real-ONNX; 355 passing overall). Requirement is `Met`;
`SPEC-0013` is `Implemented`. Deviation from spec: fp32 Lightning not fp16
Thunder — fp16 emits garbage on CPU kernels; documented in SPEC changelog.

Next focus is **R-0014 — program + diet generation from the matched archetype**
— the fast-track step that turns the matching result into an actionable proposal:
generate 2–3 program + diet target options from the top-ranked archetype(s), let
the user choose which to follow. Depends on R-0012 (`Done`) and R-0013 (`Done`).
