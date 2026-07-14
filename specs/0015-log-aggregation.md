# SPEC-0015 — Training-log aggregation (`TrainingSummary`)

- **Status:** Draft
- **Realizes:** R-0015
- **Author:** Claude (main session)
- **Created:** 2026-07-09
- **Depends on:** SPEC-0004 (`WorkoutSession`/`WorkoutExercise`/`WorkoutSet`,
  `MuscleGroup`, `Reps`, `LoadKg`), R-0034 (body measurements), SPEC-0014
  (`UserProgram.days_per_week`), SPEC-0002 (`AuthenticatedUser`, `AppState`).
- **Module(s):** `backend/crates/core/src/aggregate.rs` (new — pure);
  `backend/crates/api/src/summary/` (new — handler + route);
  `backend/crates/api/src/lib.rs` (merge route).

## 1. Motivation

Realizes [R-0015](../requirements/0015-log-aggregation.md). Turn a user's raw
logs into one typed, deterministic `TrainingSummary` — the feature layer the
adjustment engine (R-0017) and the learned model (R-0016) both read. Pure
computation over data we already store; no recommendations, no model.

## 2. Design

### 2.1 Placement & purity

All computation is pure functions in `fitai-core::aggregate` over borrowed
slices — no DB, HTTP, or clock access inside (today's date is a parameter). The
API edge fetches rows, maps them to the input types, and calls one entry point.
Dependencies point inward (CLAUDE.md §2/§6); the aggregator is unit-testable
with no database (AC1).

### 2.2 Inputs

```rust
/// A body-measurement sample (mapped from the R-0034 rows at the API edge, so
/// `core` needn't know the DB shape). lean_mass = weight × (1 − bf/100).
pub struct BodyPoint {
    pub on: NaiveDate,
    pub weight_kg: f64,
    pub body_fat_pct: Option<f64>,
}
```

The entry point borrows the persisted `WorkoutSession`s (core type) directly:

```rust
pub fn summarize(
    today: NaiveDate,          // injected — determinism (AC1)
    window_weeks: u32,         // configurable window (AC6); default 8 at the edge
    sessions: &[WorkoutSession],
    measurements: &[BodyPoint],
    target_days_per_week: u32, // from the active program, for adherence (AC4)
) -> TrainingSummary
```

Only rows with `performed_on` / `on` in the **half-open** window
`(today − window_weeks, today]` are considered (start exclusive, today
inclusive); the cutoff is computed once and applied to every sub-aggregate. The
API edge filters to the same bound so it doesn't over-fetch.

### 2.3 Output

```rust
pub struct TrendPoint { pub on: NaiveDate, pub value: f64 } // oldest first

pub struct LiftSummary {
    pub name: String,              // display name, case-preserved
    pub e1rm: Vec<TrendPoint>,     // best e1RM per session in-window
    pub current_e1rm: f64,
    pub slope_kg_per_week: f64,    // least-squares fit; 0.0 if < 2 points
    pub sessions: u32,
    pub stalled: bool,             // no gain > EPS over last STALL_N sessions
}

pub struct MuscleVolume {
    pub group: MuscleGroup,
    pub weekly_sets: Vec<(NaiveDate, u32)>,   // ISO-week Monday → working sets
    pub weekly_tonnage: Vec<(NaiveDate, f64)>,// ISO-week Monday → Σ reps×weight
    pub mean_weekly_sets: f64,
    pub mean_weekly_tonnage: f64,
}

pub struct Adherence {
    pub weekly_days: Vec<(NaiveDate, u32)>,   // ISO-week Monday → distinct days
    pub target_days_per_week: u32,
    // Σ distinct training days over the window ÷ (target × window_weeks), capped
    // at 1.0. Denominator is the WHOLE window — absent weeks count as zero days
    // (AC4), so 1-of-8 weeks trained ≠ 100%. 0.0 when target or window is 0.
    pub ratio: f64,
}

pub struct BodyTrend {
    pub weight: Vec<TrendPoint>,
    pub weight_slope_kg_per_week: f64,
    pub body_fat_pct: Vec<TrendPoint>,        // only points that carried bf%
    pub body_fat_slope: Option<f64>,          // None if < 2 bf% points
    pub lean_mass: Vec<TrendPoint>,           // derived; only where bf% present
    pub lean_mass_slope: Option<f64>,
}

pub struct TrainingSummary {
    pub window_weeks: u32,
    pub generated_for: NaiveDate,
    pub lifts: Vec<LiftSummary>,       // sorted: most sessions, then biggest gain
    pub muscle_volume: Vec<MuscleVolume>, // sorted: highest mean tonnage first
    pub adherence: Adherence,
    pub body: BodyTrend,
}
```

All output types `#[derive(Clone, Debug, PartialEq, Serialize)]` (+ `Deserialize`
for tests/mobile) for the wire.

### 2.4 Algorithms

- **e1RM:** Epley `w × (1 + reps/30)`, best weighted set per session (mirrors the
  mobile `strength_trend.dart` already shipped — same numbers server-side).
- **Per-lift keying:** group by `name.trim().to_lowercase()`; the display `name`
  is the first-seen trimmed form (matches `strength_trend.dart:112`).
- **Sparse lifts kept (deliberate divergence):** unlike the mobile
  `computePerLiftTrends` (which drops lifts with `< 2` weighted sessions), the
  feature layer keeps a one-session lift (`slope 0.0`, `stalled false`, real
  `current_e1rm`) — it satisfies AC7 and R-0016/17 want the datum. Mobile is the
  oracle for the *math*, not for lift *membership*.
- **Slope:** ordinary least squares of `value` on `x = days_since_CE / 7.0`
  (→ units/week). Guard on **x-variance**, not point count: return `0.0` when
  `n·Σx² − (Σx)² ≈ 0` (all points share an x — e.g. two same-date sessions, or a
  series confined to one week). This avoids a `0/0 = NaN` that `serde_json` would
  emit as `null` and break the `Deserialize` round-trip. `< 2` points ⇒ `0.0`;
  the `Option` slopes (bf%, lean) are `None` only when `< 2` points, else
  `Some(0.0)` for zero-variance. (Theil–Sen deferred, OQ-2.)
- **Stall:** with `e = e1rm` oldest-first, `stalled = e.len() ≥ STALL_N+1 &&
  (e[last].value − max(e[last−STALL_N .. last]).value) ≤ EPS` — i.e. the latest
  session set no new peak versus the `STALL_N` sessions before it (compares only
  `current_e1rm`; a PR that later regressed still reads stalled — intended).
  `< STALL_N+1` points ⇒ `false`.
- **Weekly buckets:** key by the ISO-week Monday (`NaiveDate.week(Mon).first_day`).
- **Volume:** a "working set" = a set with `weight_kg.is_some()`; untagged-muscle
  exercises excluded from muscle volume (matches existing `computeMuscleVolume`).
- **Lift sort tiebreak:** most sessions, then largest first→last e1RM delta
  (`strength_trend.dart:59/143`), not slope.
- **Constants:** `STALL_N = 3`, `EPS = 0.5` (kg) — module consts, revisited in QA.

### 2.5 API edge

`GET /training-summary` (auth). Handler: fetch in-window sessions + measurements
+ the active program's `days_per_week` (reuse existing list/fetch fns), map
measurements → `BodyPoint`, call `summarize(today, 8, …)`, return JSON. No
program ⇒ `target_days_per_week = 0` and `ratio = 0.0` (still well-formed).

## 3. Code outline

```rust
// core/src/aggregate.rs
pub fn summarize(today, window_weeks, sessions, measurements, target) -> TrainingSummary {
    let start = today - Duration::weeks(window_weeks as i64);
    let s: Vec<&WorkoutSession> = sessions.iter()
        .filter(|x| x.performed_on > start && x.performed_on <= today).collect();
    TrainingSummary {
        window_weeks, generated_for: today,
        lifts: per_lift(&s, start),
        muscle_volume: per_muscle(&s, start),
        adherence: adherence(&s, target),
        body: body_trend(measurements, today, start),
    }
}
fn slope(points: &[(f64, f64)]) -> f64 { /* OLS; 0.0 if <2 */ }
fn epley(reps: i32, kg: f64) -> f64 { kg * (1.0 + reps as f64 / 30.0) }
```

## 4. Non-goals

- No recommendations / thresholds-as-decisions (that is R-0017).
- No learned model, no `linfa` (that is R-0016).
- No new data capture; no materialized summary table (compute-on-request; OQ-4).
- No archetype/prior data in the aggregate (R-0015 AC11).

## 5. Open questions

- **OQ-1:** `STALL_N` / `EPS` values (start 3 / 0.5 kg).
- **OQ-2:** OLS vs Theil–Sen slope for sparse noisy data (OLS v1).
- **OQ-4:** compute-on-request vs materialized table (compute v1).

## 6. Acceptance criteria

Maps to R-0015 AC1–AC11.

- [ ] **AC1** — `aggregate` is pure; `summarize` is deterministic given
  `(today, window, sessions, measurements, target)`; tested with no DB.
- [ ] **AC2** — `LiftSummary` has e1RM series, current, `slope_kg_per_week`,
  `sessions`, `stalled`; unit-tested incl. a stall case.
- [ ] **AC3** — `MuscleVolume` weekly sets + tonnage + means; untagged excluded.
- [ ] **AC4** — `Adherence` weekly distinct days + ratio vs target.
- [ ] **AC5** — `BodyTrend` weight/bf%/lean series + slopes.
- [ ] **AC6** — `window_weeks` is a parameter; two windows over the same logs
  differ as expected (tested).
- [ ] **AC7** — 0/1-session and no-measurement inputs yield a well-formed summary
  (empty vecs, `0.0`/`None`), never a panic (tested).
- [ ] **AC8** — `GET /training-summary` returns the caller's summary; integration
  test (auth required, shape, empty-user).
- [ ] **AC9** — unit tests per metric + endpoint integration test.
- [ ] **AC10** — no recommendation/model code in the module (review).
- [ ] **AC11** — summary built only from the user's own logs (review).

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-09 | Pure `core::aggregate`, DB fetch at the edge | Testable without a DB; one feature source for both engines. |
| 2026-07-09 | Mirror the shipped mobile e1RM/volume math | Server + client agree on the numbers. |
| 2026-07-09 | ISO-week Monday buckets | Stable, locale-independent weekly grouping. |
| 2026-07-09 | Slope guards on x-variance, not point count (architect review) | Avoids `0/0 = NaN` → `null` on same-x series, keeping the summary serde-round-trippable (AC7). |
| 2026-07-09 | Adherence denominator = whole window (architect review) | "Rolling adherence over the window" (AC4) — absent weeks are zero, so sparse training scores low. |

## Changelog

- _2026-07-09 — created (Draft)._
