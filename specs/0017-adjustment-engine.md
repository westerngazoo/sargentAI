# SPEC-0017 — Heuristic program-adjustment engine (`core::adjust`)

- **Status:** Draft
- **Realizes:** R-0017
- **Author:** Claude (main session)
- **Created:** 2026-07-09
- **Depends on:** SPEC-0015 (`TrainingSummary` and friends — the input),
  SPEC-0014 (`GeneratedProgram`, `GeneratedDiet` — the thing being adjusted),
  SPEC-0002 (`AuthenticatedUser`, `AppState`).
- **Module(s):** `backend/crates/core/src/adjust.rs` (new — pure engine);
  `backend/crates/api/src/summary/` (extend — `GET /adjustments` beside the
  R-0015 endpoint, sharing its fetch path); mobile Coach card is a follow-up PR
  within R-0017.

## 1. Motivation

Realizes [R-0017](../requirements/0017-adjustment-engine.md): deterministic
rules over the R-0015 feature layer that emit typed, explainable **suggestions**
— the app's first adaptive behavior, and the interface the learned model
(R-0016) later slots behind.

## 2. Design

### 2.1 Types

```rust
/// What a suggestion proposes, machine-readably. A closed vocabulary —
/// `change` is data, never prose.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Change {
    DeloadLift { lift: String, pct: f64 },            // reduce next load by pct
    ProgressLift { lift: String, add_kg: f64 },       // next increment
    AddWeeklySets { group: MuscleGroup, sets: u32 },
    ReduceDaysPerWeek { from: u8, to: u8 },
    AdjustKcal { delta_pct: i32 },                    // signed, e.g. -10
}

// Declared Action-first and Ord-derived so an ascending sort puts Actions
// first and MAX_SUGGESTIONS truncation can never drop an Action for an Info
// (architect review).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity { Action, Info }                     // OQ-4 → two levels (v1)

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Adjustment {
    pub change: Change,
    pub severity: Severity,
    /// Plain-language, cites the triggering facts. No medical claims.
    pub rationale: String,
}
```

### 2.2 Entry point

```rust
/// Pure + deterministic. Empty output = "on track" (a valid result).
#[must_use]
pub fn suggest(
    summary: &TrainingSummary,
    program: &GeneratedProgram,
    diet: &GeneratedDiet,
) -> Vec<Adjustment>
```

Rules run in a fixed order, each returning `Option<Adjustment>`/`Vec`; results
are concatenated, sorted `Action` before `Info` (stable within), truncated to
`MAX_SUGGESTIONS`. No I/O, no clock — the summary already carries its window.

### 2.3 The five rules (thresholds = named consts)

| # | Rule | Trigger (all required) | Change | Severity |
|---|------|------------------------|--------|----------|
| 1 | Stall → deload | `lift.stalled && lift.sessions ≥ MIN_SESSIONS` | `DeloadLift { pct: DELOAD_PCT }` | Action |
| 2 | Gain → progress | `!stalled && slope ≥ PROGRESS_SLOPE && sessions ≥ MIN_SESSIONS && span(e1rm) ≥ MIN_LIFT_SPAN_DAYS` | `ProgressLift { add_kg: INCREMENT_KG }` | Info |
| 3 | Volume gap | `program.volume ≠ Low && mean_weekly_sets < floor(program.volume) && weeks_present ≥ MIN_WEEKS && adherence.ratio ≥ ADHERENT` | `AddWeeklySets { sets: 1 }` | Info |
| 4 | Under-adherence | `adherence.ratio < LOW_ADHERENCE && program.days ≥ 3 && weeks_with_data ≥ MIN_WEEKS` | `ReduceDaysPerWeek { to }` where `to = max(round(ratio × target), 2)`, **silent unless `to < from`** | Action |
| 5 | Scale drift vs diet intent | `diet.intent ≠ Maintain && weight points ≥ MIN_BODY_POINTS && span(weight) ≥ MIN_WEIGHT_SPAN_DAYS && slope contradicts intent by ≥ WEIGHT_DRIFT` | `AdjustKcal { ±KCAL_STEP_PCT }` | Action |

Post-review clarifications (architect):
- **Rule 2/5 span guards** (`MIN_LIFT_SPAN_DAYS = 14`, `MIN_WEIGHT_SPAN_DAYS =
  21`): point *count* alone lets clustered points fake a weekly slope; the
  first→last span must also be long enough.
- **Rule 3 floor is per volume band** (`Low → rule skipped`, `Moderate → 6`,
  `High → 8` weekly sets): a low-volume HIT program is intentionally sparse and
  must not be contradicted. A group must also appear in ≥ `MIN_WEEKS` distinct
  weeks (one stray session is not a pattern).
- **Rule 4 observed frequency is over the whole window**: `ratio × target =
  total_days / window_weeks`. A mean over active weeks alone would overstate
  frequency (6 days in one of eight weeks is not "6/week").
- **Rule 5 sign convention**: `Surplus ∧ slope ≤ −WEIGHT_DRIFT → +KCAL_STEP_PCT`
  (eat more); `Deficit ∧ slope ≥ +WEIGHT_DRIFT → −KCAL_STEP_PCT` (eat less);
  `Maintain` never fires.

Defaults (module consts, tuned in QA): `MIN_SESSIONS = 4`, `DELOAD_PCT = 0.10`,
`PROGRESS_SLOPE = 0.5` kg/wk, `INCREMENT_KG = 2.5`, `LOW_SETS = 6.0`,
`ADHERENT = 0.75`, `LOW_ADHERENCE = 0.6`, `MIN_WEEKS = 3`,
`WEIGHT_DRIFT = 0.25` kg/wk, `KCAL_STEP_PCT = 10`, `MIN_BODY_POINTS = 3`,
`MAX_SUGGESTIONS = 4`.

Rules 1/2 emit at most one suggestion per lift and consider only the top
`TOP_LIFTS = 5` lifts (the summary is already sorted by sessions); 1 wins over
2 for the same lift by construction (`stalled` excludes 2).

### 2.4 Diet intent (OQ-2 — resolved by architect review)

Keyword-parsing the `calorie_strategy` prose is **unsound**: all six shipped
archetype strings say "surplus", while `instantiate` applies the goal
multiplier (`LoseFat → tdee × 0.80`) and keeps the prose — so a cutting user's
diet *says* surplus but *is* a deficit, and a prose-derived intent would tell a
successfully-cutting user to eat more.

Instead, `GeneratedDiet` gains a **typed field** set from the SAME goal branch
as the kcal math (single source of truth):

```rust
#[derive(…, Default)]
#[serde(rename_all = "snake_case")]
pub enum DietIntent { Surplus, Deficit, #[default] Maintain }

pub struct GeneratedDiet { …, #[serde(default)] pub intent: DietIntent, … }

// in instantiate(): intent = match primary goal {
//   LoseFat → Deficit, BuildMuscle | GainStrength → Surplus,
//   Recomp | Maintain → Maintain }
```

`#[serde(default)]` keeps pre-existing stored diets (JSONB in `user_programs`)
deserializable — they read as `Maintain`, so rule 5 stays silent for programs
created before this field existed. There is exactly one intent source; no
keyword fallback exists.

### 2.5 API edge

`GET /adjustments` in the same `summary` module as R-0015's endpoint (which is
itself still to be built — sequence the R-0015 endpoint PR first so this
extends, rather than invents, the fetch path): reuse its
fetch path (sessions, measurements, active program) → `summarize(…)` →
`suggest(…)` → `{ window_weeks, suggestions: [...] }`. No active program ⇒
`200 { suggestions: [], reason: "no_active_program" }`.

## 3. Code outline

```rust
pub fn suggest(summary, program, diet) -> Vec<Adjustment> {
    let mut out = Vec::new();
    out.extend(lift_rules(&summary.lifts));               // rules 1–2
    out.extend(volume_rule(&summary.muscle_volume, &summary.adherence));
    out.extend(adherence_rule(&summary.adherence, program));
    out.extend(diet_rule(&summary.body, diet));
    out.sort_by_key(|a| a.severity);                      // Action first
    out.truncate(MAX_SUGGESTIONS);
    out
}
```

## 4. Non-goals

- No auto-apply, no program/diet mutation, no writes of any kind.
- No ML / `linfa` (R-0016); no archetype data; no notifications (R-0036).
- No program-structure rewrites (split changes beyond `days_per_week`).

## 5. Open questions

- **OQ-1:** threshold defaults above — confirm/tune during QA on seeded data.
- **OQ-4:** severity levels — v1 ships two (`Info`/`Action`).

## 6. Acceptance criteria

Maps to R-0017 AC1–AC10: purity/determinism (AC1, tested); typed `Change` enum
(AC2); five rules each with trigger + non-trigger + boundary tests (AC3, AC9);
`MAX_SUGGESTIONS` bound + valid empty output (AC4); data-sufficiency guards
tested with sparse inputs (AC5); no mutation — engine takes `&` and returns
suggestions only (AC6); endpoint + integration test (AC7); mobile card in the
follow-up PR (AC8); scope guards by review (AC10).

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-09 | Closed `Change` enum, serde-tagged | Machine-readable now; the same wire shape survives R-0016. |
| 2026-07-09 | Two severities (Info/Action) | Simplest useful split; three-level deferred. |
| 2026-07-09 | ~~Diet intent parsed from `calorie_strategy` keywords~~ **Superseded**: typed `DietIntent` on `GeneratedDiet`, set from the goal branch (architect review) | The prose says "surplus" even on fat-loss plans — parsing it gives backwards advice; the typed field shares the kcal math's source of truth. |
| 2026-07-09 | Requirement's `Adjustment { kind, target, change }` collapsed into the tagged `Change` enum | Target lives inside the variant — no illegal states; same machine-readable wire contract (architect-endorsed). |
| 2026-07-09 | Span guards + per-band volume floors + whole-window observed frequency (architect review) | Keeps sparse/clustered data silent and never contradicts the program's own philosophy. |

## Changelog

- _2026-07-09 — created (Draft)._
