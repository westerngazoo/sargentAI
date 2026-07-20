# R-0017 — Heuristic Program-Adjustment Engine

- **Status:** Draft
- **Milestone:** M5 (ML inference — Phase 1)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-07-09
- **Depends on:** R-0015 (training-log aggregation — the `TrainingSummary` this
  reads), R-0014 (active program + diet — the thing being adjusted)
- **Realized by:** SPEC-0017 (to be written)
- **QA:** `qa` agent run scoped to this requirement

---

## 1. Statement

A deterministic, rules-based engine that reads a user's `TrainingSummary`
(R-0015) plus their active program/diet and produces a small set of
**adjustment suggestions** — e.g. *"Bench has stalled 3 sessions → deload it
~10% next session"*, *"Legs volume is well below your program's band → add a
set"*, *"You're averaging 2 of 4 planned days → consider a 3-day split"*. Each
suggestion carries a machine-readable change and a plain-language rationale.
**Suggestions only** — nothing is auto-applied; the user accepts or ignores
them. This is the "heuristic adjustment" the ROADMAP names as the stepping stone
to the learned model (R-0016).

## 2. Rationale

This is what turns the app from a logger into a coach *today*, on data we
already have — no training corpus needed. It also scaffolds R-0016: the learned
model later slots in behind the same suggestion interface, and each surfaced
suggestion + user response becomes a labeled example ("we suggested X → outcome
Y") for it to train on.

## 3. Acceptance criteria

- **AC1. Pure `core` engine.** `core::adjust` is a pure function
  `(TrainingSummary, program, diet) → Vec<Adjustment>` — deterministic, no I/O,
  no clock, unit-testable without a DB. Same layering as R-0015.
- **AC2. Typed suggestions.** `Adjustment { kind, target, change, rationale,
  severity }` where `kind` is a closed enum (e.g. `DeloadLift`,
  `ProgressLift`, `AddVolume`, `ReduceVolume`, `ReduceFrequency`,
  `MacroTweak`), `change` is machine-readable (numeric delta / new value), and
  `rationale` cites the triggering facts (lift name, slope, weeks observed).
  Never stringly-typed decisions.
- **AC3. v1 rule set** (each rule individually unit-tested, thresholds as named
  module constants):
  1. **Stall → deload.** A lift with `stalled == true` and ≥ `MIN_SESSIONS`
     in-window → suggest a one-session deload of `DELOAD_PCT` (~10%) on that
     lift.
  2. **Steady gain → progress.** A lift with positive slope ≥ `PROGRESS_SLOPE`,
     not stalled, whose sessions span ≥ `MIN_LIFT_SPAN_DAYS` (clustered
     same-week sessions don't count as a trend) → suggest the next load
     increment (`+2.5 kg` default).
  3. **Volume gap → add a set.** A muscle group trained in ≥ `MIN_WEEKS`
     distinct weeks whose mean weekly sets is below the **program's own
     volume-band floor** (rule skipped entirely for low-volume programs),
     while adherence is ≥ `ADHERENT` → suggest +1 weekly set for that group.
  4. **Chronic under-adherence → right-size.** Adherence ratio < `LOW_ADHERENCE`
     over the window → suggest reducing `days_per_week` toward the
     whole-window observed frequency (`ratio × target`, never below 2); silent
     if that wouldn't reduce the plan.
  5. **Scale drift vs diet intent → macro nudge.** Weight slope contradicting
     the diet's **typed intent** (a `DietIntent` field derived from the same
     goal branch as the kcal math — never parsed from strategy prose) beyond
     `WEIGHT_DRIFT` kg/week → suggest a `KCAL_STEP` (~10%) kcal adjustment in
     the correcting direction. Skipped when measurements are fewer than
     `MIN_BODY_POINTS` or span less than `MIN_WEIGHT_SPAN_DAYS`.
- **AC4. Bounded output.** At most `MAX_SUGGESTIONS` per run (highest-severity
  first); an empty list is valid ("keep doing what you're doing" — surfaced as
  such, not as silence).
- **AC5. Data-sufficiency guards.** Every rule declares its minimum data (e.g.
  sessions in-window, measurement count) and stays silent below it — sparse
  data must produce *fewer* suggestions, never noisier ones.
- **AC6. Suggestions only.** The engine never mutates the program, diet, or any
  log. Applying a suggestion is a separate, explicit user action (out of scope
  for v1 beyond the existing program screens).
- **AC7. Endpoint.** `GET /adjustments` (authenticated) — fetches the summary
  inputs (shared with R-0015's path), runs the engine, returns the suggestions
  with the window used. No-program users get an empty list with a clear reason.
- **AC8. Mobile surface.** A "Coach suggestions" card (Progress or program
  screen) listing each suggestion's plain-language rationale; empty state shows
  the positive "on track" message. Display-only in v1.
- **AC9. Tests.** Per-rule unit tests (trigger + non-trigger + boundary at the
  threshold), determinism test, bounded-output test, and an endpoint
  integration test (auth, shape, empty-user). Rule thresholds asserted via the
  named constants, not magic numbers.
- **AC10. Scope guards.** No ML/`linfa` (R-0016). No archetype/famous-athlete
  data — rules read only the user's own summary + their chosen program. No
  medical/health claims in rationale copy; phrasing stays training-practical.
  Reminders/notifications remain R-0036.

## 4. Constraints & non-goals

- Deterministic rules with named thresholds — tunable in one place, explainable
  in the UI ("why am I seeing this?").
- Not auto-periodization: v1 does not rewrite the program structure, only
  suggests bounded tweaks.
- The learned model (R-0016) later replaces/augments rule *selection*, not the
  suggestion interface — `Adjustment` is designed to outlive the heuristics.

## 5. Open questions (deferred to SPEC-0017)

- **OQ-1:** Exact threshold defaults (`DELOAD_PCT`, `LOW_ADHERENCE`,
  `WEIGHT_DRIFT`…) — propose in spec, tune in QA against seeded demo data.
- **OQ-2 (resolved, architect review):** a typed `DietIntent` field on
  `GeneratedDiet`, set in `instantiate` from the goal branch; prose is never
  parsed (all six templates say "surplus" even on fat-loss kcal).
- **OQ-3:** Does `GET /adjustments` reuse the R-0015 endpoint's fetch path or
  take a `TrainingSummary` internally? (Lean: shared fetch, engine takes the
  summary.)
- **OQ-4:** Severity model — 2 levels (info/action) or 3 (info/suggested/urgent).

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-09 | Heuristics before the learned model (R-0017 before R-0016) | Adaptive behavior now on existing data; generates the labeled outcomes R-0016 trains on. |
| 2026-07-09 | Suggestions-only, never auto-apply | Trust + safety: the user stays in command; also keeps v1 write-free. |
| 2026-07-09 | `Adjustment` interface designed to outlive the rules | R-0016 slots in behind the same type; mobile UI never changes. |

## Changelog

- _2026-07-09 — created (Draft), pending owner sign-off of acceptance criteria._
