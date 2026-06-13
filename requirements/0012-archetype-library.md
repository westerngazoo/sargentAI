# R-0012 — Archetype library

- **Status:** Met
- **Milestone:** M4 (Archetype prior) — pulled forward by the differentiator fast-track
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-06-10
- **Depends on:** R-0002 (Done — auth, `AppState`, the `db`/error machinery) — for the read API only; the curated data itself depends on nothing
- **Realized by:** [SPEC-0012](../specs/0012-archetype-library.md) (Implemented)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

The system carries a curated **archetype library**: a small set of bodybuilder/
athlete archetypes, each modelling a **frame profile** (the body structure it
suits), a **program template** (how that archetype trains), a **diet template**
(how it eats), and **provenance** (how well-documented the source is). The
library is the **prior** for personalization — R-0013 matches a new user's
photo-derived frame features to the nearest archetype, and R-0014 instantiates a
starting program + diet from it, before the user has logged enough data to
personalize further.

The first cut seeds **six** archetypes spanning distinct frames and training/diet
philosophies (Yates, Mentzer, Arnold, Columbu, Cutler, Heath — internal research
labels). The library is exposed through a **read-only API**. Famous-athlete data
seeds the **prior only**; it must never be used as training data for the M5
response model (these are PED-era genetic outliers — their *response* to training
is not a model for a real user's). User-facing names are **abstracted**; the
famous names stay internal.

## 2. Rationale

"Upload your photo and the AI gives you your archetype, then a routine and diet"
is the product's differentiator, and the archetype library is its knowledge base.
Without it, a new user with no logged data has nothing to personalize from. The
curated archetypes — the *prior* — bootstrap a credible starting program the
moment a user joins, keyed to their actual frame rather than a one-size split.

## 3. Acceptance criteria

- **AC1.** An **`Archetype` domain model** with: `internal_name` (research label,
  not user-facing), `display_name` (abstracted, user-facing), `summary`; a
  **structured `frame_profile`** — numeric ratios (e.g. shoulder-to-waist) **and**
  categorical descriptors (`height_band`, `clavicle_width`, `limb_length`,
  `build`, free `structure_tags`); a **`program_template`** (philosophy, split,
  weekly frequency per muscle, volume band, intensity/rep scheme, rest,
  progression); a **`diet_template`** (approach, calorie strategy, macro split,
  meal structure); **`provenance`** (a `confidence` of
  `documented`/`reconstructed`/`folklore`, source notes); and `goals_served`.
  The model is validated (parse-don't-validate): malformed records are rejected.
- **AC2.** The library is **seeded with the six approved archetypes** (Yates '92–97,
  Mentzer, Arnold '70s, Columbu, Cutler '00s, Heath '10s). Every seed record
  populates all sections and **validates**; each is researched and **owner-
  approved** before it lands.
- **AC3.** A read API lists the library: **`GET /archetypes`** → `200` + all
  archetypes (authenticated). **`GET /archetypes/:id`** → `200` one / `404`
  unknown.
- **AC4.** The **user-facing wire never exposes `internal_name`** (famous names
  are internal-only — likeness/legal) — only `display_name` and the abstracted
  content cross the wire. Source notes are likewise internal.
- **AC5.** **Prior-only guardrail.** Each record is flagged as the matching
  **prior**; the requirement and the data state explicitly that famous-athlete
  records must **never** feed the M5 response/training model. (Enforced by
  documentation + the data living in a prior-only module; M5 requirements will
  not read it.)
- **AC6.** The **`frame_profile` is matchable**: its numeric ratios are in a form
  R-0013 can compute a distance against (the pose-estimation output shape), and
  its categorical tags are a controlled, documented vocabulary.
- **AC7.** **Provenance is honest:** an archetype whose routine/diet is
  well-documented (e.g. Yates' *Blood & Guts*, Mentzer's books) is flagged
  `documented`; a reconstructed or folklore one is flagged accordingly — no
  fabricated precision presented as fact.
- **AC8.** **Tests:** unit tests for the domain model + validation; a test that
  **every seed record validates** and that the controlled vocabularies hold;
  integration tests for the read API (list, get-one, `404`, `401`). All gates
  green (`cargo fmt`/`clippy`/`test`/`build`).
- **AC9.** **No matching, no generation, no ML.** R-0012 is the *library*:
  schema + curated data + read API. Photo→archetype matching is R-0013; program/
  diet generation is R-0014; no M5 training. No mobile UI.

## 4. Constraints & non-goals

- **No archetype matching** (photo → nearest archetype) — R-0013.
- **No program/diet generation** from an archetype — R-0014.
- **No ML / no training** — the famous data is the prior, never a training set
  (AC5).
- **No mobile UI** — the read API only; a "choose your archetype" screen is a
  later M3/M4 requirement.
- **No user-authored or editable archetypes** — curated reference data, changed
  only via a reviewed code/data change (each record owner-approved).
- **No medical/coaching claims** — the templates are starting priors, not
  prescriptions; framing is "a starting point to personalize from."
- **No real athlete likeness/imagery** — text templates only; internal names
  never shipped.

## 5. Open questions

Settled in the step-1 discussion (owner, 2026-06-10):

- **OQ1 — `frame_profile` representation?** RESOLVED → **structured: numeric
  ratios + categorical tags** (matchable to R-0013's pose features). (AC1/AC6)
- **OQ2 — Seed roster?** RESOLVED → the **six** (Yates, Mentzer, Arnold,
  Columbu, Cutler, Heath); more later. (AC2)
- **OQ3 — Provenance + famous names?** RESOLVED → **full discipline**: a
  confidence flag + source notes + the prior-only guardrail + abstracted
  user-facing `display_name`s (internal names never shipped). (AC4/AC5/AC7)

Deferred to the SPEC-0012 design discussion (HOW): whether the curated records
live as **embedded repo data** (a reviewed Rust/JSON const — fits "owner approves
each record via PR") or a **seeded DB table**; the exact numeric-ratio set and
the controlled tag vocabulary; whether the read API is authenticated or public;
and the module boundary that enforces the prior-only guardrail.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-10 | **Structured `frame_profile` (numeric ratios + categorical tags).** | R-0013 matches a user's photo-derived pose features against these; numeric ratios make it a nearest-neighbor lookup, tags keep it human-readable. (OQ1) |
| 2026-06-10 | **Seed six archetypes spanning distinct frames × philosophies.** | Variety so a new user's frame has a meaningful nearest match; HIT-low-volume / high-volume-aesthetic / powerbuilding / mass / modern-precision are all represented. (OQ2) |
| 2026-06-10 | **Full provenance discipline: confidence flag, source notes, prior-only guardrail, abstracted display names.** | Honest about documented-vs-folklore; protects against likeness/legal exposure; keeps the famous data out of the M5 response model. (OQ3) |
| 2026-06-10 | **R-0012 is library-only (schema + curated data + read API); matching and generation are R-0013/R-0014.** | Keeps each fast-track step a coherent, reviewable slice. |
| 2026-06-10 | **No medical/coaching claims; templates are starting priors.** | Avoids prescriptive liability; matches the "personalize from a prior" model. |

## Changelog

- _2026-06-10 — created (Draft). The differentiator's knowledge base: a curated, provenance-flagged archetype library (frame profile + program + diet) seeded with six athletes. Three step-1 forks owner-resolved (structured frame profile; the six; full provenance discipline)._
- _2026-06-10 — **Accepted.** Owner accepted AC1–AC9 and chose to review all six curated records together before implementation. Next: step 2 — SPEC-0012 + architect design review._
- _2026-06-13 — **Met.** Eight-step loop completed and merged via PR #18 (squash `600b0c7`): architect **APPROVE**, qa **SIGN-OFF** on AC1–AC9. The embedded `core::archetype` library (validated `Archetype` model + the six owner-approved records via `seed::all`, exposed through `library()`/`find()`) plus the `api::archetype` read surface (`GET /archetypes`, `GET /archetypes/:id`) with an `ArchetypeResponse` DTO that omits `internal_name` + `provenance.sources` (AC4). 39 new tests (29 core unit + 10 api integration; 321 passing overall); gates green (`cargo fmt`/`clippy -D warnings`/`test`/`build`). `SPEC-0012` is `Implemented`._
