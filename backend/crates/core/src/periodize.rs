//! R-0038 / SPEC-0038 — periodization engines.
//!
//! A structured, time-indexed program model plus three deterministic engines —
//! [`linear`], [`undulating`] (DUP), and [`block`] — that generate a multi-week
//! plan whose loads are `%1RM × e1RM`. Pure: no I/O, no clock, no ML. The e1RM
//! anchor comes from R-0015 (`LiftSummary.current_e1rm`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Per-lift current estimated 1RM, keyed by `lift.trim().to_lowercase()`
/// (matching R-0015's keying).
pub type E1rmMap = BTreeMap<String, f64>;

/// Default plate-rounding increment (kg).
const DEFAULT_PLATE_KG: f64 = 2.5;

// ---------------------------------------------------------------------------
// Model (AC2)
// ---------------------------------------------------------------------------

/// Which periodization scheme produced a program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeriodizationScheme {
    Linear,
    Undulating,
    Block,
}

/// One prescribed set: reps at a fraction of e1RM, with the concrete load when
/// the lift's e1RM is known.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrescribedSet {
    pub reps: u32,
    pub intensity_pct: f64,
    pub target_load_kg: Option<f64>,
}

/// A lift and its sets for one session.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrescribedExercise {
    pub lift: String,
    pub sets: Vec<PrescribedSet>,
}

/// One training day.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrainingSession {
    pub label: String,
    pub exercises: Vec<PrescribedExercise>,
}

/// One week (1-based `index`, global across the whole program).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrainingWeek {
    pub index: u32,
    pub sessions: Vec<TrainingSession>,
}

/// A complete periodized plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeriodizedProgram {
    pub scheme: PeriodizationScheme,
    pub weeks: Vec<TrainingWeek>,
}

// ---------------------------------------------------------------------------
// Parameters (AC7)
// ---------------------------------------------------------------------------

/// Shape shared by every engine. `lifts` are the main lifts programmed each
/// session; `sets` sets of each are prescribed (uniform in v1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanParams {
    pub lifts: Vec<String>,
    pub weeks: u32,
    pub sessions_per_week: u32,
    pub sets: u32,
    pub plate_kg: f64,
}

impl Default for PlanParams {
    fn default() -> Self {
        Self {
            lifts: vec!["Squat".into(), "Bench Press".into(), "Deadlift".into()],
            weeks: 4,
            sessions_per_week: 3,
            sets: 3,
            plate_kg: DEFAULT_PLATE_KG,
        }
    }
}

/// Linear: reps fall and intensity rises from start to end across the block.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LinearParams {
    pub start_reps: u32,
    pub end_reps: u32,
    pub start_pct: f64,
    pub end_pct: f64,
}

impl Default for LinearParams {
    fn default() -> Self {
        Self {
            start_reps: 8,
            end_reps: 3,
            start_pct: 0.70,
            end_pct: 0.875,
        }
    }
}

/// One day's (reps, %1RM) profile in an undulating rotation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DayProfile {
    pub reps: u32,
    pub pct: f64,
}

/// Undulating (DUP): a rotation of day profiles applied across a week's
/// sessions, plus a gentle week-over-week intensity ramp.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UndulatingParams {
    pub day_profiles: Vec<DayProfile>,
    pub weekly_pct_step: f64,
}

impl Default for UndulatingParams {
    fn default() -> Self {
        Self {
            day_profiles: vec![
                DayProfile { reps: 5, pct: 0.85 }, // heavy
                DayProfile {
                    reps: 10,
                    pct: 0.65,
                }, // light
                DayProfile { reps: 8, pct: 0.75 }, // medium
            ],
            weekly_pct_step: 0.02,
        }
    }
}

/// One block of a block-periodized program.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub name: String,
    pub weeks: u32,
    pub start_reps: u32,
    pub end_reps: u32,
    pub start_pct: f64,
    pub end_pct: f64,
}

/// Block: ordered phases; total weeks = Σ block weeks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockParams {
    pub blocks: Vec<Block>,
}

impl Default for BlockParams {
    fn default() -> Self {
        Self {
            blocks: vec![
                Block {
                    name: "Accumulation".into(),
                    weeks: 4,
                    start_reps: 10,
                    end_reps: 8,
                    start_pct: 0.65,
                    end_pct: 0.72,
                },
                Block {
                    name: "Intensification".into(),
                    weeks: 3,
                    start_reps: 6,
                    end_reps: 4,
                    start_pct: 0.75,
                    end_pct: 0.85,
                },
                Block {
                    name: "Realization".into(),
                    weeks: 2,
                    start_reps: 3,
                    end_reps: 1,
                    start_pct: 0.87,
                    end_pct: 0.95,
                },
            ],
        }
    }
}

/// Why a plan could not be generated (AC8 — never a panic).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    #[error("weeks must be >= 1")]
    NoWeeks,
    #[error("at least one lift is required")]
    NoLifts,
    #[error("sessions_per_week must be >= 1")]
    NoSessions,
    #[error("intensity must be in (0, 1]")]
    BadIntensity,
    #[error("reps must be >= 1")]
    BadRepRange,
    #[error("at least one block is required")]
    EmptyBlocks,
    #[error("plan.weeks does not equal the sum of block weeks")]
    WeekMismatch,
    #[error("plate_kg must be finite and > 0")]
    BadPlate,
    #[error("start/end reps or intensity are ordered the wrong way for this scheme")]
    BadOrdering,
}

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

fn round_to_plate(kg: f64, inc: f64) -> f64 {
    (kg / inc).round() * inc
}

fn load(e1rm: Option<f64>, pct: f64, inc: f64) -> Option<f64> {
    e1rm.map(|r| round_to_plate(r * pct, inc))
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    (b - a).mul_add(t, a)
}

/// The lowest intensity a set may prescribe — the DUP ramp clamps here so a
/// negative step can never breach the `0 < pct` invariant (AC8).
const MIN_PCT: f64 = 0.01;

/// Interpolate a rep count. Widened to f64 before subtracting so a falling rep
/// range (start > end) never underflows; rounded and clamped `>= 1`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // small, clamped >= 1
fn lerp_reps(start: u32, end: u32, t: f64) -> u32 {
    let v = lerp(f64::from(start), f64::from(end), t).round().max(1.0);
    v as u32
}

fn valid_pct(p: f64) -> bool {
    p > 0.0 && p <= 1.0
}

/// Common parameter validation shared by every engine (weeks are scheme-specific
/// so checked separately). Guards against panics/NaN per AC8: non-empty +
/// non-blank lifts, a real session count, and a finite positive plate increment.
fn validate_common(plan: &PlanParams) -> Result<(), PlanError> {
    if plan.sessions_per_week == 0 {
        return Err(PlanError::NoSessions);
    }
    if plan.lifts.is_empty() || plan.lifts.iter().any(|l| l.trim().is_empty()) {
        return Err(PlanError::NoLifts);
    }
    if !(plan.plate_kg.is_finite() && plan.plate_kg > 0.0) {
        return Err(PlanError::BadPlate);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Shared week builder
// ---------------------------------------------------------------------------

/// Build one week: `prescribe(session_idx) -> (reps, pct)` gives the scheme, the
/// same set for every lift and every one of `plan.sets` sets. `label(session)`
/// names each session.
fn build_week<P, L>(
    index: u32,
    plan: &PlanParams,
    e1rm: &E1rmMap,
    prescribe: P,
    label: L,
) -> TrainingWeek
where
    P: Fn(u32) -> (u32, f64),
    L: Fn(u32) -> String,
{
    let sessions = (0..plan.sessions_per_week)
        .map(|s| {
            let (reps, pct) = prescribe(s);
            let exercises = plan
                .lifts
                .iter()
                .map(|lift| {
                    let e = e1rm.get(&lift.trim().to_lowercase()).copied();
                    let set = PrescribedSet {
                        reps,
                        intensity_pct: pct,
                        target_load_kg: load(e, pct, plan.plate_kg),
                    };
                    PrescribedExercise {
                        lift: lift.clone(),
                        sets: vec![set; plan.sets as usize],
                    }
                })
                .collect();
            TrainingSession {
                label: label(s),
                exercises,
            }
        })
        .collect();
    TrainingWeek { index, sessions }
}

// ---------------------------------------------------------------------------
// Engines (AC4–AC6)
// ---------------------------------------------------------------------------

/// Linear periodization: reps fall, intensity rises week to week.
///
/// # Errors
/// [`PlanError`] on invalid parameters (incl. `start_pct > end_pct` or
/// `start_reps < end_reps`, which would break the monotonic shape — AC4).
pub fn linear(
    plan: &PlanParams,
    p: &LinearParams,
    e1rm: &E1rmMap,
) -> Result<PeriodizedProgram, PlanError> {
    validate_common(plan)?;
    if plan.weeks == 0 {
        return Err(PlanError::NoWeeks);
    }
    if !valid_pct(p.start_pct) || !valid_pct(p.end_pct) {
        return Err(PlanError::BadIntensity);
    }
    if p.start_reps < 1 || p.end_reps < 1 {
        return Err(PlanError::BadRepRange);
    }
    // Linear guarantees intensity↑ / reps↓ (AC4) — reject reversed inputs rather
    // than silently producing a non-monotonic plan.
    if p.start_pct > p.end_pct || p.start_reps < p.end_reps {
        return Err(PlanError::BadOrdering);
    }

    let weeks = (0..plan.weeks)
        .map(|w| {
            let t = if plan.weeks == 1 {
                0.0
            } else {
                f64::from(w) / f64::from(plan.weeks - 1)
            };
            let reps = lerp_reps(p.start_reps, p.end_reps, t);
            let pct = lerp(p.start_pct, p.end_pct, t);
            let n = w + 1;
            build_week(
                n,
                plan,
                e1rm,
                |_s| (reps, pct),
                move |s| format!("Linear · W{n} · Day {}", s + 1),
            )
        })
        .collect();
    Ok(PeriodizedProgram {
        scheme: PeriodizationScheme::Linear,
        weeks,
    })
}

/// Undulating (DUP): the day-profile rotation varies intensity/reps within each
/// week, with a gentle week-over-week ramp.
///
/// # Errors
/// [`PlanError`] on invalid parameters.
pub fn undulating(
    plan: &PlanParams,
    p: &UndulatingParams,
    e1rm: &E1rmMap,
) -> Result<PeriodizedProgram, PlanError> {
    validate_common(plan)?;
    if plan.weeks == 0 {
        return Err(PlanError::NoWeeks);
    }
    if p.day_profiles.is_empty() {
        return Err(PlanError::NoSessions);
    }
    if p.day_profiles.iter().any(|d| !valid_pct(d.pct)) {
        return Err(PlanError::BadIntensity);
    }
    if p.day_profiles.iter().any(|d| d.reps < 1) {
        return Err(PlanError::BadRepRange);
    }

    let weeks = (0..plan.weeks)
        .map(|w| {
            let n = w + 1;
            let profiles = &p.day_profiles;
            let step = p.weekly_pct_step;
            build_week(
                n,
                plan,
                e1rm,
                move |s| {
                    let d = &profiles[s as usize % profiles.len()];
                    let pct = f64::from(w).mul_add(step, d.pct).clamp(MIN_PCT, 1.0);
                    (d.reps, pct)
                },
                move |s| format!("Undulating · W{n} · Day {}", s + 1),
            )
        })
        .collect();
    Ok(PeriodizedProgram {
        scheme: PeriodizationScheme::Undulating,
        weeks,
    })
}

/// Block periodization: ordered phases (accumulation → intensification →
/// realization), each progressing internally. Total weeks = Σ block weeks.
///
/// # Errors
/// [`PlanError`] on invalid parameters (incl. `plan.weeks` set but ≠ Σ block
/// weeks).
pub fn block(
    plan: &PlanParams,
    p: &BlockParams,
    e1rm: &E1rmMap,
) -> Result<PeriodizedProgram, PlanError> {
    // `plan.weeks` is scheme-derived for Block (Σ block weeks); 0 means "let the
    // blocks define the length", any other value must match (checked below).
    validate_common(plan)?;
    if p.blocks.is_empty() {
        return Err(PlanError::EmptyBlocks);
    }
    if p.blocks.iter().any(|b| b.weeks == 0) {
        return Err(PlanError::NoWeeks);
    }
    if p.blocks
        .iter()
        .any(|b| !valid_pct(b.start_pct) || !valid_pct(b.end_pct))
    {
        return Err(PlanError::BadIntensity);
    }
    if p.blocks.iter().any(|b| b.start_reps < 1 || b.end_reps < 1) {
        return Err(PlanError::BadRepRange);
    }
    // Each block progresses intensity↑ / reps↓ (AC6) — same ordering as Linear.
    if p.blocks
        .iter()
        .any(|b| b.start_pct > b.end_pct || b.start_reps < b.end_reps)
    {
        return Err(PlanError::BadOrdering);
    }
    let total: u32 = p.blocks.iter().map(|b| b.weeks).sum();
    if plan.weeks != 0 && plan.weeks != total {
        return Err(PlanError::WeekMismatch);
    }

    let mut weeks = Vec::with_capacity(total as usize);
    let mut global = 0u32;
    for b in &p.blocks {
        for bw in 0..b.weeks {
            global += 1;
            let t = if b.weeks == 1 {
                0.0
            } else {
                f64::from(bw) / f64::from(b.weeks - 1)
            };
            let reps = lerp_reps(b.start_reps, b.end_reps, t);
            let pct = lerp(b.start_pct, b.end_pct, t);
            let name = b.name.clone();
            let wk = global;
            weeks.push(build_week(
                global,
                plan,
                e1rm,
                |_s| (reps, pct),
                move |s| format!("{name} · W{wk} · Day {}", s + 1),
            ));
        }
    }
    Ok(PeriodizedProgram {
        scheme: PeriodizationScheme::Block,
        weeks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e1rm() -> E1rmMap {
        E1rmMap::from([
            ("squat".to_string(), 200.0),
            ("bench press".to_string(), 100.0),
            ("deadlift".to_string(), 240.0),
        ])
    }

    fn plan() -> PlanParams {
        PlanParams {
            lifts: vec!["Squat".into(), "Bench Press".into()],
            weeks: 4,
            sessions_per_week: 3,
            sets: 3,
            plate_kg: 2.5,
        }
    }

    fn first_set(prog: &PeriodizedProgram, week: usize, session: usize) -> &PrescribedSet {
        &prog.weeks[week].sessions[session].exercises[0].sets[0]
    }

    #[test]
    fn round_to_plate_snaps_to_increment() {
        assert!((round_to_plate(141.0, 2.5) - 140.0).abs() < 1e-9);
        assert!((round_to_plate(141.3, 2.5) - 142.5).abs() < 1e-9);
    }

    #[test]
    fn shape_matches_parameters() {
        let p = linear(&plan(), &LinearParams::default(), &e1rm()).unwrap();
        assert_eq!(p.weeks.len(), 4);
        assert_eq!(p.weeks[0].index, 1);
        assert_eq!(p.weeks[0].sessions.len(), 3);
        assert_eq!(p.weeks[0].sessions[0].exercises.len(), 2); // squat + bench
        assert_eq!(p.weeks[0].sessions[0].exercises[0].sets.len(), 3);
    }

    #[test]
    fn load_is_pct_of_e1rm_rounded_to_plate() {
        let p = linear(&plan(), &LinearParams::default(), &e1rm()).unwrap();
        // Week 1 default = 8 reps @ 70%. Squat e1RM 200 → 140.0.
        let s = first_set(&p, 0, 0);
        assert_eq!(s.reps, 8);
        assert!((s.intensity_pct - 0.70).abs() < 1e-9);
        assert_eq!(s.target_load_kg, Some(140.0));
    }

    #[test]
    fn missing_e1rm_prescribes_reps_and_pct_without_load() {
        let mut pl = plan();
        pl.lifts = vec!["Overhead Press".into()]; // not in the e1RM map
        let p = linear(&pl, &LinearParams::default(), &e1rm()).unwrap();
        let s = first_set(&p, 0, 0);
        assert_eq!(s.target_load_kg, None);
        assert_eq!(s.reps, 8);
        assert!(s.intensity_pct > 0.0);
    }

    #[test]
    fn linear_reps_fall_and_intensity_rises_monotonically() {
        let p = linear(&plan(), &LinearParams::default(), &e1rm()).unwrap();
        let mut prev_pct = 0.0;
        let mut prev_reps = u32::MAX;
        for w in 0..p.weeks.len() {
            let s = first_set(&p, w, 0);
            assert!(s.intensity_pct >= prev_pct, "intensity must not decrease");
            assert!(s.reps <= prev_reps, "reps must not increase");
            prev_pct = s.intensity_pct;
            prev_reps = s.reps;
        }
        // End week hits the configured end (3 reps @ 87.5%).
        let last = first_set(&p, 3, 0);
        assert_eq!(last.reps, 3);
        assert!((last.intensity_pct - 0.875).abs() < 1e-9);
    }

    #[test]
    fn single_week_is_valid_and_uses_start_values() {
        let mut pl = plan();
        pl.weeks = 1;
        let p = linear(&pl, &LinearParams::default(), &e1rm()).unwrap();
        assert_eq!(p.weeks.len(), 1);
        let s = first_set(&p, 0, 0);
        assert_eq!(s.reps, 8); // t=0 → start
        assert!((s.intensity_pct - 0.70).abs() < 1e-9);
    }

    #[test]
    fn undulating_sessions_vary_within_a_week() {
        let p = undulating(&plan(), &UndulatingParams::default(), &e1rm()).unwrap();
        let w0 = &p.weeks[0].sessions;
        let d0 = first_set(&p, 0, 0);
        let d1 = first_set(&p, 0, 1);
        let d2 = first_set(&p, 0, 2);
        // heavy 5@85, light 10@65, medium 8@75 — all distinct.
        assert_eq!((d0.reps, d1.reps, d2.reps), (5, 10, 8));
        assert!(d0.intensity_pct > d2.intensity_pct && d2.intensity_pct > d1.intensity_pct);
        assert_eq!(w0.len(), 3);
    }

    #[test]
    fn undulating_ramps_intensity_week_over_week() {
        let p = undulating(&plan(), &UndulatingParams::default(), &e1rm()).unwrap();
        // Same day (heavy) later in the plan is heavier by the weekly step.
        let wk1 = first_set(&p, 0, 0).intensity_pct;
        let wk4 = first_set(&p, 3, 0).intensity_pct;
        assert!((wk4 - (wk1 + 3.0 * 0.02)).abs() < 1e-9);
    }

    #[test]
    fn block_orders_phases_with_global_week_index() {
        let mut pl = plan();
        pl.weeks = 0; // let the blocks define the length (4+3+2 = 9)
        let p = block(&pl, &BlockParams::default(), &e1rm()).unwrap();
        assert_eq!(p.weeks.len(), 9);
        assert_eq!(p.weeks[0].index, 1);
        assert_eq!(p.weeks[8].index, 9);
        assert!(p.weeks[0].sessions[0].label.starts_with("Accumulation"));
        assert!(p.weeks[8].sessions[0].label.starts_with("Realization"));
        // Realization is heavier + lower-rep than accumulation.
        let acc = first_set(&p, 0, 0);
        let real = first_set(&p, 8, 0);
        assert!(real.intensity_pct > acc.intensity_pct);
        assert!(real.reps < acc.reps);
    }

    #[test]
    fn block_week_mismatch_is_an_error() {
        let mut pl = plan();
        pl.weeks = 5; // blocks sum to 9
        assert_eq!(
            block(&pl, &BlockParams::default(), &e1rm()),
            Err(PlanError::WeekMismatch)
        );
    }

    #[test]
    fn invalid_params_return_typed_errors_never_panic() {
        let bad_weeks = PlanParams { weeks: 0, ..plan() };
        assert_eq!(
            linear(&bad_weeks, &LinearParams::default(), &e1rm()),
            Err(PlanError::NoWeeks)
        );

        let no_lifts = PlanParams {
            lifts: vec![],
            ..plan()
        };
        assert_eq!(
            linear(&no_lifts, &LinearParams::default(), &e1rm()),
            Err(PlanError::NoLifts)
        );

        let bad_pct = LinearParams {
            start_pct: 1.5,
            ..LinearParams::default()
        };
        assert_eq!(
            linear(&plan(), &bad_pct, &e1rm()),
            Err(PlanError::BadIntensity)
        );

        assert_eq!(
            block(&plan(), &BlockParams { blocks: vec![] }, &e1rm()),
            Err(PlanError::EmptyBlocks)
        );

        // Architect-flagged guards.
        let zero_plate = PlanParams {
            plate_kg: 0.0,
            ..plan()
        };
        assert_eq!(
            linear(&zero_plate, &LinearParams::default(), &e1rm()),
            Err(PlanError::BadPlate)
        );

        let blank_lift = PlanParams {
            lifts: vec!["  ".into()],
            ..plan()
        };
        assert_eq!(
            linear(&blank_lift, &LinearParams::default(), &e1rm()),
            Err(PlanError::NoLifts)
        );

        // Reversed Linear (intensity would fall) → rejected, not silently
        // non-monotonic.
        let reversed = LinearParams {
            start_pct: 0.85,
            end_pct: 0.70,
            ..LinearParams::default()
        };
        assert_eq!(
            linear(&plan(), &reversed, &e1rm()),
            Err(PlanError::BadOrdering)
        );
    }

    #[test]
    fn undulating_never_prescribes_zero_intensity_on_a_negative_ramp() {
        // A steeply negative weekly step must clamp at MIN_PCT, never 0 (AC8).
        let params = UndulatingParams {
            day_profiles: vec![DayProfile { reps: 5, pct: 0.5 }],
            weekly_pct_step: -0.30,
        };
        let mut pl = plan();
        pl.weeks = 6;
        let p = undulating(&pl, &params, &e1rm()).unwrap();
        for w in 0..p.weeks.len() {
            let pct = first_set(&p, w, 0).intensity_pct;
            assert!((MIN_PCT..=1.0).contains(&pct), "pct {pct} out of (0,1]");
        }
    }

    #[test]
    fn deterministic_and_serde_round_trips() {
        let a = undulating(&plan(), &UndulatingParams::default(), &e1rm()).unwrap();
        let b = undulating(&plan(), &UndulatingParams::default(), &e1rm()).unwrap();
        assert_eq!(a, b);
        let json = serde_json::to_string(&a).unwrap();
        let back: PeriodizedProgram = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
        assert!(json.contains("\"scheme\":\"undulating\""));
    }
}
