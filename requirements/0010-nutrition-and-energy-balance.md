# R-0010 — Nutrition Logging & Energy Balance

- **Status:** Draft
- **Milestone:** M5 (adaptive intelligence) — the intake half
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-08-25
- **Depends on:** R-0005 (nutrition log — the daily-total model this extends),
  R-0015 (body trend + aggregation, the falsification signal), R-0014
  (`GeneratedDiet` targets), R-0034/PR #52 (body measurements)
- **Realized by:** SPEC-0010 (to be written)
- **QA:** `qa` agent run scoped to this requirement
- **Supersedes:** the roadmap's R-0010 "Nutrition logger UI" row, which had no
  requirement file. Closes issues #45 and #59 (duplicates of each other).

---

## 1. Statement

Make food logging survivable, then answer the question the owner calls the
product's core: **am I eating more or less than I am burning?**

Two halves, in order:

1. **Log and see.** Per-meal entries with a day view: what you ate, running
   macro totals against your program's target, and the ability to fix or delete
   a mistake.
2. **Energy balance.** Calories eaten vs calories burned over a window, with
   expenditure **measured from the user's own bodyweight trend** rather than
   assumed from a population multiplier — and refusing to answer when the data
   cannot support one.

## 2. Rationale

Nutrition is currently write-only: logs go in and are never shown again. Worse,
the second meal of any day **cannot be logged at all** — `nutrition_logs` has
`UNIQUE (user_id, performed_on)`, so a second `POST` is a 409 that surfaced to
users as *"that email is already registered"*. Intake is therefore
systematically under-recorded by a factor of 2–4, which would poison any
energy-balance figure built on it.

Energy balance is also the one number in this app that is **falsifiable**. The
app holds 8 weeks of bodyweight trend, so a predicted deficit can be checked
against observed weight change. That makes it possible to *measure* expenditure
instead of guessing it — a claim almost no consumer calorie tracker can make.

## 3. Acceptance criteria

### Logging and the day view

- **AC1. Per-entry meals.** Each logged meal is its own record with a
  timestamp and an optional free-text label. The daily total remains available
  as a **derived rollup**, so R-0005's daily grain (which the ML model consumes)
  is unchanged. Logging a second meal on the same day succeeds.
- **AC2. Day view.** A screen shows a chosen day's meals, running macro totals,
  calories, and progress against the program's target, with day-to-day
  navigation. A day with no data is an empty day, never an error.
- **AC3. Correct a mistake.** A logged meal can be edited or deleted. Delete
  confirms and names the meal. Deleting one meal must never delete the day.
- **AC4. Targets are attributed.** Where a target came from is shown
  (`your program`), and the app never presents a target as fact when no program
  exists.
- **AC5. Logging is survivable.** Repeat-yesterday, the user's own most-logged
  meals, and food search by name with portion (the USDA lookup that today is
  reachable **only** by voice) are all available from the logging sheet.
  Adherence is the whole game with food logging; a macros-only form loses it.

### Energy balance

- **AC6. Measured, not assumed, expenditure.** Expenditure is back-calculated
  from logged intake and the bodyweight/lean-mass trend over the window. The
  population activity multiplier is a **labelled bootstrap** used only until a
  measurement exists, never the permanent answer. The user's implied activity
  factor is an output.
- **AC7. Never double-count training.** An activity multiplier already includes
  training. A per-session "calories burned" figure must not be added on top of
  one. v1 reports total expenditure only.
- **AC8. Never reuse the diet target as expenditure.** `GeneratedDiet
  .estimated_kcal` is already goal-adjusted (`TDEE × 0.80` for fat loss). Using
  it as maintenance makes intake equal expenditure by construction and the
  deficit vanish. Expenditure comes from a shared RMR/TDEE source, not from the
  diet target.
- **AC9. Refuse rather than fabricate.** Where inputs cannot support an answer,
  the response is a typed reason, never a number: no sex **and** no body fat;
  no intake logged; intake coverage below the floor; too few body points or too
  short a span. Partial results are never blended into a confident figure.
  (The R-0041 `null` load / R-0042 `InsufficientData` precedent.)
- **AC10. Partial days are detectable and disclosed.** A day logged only
  through lunch must not read as a low-calorie day. The day view discloses when
  a day looks incomplete, and an incomplete day is excluded from — or flagged
  in — the balance calculation. **This is a safety criterion, not a nicety:** a
  fabricated 2,000 kcal deficit at 10 a.m. is advice to under-eat.
- **AC11. No false precision.** Expenditure and balance are reported as a
  **range**, never a point, with a permanently visible caveat that expenditure
  is estimated. The honest reporting unit is the week, not the day — a single
  day's weight change is water. No per-day balance verdict.
- **AC12. Falsifiable.** The app compares predicted change against the observed
  bodyweight trend and surfaces the residual. It **reports** the discrepancy and
  its likely cause (under-logging ranked first, since it is the largest and most
  common bias); it does **not** silently re-tune the model, which would fit the
  model to the logging gap.

### Correctness

- **AC13. Fix the unstated-sex defect.** `sex: None → 0.0` is documented as a
  "conservative mid-point" and is not one (the midpoint of `+5` and `−161` is
  `−78`); `0.0` is effectively the male value, giving a woman who declined the
  field ~250 kcal/day of expenditure that does not exist — ~1.8 kg of phantom
  8-week fat loss. Refuse, or use a stated value, but stop calling `0.0` a
  midpoint.
- **AC14. Fix the timezone boundary.** The client sends a device-local date
  while the server validates against UTC, so any user east of UTC logging in
  the early morning is rejected with a future-date error. Day boundaries must
  agree.
- **AC15. Tests.** Per-entry CRUD incl. multiple meals per day; ownership
  (404-never-403) and auth (401); the day view's empty/partial/complete states;
  each `Unknown` reason; the measured-expenditure path against a known
  fixture; the double-count guard; range output rather than point; and the
  partial-day exclusion.

## 4. Constraints & non-goals

- **Out of scope:** per-session calories burned (no honest basis without
  duration, and it double-counts — AC7); wearable/step integration; barcode
  scanning; recipes or multi-ingredient meals; a food database beyond the
  existing USDA lookup; auto-adjusting the program from intake (that is
  R-0017's territory and a later requirement); weekly rollups beyond what AC11
  requires.
- Expenditure must have **one** authority in `core`, shared with the program's
  diet generation — not a second, divergent TDEE definition.

## 5. Open questions (deferred to SPEC-0010)

- **OQ-1:** RMR equation selection — Katch-McArdle when body fat is known,
  Mifflin-St Jeor otherwise? (Lean: yes. Mifflin errs ~280 kcal/day on a
  muscular user, which is this app's modal user given the archetype library.)
- **OQ-2:** The intake-coverage floor for the measured path (proposed 0.80 of
  window days). No precedent exists in the codebase; this is a fresh constant.
- **OQ-3:** How is "this day looks incomplete" decided — time since last entry,
  a user-set day boundary, or an explicit "day complete" action?
- **OQ-4:** Does the balance strip ship with the day view, or in a second slice
  once the model is agreed? (The logging half stands alone and is the
  survivable-adherence work.)
- **OQ-5:** Meal label: free text with suggested chips, or an enum?
  (Lean: free text — breakfast/lunch/dinner does not fit *comida*/*cena* in the
  target market.)
- **OQ-6:** Fate of the existing day-level `POST /nutrition` and
  `DELETE /nutrition/:id`, which the voice path writes through today.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-25 | Per-entry meals with a derived daily rollup | Owner decision, revised after review: the originally chosen accumulate-into-one-row model cannot detect a partial day, which blocks AC10 — a safety criterion for the headline feature. The rollup keeps R-0005's ML grain intact. |
| 2026-08-25 | Expenditure measured from the bodyweight trend, not an activity multiplier | Owner priority is "spent vs ate"; back-calculation needs no multiplier guess and is falsifiable against data the app already holds. |
| 2026-08-25 | No per-session calories burned in v1 | The app records no session duration, and a multiplier already includes training — adding both double-counts. Capturing duration is worthwhile independently, but does not license a per-session burn figure. |
| 2026-08-25 | Ranges, not point estimates; weekly, not daily | Realistic resolution is ±150–250 kcal/day; a three-significant-figure daily number over-claims against the measurement floor. |
| 2026-08-25 | Report the residual, never silently re-tune | Auto-tuning RMR from a residual dominated by under-logging fits the model to the logging gap. |

## Changelog

- _2026-08-25 — created (Draft); supersedes the file-less roadmap R-0010 row._
