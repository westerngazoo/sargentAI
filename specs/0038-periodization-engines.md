# SPEC-0038 — Periodization engines (`core::periodize`)

- **Status:** Draft
- **Realizes:** R-0038
- **Author:** Claude (main session)
- **Created:** 2026-07-22
- **Depends on:** SPEC-0015 (`LiftSummary.current_e1rm` — the load anchor).
- **Module(s):** `backend/crates/core/src/periodize.rs` (new — pure).

## 1. Motivation

Realizes [R-0038](../requirements/0038-periodization-engines.md): a structured
periodized-program model + three deterministic engines (Linear, Undulating/DUP,
Block) that generate a multi-week plan with concrete loads = `%1RM × e1RM`.

## 2. Design

### 2.1 Model

```rust
#[serde(rename_all = "snake_case")]
pub enum PeriodizationScheme { Linear, Undulating, Block }

pub struct PrescribedSet {
    pub reps: u32,
    pub intensity_pct: f64,           // 0 < x ≤ 1.0, fraction of e1RM
    pub target_load_kg: Option<f64>,  // round_to_plate(e1rm × pct); None if no e1RM
}
pub struct PrescribedExercise { pub lift: String, pub sets: Vec<PrescribedSet> }
pub struct TrainingSession   { pub label: String, pub exercises: Vec<PrescribedExercise> }
pub struct TrainingWeek      { pub index: u32, pub sessions: Vec<TrainingSession> }
pub struct PeriodizedProgram { pub scheme: PeriodizationScheme, pub weeks: Vec<TrainingWeek> }
```

All types derive `Clone, Debug, PartialEq, Serialize, Deserialize` (AC9).

### 2.2 Parameters

```rust
/// Shared shape. `lifts` are the main lifts programmed each session.
pub struct PlanParams {
    pub lifts: Vec<String>,
    pub weeks: u32,
    pub sessions_per_week: u32,
    pub sets: u32,           // sets per lift per session (uniform in v1)
    pub plate_kg: f64,       // rounding increment (default 2.5)
}

pub struct LinearParams { pub start_reps: u32, pub end_reps: u32, pub start_pct: f64, pub end_pct: f64 }

/// A rotation of (reps, %1RM) day profiles applied across a week's sessions.
pub struct UndulatingParams { pub day_profiles: Vec<DayProfile>, pub weekly_pct_step: f64 }
pub struct DayProfile { pub reps: u32, pub pct: f64 }

pub struct Block { pub name: String, pub weeks: u32,
                   pub start_reps: u32, pub end_reps: u32, pub start_pct: f64, pub end_pct: f64 }
pub struct BlockParams { pub blocks: Vec<Block> }   // total weeks = Σ block.weeks
```

`Default` impls give textbook values (e.g. Linear 3×8 @70% → 3×3 @87.5%; DUP
profiles `[(5,0.85),(10,0.65),(8,0.75)]`, step 0.02; Block = accumulation(4,
10→8, .65→.72) · intensification(3, 6→4, .75→.85) · realization(2, 3→1,
.87→.95)).

### 2.3 Load & rounding (AC3)

```rust
fn round_to_plate(kg: f64, inc: f64) -> f64 { (kg / inc).round() * inc }
fn load(e1rm: Option<f64>, pct: f64, inc: f64) -> Option<f64>
    { e1rm.map(|r| round_to_plate(r * pct, inc)) }
```

The e1RM map is `&BTreeMap<String, f64>` keyed by `lift.trim().to_lowercase()`
(matches R-0015 keying); lookups normalize the same way. Missing lift ⇒ `None`
load, reps × %intensity still prescribed.

### 2.4 Errors (AC8)

```rust
pub enum PlanError { NoWeeks, NoLifts, NoSessions, BadIntensity, BadRepRange,
                     EmptyBlocks, WeekMismatch, BadPlate, BadOrdering }
```
`PlanError` derives `thiserror::Error`.

Validation (before generation) — a shared `validate_common` guards the
panic/NaN paths (architect review): `sessions_per_week ≥ 1`; `!lifts.is_empty()`
**and no blank/whitespace lift** (→ `NoLifts`); `plate_kg` **finite and > 0**
(→ `BadPlate`, else `round_to_plate` divides by zero → NaN → breaks the AC9
round-trip). Then per engine: `weeks ≥ 1` for Linear/DUP; every `pct ∈ (0,1]`;
every `reps ≥ 1`; DUP `!day_profiles.is_empty()` (→ `NoSessions`, else `s % 0`
panics). **Ordering (AC4/AC6):** Linear and each Block require `start_pct ≤
end_pct` and `start_reps ≥ end_reps`, else `BadOrdering` — reversed inputs are
rejected rather than silently producing a non-monotonic plan. Block:
`!blocks.is_empty()` and each block `weeks ≥ 1`.

### 2.5 Engines (the math)

Entry points, all `-> Result<PeriodizedProgram, PlanError>`:

```rust
pub fn linear(plan: &PlanParams, p: &LinearParams, e1rm: &E1rmMap) -> Result<…>;
pub fn undulating(plan: &PlanParams, p: &UndulatingParams, e1rm: &E1rmMap) -> Result<…>;
pub fn block(plan: &PlanParams, p: &BlockParams, e1rm: &E1rmMap) -> Result<…>;
```

- **Linear (AC4):** for week `w ∈ 0..weeks`, `t = if weeks==1 {0} else {w/(weeks-1)}`;
  `reps = round(lerp(start_reps, end_reps, t))`, `pct = lerp(start_pct, end_pct, t)`.
  Every session in the week uses the same (reps, pct); every lift gets `sets`
  identical sets. Reps decrease, intensity increases monotonically week to week.
- **Undulating / DUP (AC5):** within week `w`, session `s ∈ 0..sessions_per_week`
  uses `profile = day_profiles[s % day_profiles.len()]`; `reps = profile.reps`,
  `pct = clamp(profile.pct + weekly_pct_step × w, MIN_PCT, 1.0)` where `MIN_PCT = 0.01` — a negative step can never breach the `0 < pct` invariant. Sessions within a week differ (heavy/light/medium); a gentle weekly ramp adds progression. (With `sessions_per_week == 1` there is no within-week variance — a documented degenerate case, not an error.)
- **Block (AC6):** length is derived from the blocks (total = Σ block.weeks); `plan.weeks` is redundant for Block — `0` means "let the blocks define it", any other value must equal the sum else `WeekMismatch`. Concatenate blocks in order. Within a block
  of `bw` weeks, interpolate reps↓/pct↑ from block start→end just like Linear.
  `TrainingWeek.index` is global (1-based across the whole program); the session
  label carries the block name (e.g. "Accumulation · W2 · Day 1").

Session labels: `"{scheme/phase} · W{week} · Day {d}"`. Lerp helpers: `lerp(a,b,t)=a+(b−a)t`; **reps interpolate in f64 before any subtraction** (a falling rep range would underflow in u32), rounded and clamped `≥1`.

## 3. Code outline

```rust
fn build_week(idx, sessions_per_week, lifts, sets, prescribe, e1rm, inc, label_of)
  -> TrainingWeek  // prescribe: Fn(session_idx) -> (reps, pct)

pub fn linear(plan, p, e1rm) -> Result<PeriodizedProgram, PlanError> {
    validate(plan)?; validate_linear(p)?;
    let weeks = (0..plan.weeks).map(|w| {
        let t = if plan.weeks==1 {0.0} else {w as f64/(plan.weeks-1) as f64};
        let reps = lerp_u32(p.start_reps, p.end_reps, t);
        let pct  = lerp(p.start_pct, p.end_pct, t);
        build_week(w+1, plan, |_s| (reps, pct), e1rm, "Linear")
    }).collect();
    Ok(PeriodizedProgram { scheme: Linear, weeks })
}
```

## 4. Non-goals

Per R-0038 §4: no persistence, endpoint, mobile UI, archetype rewiring, user
builder, RPE model, or ML in this spec.

## 5. Open questions

- **OQ-1/3/4:** default curves, plate policy (2.5 kg v1), accessories (main lifts
  only in v1) — confirmed/tuned in QA.

## 6. Acceptance criteria

Maps R-0038 AC1–AC11: purity/determinism (AC1); model shape (AC2); load+rounding
+ no-e1RM `None` (AC3); Linear monotonic (AC4); DUP within-week variance (AC5);
Block phase order (AC6); named params/consts (AC7); bounded output + typed errors
(AC8); serde round-trip (AC9); per-engine + determinism + serde tests (AC10);
pure-only scope (AC11).

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-22 | One module, three entry fns sharing a `build_week` helper | DRY; each engine is just its (reps,pct) rule. |
| 2026-07-22 | e1RM map keyed lowercase/trimmed | Matches R-0015 lift keying; server/client agree. |
| 2026-07-22 | Missing e1RM ⇒ `None` load, still prescribe reps×% | New users get a usable plan; loads fill in as they log. |
| 2026-07-22 | `validate_common` guards plate>0, non-blank lifts, non-empty profiles (architect review) | Closes the divide-by-zero/NaN and index panics AC8 forbids. |
| 2026-07-22 | Linear/Block reject reversed start/end (`BadOrdering`) | AC4/AC6 promise monotonic shape; reversed input is a caller error, not a silent reverse ramp. |
| 2026-07-22 | Block ignores `plan.weeks` (0 = derive; non-0 must match) | ISP — `weeks` is meaningful for Linear/DUP, redundant for Block. |

## Changelog

- _2026-07-22 — created (Draft)._
