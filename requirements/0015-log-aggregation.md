# R-0015 — Training-Log Aggregation (per user, per time window)

- **Status:** Draft
- **Milestone:** M5 (ML inference — Phase 1)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-07-09
- **Depends on:** R-0004 (workout log), R-0005 (nutrition log),
                  R-0034 (body measurements), R-0014 (active program)
- **Realized by:** SPEC-0015 (to be written)
- **QA:** `qa` agent run scoped to this requirement

---

## 1. Statement

A pure aggregation layer that turns a user's raw logs — workout sessions,
nutrition logs, body measurements — into a typed, deterministic
**`TrainingSummary`**: per-lift strength trends, per-muscle-group weekly volume,
training adherence, and body-composition trend, over configurable time windows.
This is the **feature layer** that the adjustment engine (R-0017) and the future
learned model (R-0016) both consume. It computes *facts about the logs*; it makes
no recommendations and runs no model.

## 2. Rationale

M5 turns logs into adaptation. Before any engine (heuristic or learned) can act,
the raw log tables must become a compact, stable feature set. Centralizing this
in one pure, tested module means the heuristic and learned paths read the **same**
inputs, the mobile app can show the same trends, and the model later trains on
exactly what the engine sees. It needs **no new data** — it operates on logs we
already collect.

## 3. Acceptance criteria

- **AC1. Pure `core` module.** Aggregation lives in `fitai-core` as pure
  functions over already-fetched log data — no DB/HTTP/clock access inside the
  computation (today's date is injected). Same inputs + same window ⇒ identical
  `TrainingSummary` (deterministic; unit-testable without a database).
- **AC2. Per-lift strength.** For each exercise: estimated-1RM series (Epley
  `w × (1 + reps/30)`, best set per session), current e1RM, **trend slope**
  (kg/week over the window via linear fit), session count, and a **stall flag**
  (no e1RM improvement beyond a small epsilon over the last *N* sessions).
- **AC3. Per-muscle-group weekly volume.** Σ working sets and Σ tonnage
  (reps × weight) per muscle group per ISO week, plus the trailing-window mean.
  Untagged exercises are excluded (consistent with existing volume logic).
- **AC4. Adherence.** Distinct training days per week vs the active program's
  `days_per_week`, and a rolling adherence ratio over the window.
- **AC5. Body-composition trend.** Weight / body-fat% / lean-mass series with a
  slope over the window, derived from `body_measurements`.
- **AC6. Configurable windows.** Aggregation accepts an explicit window (e.g.
  last 4 / 8 / 12 weeks); windowing is a parameter, not hard-coded.
- **AC7. Sparse/empty-safe.** A user with 0–1 sessions (or no measurements)
  yields a well-formed summary with empty/null fields — never a panic or error.
- **AC8. Endpoint.** `GET /training-summary` (authenticated) returns the current
  user's summary for a default window; consumed by the mobile app and, later,
  R-0017. The handler fetches rows and calls the pure aggregator.
- **AC9. Tests.** Unit tests per metric (per-lift slope + stall, weekly volume,
  adherence, body-comp slope, empty-data) in `core`; a backend integration test
  for the endpoint (auth required, correct shape, empty-user case).
- **AC10. Scope guard — aggregation only.** No recommendations, no thresholds
  that imply a decision, no ML/model. Those are R-0017 (heuristic) and R-0016
  (learned).
- **AC11. Prior/posterior separation.** Archetype/famous-athlete data must never
  enter a per-user aggregate — the summary is built purely from the user's own
  logs (per the project brief: the archetype is the prior, the logs are the
  posterior).

## 4. Constraints & non-goals

- No new data capture; operates on existing R-0004/0005/0034 rows.
- No recommendations or model — a pure feature layer.
- Dependencies point inward: the aggregator is in `core`, the DB fetch stays at
  the API edge (CLAUDE.md §2/§6).

## 5. Open questions (deferred to SPEC-0015)

- **OQ-1:** Default window length and how many prior sessions define a "stall".
- **OQ-2:** Slope method — simple linear regression vs robust (Theil–Sen) given
  noisy, sparse gym data.
- **OQ-3:** Muscle-group taxonomy source (reuse the existing `MuscleGroup` enum).
- **OQ-4:** Is the summary cached/materialized (R-0015 table) or computed on
  request? (v1 leans compute-on-request; materialization is an optimization.)

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-09 | Aggregation is a pure `core` module, DB fetch at the edge | Testable without a DB; same features feed heuristic + learned engines. |
| 2026-07-09 | Do R-0015 before any engine | Both R-0017 and R-0016 need a stable feature set first. |
| 2026-07-09 | Feature layer only, no decisions | Keeps "what the logs say" separate from "what to change" (R-0017). |

## Changelog

- _2026-07-09 — created (Draft), pending owner sign-off of acceptance criteria._
