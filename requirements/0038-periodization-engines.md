# R-0038 — Periodization Engines (structured, math-driven programs)

- **Status:** Draft
- **Milestone:** M4+ / training methodology
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-07-22
- **Depends on:** R-0015 (per-lift e1RM — the load anchor), R-0004 (workout log),
                  R-0012/0014 (archetype programs this generalizes)
- **Realized by:** SPEC-0038 (to be written)
- **QA:** `qa` agent run scoped to this requirement

---

## 1. Statement

A structured, time-indexed **periodized-program model** plus three deterministic
**periodization engines** — **Linear**, **Undulating (DUP)**, and **Block** —
that mathematically generate a multi-week training plan from parameters and the
user's current per-lift estimated 1RM. Loads are prescribed as **% of e1RM**
(the value R-0015 computes), so every set carries a concrete target weight.

This is the foundation for "different approaches to training": today a program is
only guidance strings + exercise names, with no time dimension. This makes a
program a real `weeks → sessions → exercises → sets{reps, %1RM, target kg}`
structure that engines populate.

## 2. Rationale

Serious training is periodized — intensity and volume vary over time on a plan,
not ad hoc. Encoding periodization as pure math (a) gives users real,
progressive programs with exact weights, (b) generalizes the app beyond the
static archetype week-1 plan, and (c) is the substrate the user-facing program
builder and shareable "trainer templates" (e.g. an Anatoly-style blueprint)
will later sit on. It reuses the deterministic-core-engine pattern proven by
R-0015/R-0017 — no ML, fully testable.

## 3. Acceptance criteria

- **AC1. Pure `core` module.** The model + engines live in `fitai-core` as pure
  functions — no I/O, no clock, deterministic (same params + e1RM map ⇒ same
  program). Unit-testable without a DB.
- **AC2. Structured model.** A `PeriodizedProgram` is
  `scheme` + `Vec<TrainingWeek>`; a week is `index` + `Vec<TrainingSession>`; a
  session is a `label` + `Vec<PrescribedExercise>`; an exercise is a lift name +
  `Vec<PrescribedSet>`; a set is `{ reps, intensity_pct, target_load_kg }`.
- **AC3. Load from e1RM.** `target_load_kg = round_to_plate(e1rm × intensity_pct)`
  using the lift's current e1RM from the supplied per-lift map. When a lift has
  no e1RM (new user), `target_load_kg` is `None` and the set still prescribes
  reps × %intensity. Plate rounding uses a configurable increment (default
  2.5 kg).
- **AC4. Linear engine.** Weekly progression: reps trend down, intensity trends
  up across the mesocycle (e.g. wk1 3×8 @70% → wk4 3×3 @85%), interpolated by a
  documented formula between start/end parameters.
- **AC5. Undulating (DUP) engine.** Intensity/rep scheme undulates **within** the
  week across sessions (e.g. a heavy/light/medium rotation), optionally with a
  small week-over-week intensity increment. Sessions in the same week differ.
- **AC6. Block engine.** Weeks are grouped into ordered blocks —
  **accumulation** (higher reps, moderate %), **intensification** (lower reps,
  higher %), **realization/peak** (few reps, top %) — each block progressing
  internally.
- **AC7. Parameterized, not hard-coded.** Each engine takes explicit parameters
  (weeks, sessions/week, the lifts to program, start/end intensity + rep bounds,
  scheme-specific knobs) with sane documented defaults; thresholds/curves are
  named constants, not magic numbers.
- **AC8. Valid, bounded output.** Every generated set has `reps ≥ 1`,
  `0 < intensity_pct ≤ 1.0`, weeks/sessions counts match the parameters, and the
  program is non-empty for valid inputs. Invalid params (0 weeks, empty lift
  list, start > end where the scheme requires ordering) return a typed error,
  never a panic.
- **AC9. Serializable.** `PeriodizedProgram` and all sub-types round-trip through
  JSON (they will cross the wire to the mobile app in a follow-up).
- **AC10. Tests.** Per-engine unit tests: shape (weeks/sessions/sets counts),
  the defining property (Linear monotonic intensity↑/reps↓; DUP within-week
  variance; Block phase ordering), load computation + plate rounding, the
  no-e1RM `None` path, and invalid-param errors. Determinism test; serde
  round-trip test.
- **AC11. Scope guard.** Pure model + engines only — **no** persistence, **no**
  endpoint, **no** mobile UI, **no** archetype rewiring, **no** user-builder in
  this requirement (all follow-ups). No ML.

## 4. Constraints & non-goals

- Deterministic math with named parameters — explainable and tunable.
- Loads anchor on e1RM (%1RM model, per the owner decision); RPE-based
  autoregulation is a possible later extension, not v1.
- **Follow-ups (separate requirements), explicitly out of scope here:**
  - `GET /programs/periodized` (or similar) endpoint + mobile display.
  - A `PeriodizationScheme` axis on the archetype/approach taxonomy.
  - The **user program builder** (create/save custom programs in-app) and
    **shareable trainer templates** ("Anatoly style").

## 5. Open questions (deferred to SPEC-0038)

- **OQ-1:** Exact interpolation formulas + default parameter values per engine
  (propose in spec, tune in QA).
- **OQ-2:** How the "lifts to program" are specified — free-form names vs a typed
  lift set; how a lift maps to its e1RM key (case-insensitive, per R-0015).
- **OQ-3:** Plate-rounding policy (single 2.5 kg increment vs per-side / lb
  support) — v1 default 2.5 kg.
- **OQ-4:** Accessory/secondary lifts — does v1 program only the main lifts, or
  also accessories at fixed rep ranges?

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-22 | Ship all three engines (Linear/DUP/Block) in the first slice | Owner choice — "different approaches" real from day one. |
| 2026-07-22 | Loads as %1RM off the tracked e1RM | Owner choice — concrete kg targets; reuses R-0015. |
| 2026-07-22 | Pure `core` model + engines first; endpoint/UI/builder are follow-ups | Bottom-up: the builder and display can't exist without the model + math. |

## Changelog

- _2026-07-22 — created (Draft); acceptance criteria reflect the owner's two
  scope decisions (all three engines, %1RM model)._
