//! R-0042 / SPEC-0042 — goal targets & pace assessment.
//!
//! A dated target ([`Goal`]) plus the pure math turning (goal, measured
//! signal, date) into a typed [`PaceReport`]. The signal is R-0015's
//! aggregates borrowed as-is ([`GoalSignal`]); core owns every derivation from
//! them — slope presence, per-metric counts, spans. [`assess`] is **total**:
//! every input produces a report, degradation is
//! [`PaceStatus::InsufficientData`], never an error. No I/O, no clock beyond
//! the explicit `today` parameter (AC9).

use std::collections::BTreeMap;

use chrono::{Duration, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

use crate::aggregate::{
    Adherence, BodyTrend, LiftSummary, TrainingSummary, TrendPoint, DEFAULT_WINDOW_WEEKS,
};
use crate::periodize::lift_key;

// ---------------------------------------------------------------------------
// Validation caps (SPEC-0042 §2.4)
// ---------------------------------------------------------------------------

/// Longest horizon between `set_on` and `target_date` — two years. A goal
/// further out than any training block is plannable is noise, not a target.
pub const MAX_GOAL_HORIZON_DAYS: i64 = 730;

/// Largest accepted e1RM target (kg). Garbage in, garbage pace forever.
pub const MAX_TARGET_E1RM_KG: f64 = 1500.0;

/// Largest accepted bodyweight target (kg).
pub const MAX_TARGET_WEIGHT_KG: f64 = 500.0;

/// Body-fat targets must lie strictly inside `(0, MAX_TARGET_BODY_FAT_PCT)`.
pub const MAX_TARGET_BODY_FAT_PCT: f64 = 75.0;

/// Longest accepted lift name — the enforceable proxy for "a real exercise
/// name" (R-0042 AC10 as amended: lifts are free text, there is no registry).
pub const MAX_LIFT_CHARS: usize = 80;

/// Most sessions per week a consistency goal may demand (two a day).
pub const MAX_SESSIONS_PER_WEEK: u32 = 14;

// ---------------------------------------------------------------------------
// Data floors (SPEC-0042 §2.3) and pace bands (§2.2.2)
// ---------------------------------------------------------------------------

/// Strength floor: at least this many in-window sessions before projecting.
pub const MIN_STRENGTH_SESSIONS: u32 = 3;

/// Body floor: at least this many trend points for the metric…
pub const MIN_BODY_POINTS: usize = 3;

/// …spanning at least this many days.
pub const MIN_BODY_SPAN_DAYS: i64 = 14;

/// A slope is established only from at least this many distinct-date points —
/// R-0015's bare `slope == 0.0` is ambiguous between "plateau" and "no data",
/// so core decides trend presence from the point vectors, never the scalar.
pub const MIN_TREND_DATES: usize = 2;

/// The `OnTrack` tolerance: ±this fraction of the remaining required change
/// (SPEC-0042 OQ-2 — a fixed band, since a variance-based one needs data the
/// floors do not guarantee).
pub const PACE_BAND_FRACTION: f64 = 0.25;

/// Inside this many weeks of the deadline, a shortfall past the band is
/// `AtRisk` rather than `Behind` — there is no time left to recover.
const AT_RISK_FINAL_WEEKS: f64 = 2.0;

/// A stalled or wrong-signed pace is `AtRisk` only while at least this
/// fraction of the baseline→target span remains.
const AT_RISK_REMAINING_FRACTION: f64 = 0.25;

/// `|target − baseline|` below this is a zero span: the remaining-fraction
/// arm is skipped rather than dividing by (nearly) nothing (§2.2.2).
const SPAN_EPSILON: f64 = 1e-9;

// ---------------------------------------------------------------------------
// Domain model (SPEC-0042 §2.1)
// ---------------------------------------------------------------------------

/// What a goal targets: one of the three kinds R-0042 AC2 pins.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum GoalKind {
    /// A per-lift e1RM target.
    Strength {
        /// Free-text lift name, matched to the signal via the canonical
        /// trimmed/lowercased key.
        lift: String,
        /// The e1RM to reach (kg).
        target_e1rm_kg: f64,
    },
    /// A bodyweight and/or body-fat target — at least one field set (AC2).
    Body {
        /// Bodyweight to reach (kg), if targeted.
        target_weight_kg: Option<f64>,
        /// Body-fat percentage to reach, if targeted.
        target_body_fat_pct: Option<f64>,
    },
    /// Sessions per week sustained over the window.
    Consistency {
        /// The weekly session rate to sustain.
        sessions_per_week: u32,
    },
}

/// What was measurable when the goal was set — captured server-side (AC1).
/// `None` means the metric had no data at creation; assessment late-binds it
/// (SPEC-0042 §2.2.3) rather than trapping the goal in `InsufficientData`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Baseline {
    /// The e1RM measured at creation, if any.
    Strength {
        /// Baseline e1RM (kg).
        e1rm_kg: Option<f64>,
    },
    /// The body measurements at creation, if any.
    Body {
        /// Baseline bodyweight (kg).
        weight_kg: Option<f64>,
        /// Baseline body-fat percentage.
        body_fat_pct: Option<f64>,
    },
    /// Consistency has no baseline — the target is a rate, not a change.
    Consistency,
}

/// A dated target: what to reach, from where, by when.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    /// What is targeted.
    pub kind: GoalKind,
    /// What was measurable at creation (server-captured, AC1).
    pub baseline: Baseline,
    /// The date the goal was set.
    pub set_on: NaiveDate,
    /// The deadline.
    pub target_date: NaiveDate,
}

// ---------------------------------------------------------------------------
// Pace assessment types (SPEC-0042 §2.2)
// ---------------------------------------------------------------------------

/// The R-0015 aggregates, borrowed as-is. Core owns every derivation — the
/// api adapts nothing (SPEC-0042 decision log).
#[derive(Clone, Copy, Debug)]
pub struct GoalSignal<'a> {
    /// Per-lift strength picture over the window.
    pub lifts: &'a [LiftSummary],
    /// Body-composition trends over the window.
    pub body: &'a BodyTrend,
    /// Training frequency over the window.
    pub adherence: &'a Adherence,
}

impl<'a> From<&'a TrainingSummary> for GoalSignal<'a> {
    fn from(s: &'a TrainingSummary) -> Self {
        Self {
            lifts: &s.lifts,
            body: &s.body,
            adherence: &s.adherence,
        }
    }
}

/// One tracked metric of a goal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "metric")]
pub enum Metric {
    /// A lift's estimated 1RM (kg).
    E1rmKg {
        /// The goal's lift, as the user named it.
        lift: String,
    },
    /// Bodyweight (kg).
    WeightKg,
    /// Body-fat percentage.
    BodyFatPct,
    /// Training sessions per week.
    SessionsPerWeek,
}

/// The typed pace verdict for one metric or one whole goal (AC4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status", content = "detail")]
pub enum PaceStatus {
    /// The target is currently met. Pre-deadline this is a current-state
    /// report, not a terminal fact — regress and it reverts (§2.2.2).
    Achieved,
    /// Projected past the target by more than the band.
    Ahead,
    /// Projected within the band of the target.
    OnTrack,
    /// Projected short of the target by more than the band, with time left.
    Behind,
    /// Short of the target with no realistic path: the final stretch, or a
    /// stalled/wrong-signed pace with most of the journey remaining.
    AtRisk,
    /// Terminal: the deadline passed without the target demonstrably met.
    Missed,
    /// Not enough data for an honest answer — never a fabricated projection
    /// (AC5).
    InsufficientData {
        /// Why the assessment cannot be made.
        reason: InsufficientReason,
    },
}

/// Why a metric's pace cannot be assessed (AC5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsufficientReason {
    /// Neither a stored baseline nor any current value.
    NoSignal,
    /// Observations exist but sit below the per-kind floor (§2.3).
    TooFewObservations,
    /// Observations exist; no slope could be established from them.
    NoTrend,
}

impl PaceStatus {
    /// The pinned severity order (SPEC-0042 §2.2.1):
    /// `Achieved < Ahead < OnTrack < Behind < AtRisk < Missed <
    /// InsufficientData`. A report's overall status is the maximum across its
    /// metrics under this order.
    #[must_use]
    pub fn severity(&self) -> u8 {
        match self {
            PaceStatus::Achieved => 0,
            PaceStatus::Ahead => 1,
            PaceStatus::OnTrack => 2,
            PaceStatus::Behind => 3,
            PaceStatus::AtRisk => 4,
            PaceStatus::Missed => 5,
            PaceStatus::InsufficientData { .. } => 6,
        }
    }
}

/// One tracked metric's pace: the status plus the numbers behind it (AC4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricPace {
    /// Which metric this row describes.
    pub metric: Metric,
    /// The verdict for this metric.
    pub status: PaceStatus,
    /// The stored baseline — `None` when late-bound (§2.2.3).
    pub baseline: Option<f64>,
    /// The latest in-window measurement, if any.
    pub current: Option<f64>,
    /// The target value.
    pub target: f64,
    /// Change per week needed to arrive on time. `None` once terminal.
    pub required_per_week: Option<f64>,
    /// Change per week actually observed, when a trend is established.
    pub observed_per_week: Option<f64>,
    /// `current` extrapolated to the deadline at the observed pace. `None`
    /// once terminal or when no trend is established (never fabricated, AC5).
    pub projected_at_deadline: Option<f64>,
    /// Weeks until the deadline, clamped at 0.
    pub weeks_remaining: f64,
}

/// The full pace report for one goal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaceReport {
    /// The most severe metric status under the pinned order (§2.2.1).
    pub status: PaceStatus,
    /// One row per tracked metric: Strength and Consistency have exactly one;
    /// Body has one per set target field.
    pub metrics: Vec<MetricPace>,
}

// ---------------------------------------------------------------------------
// Validation (SPEC-0042 §2.4)
// ---------------------------------------------------------------------------

/// Why a goal is invalid (never a panic).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GoalError {
    #[error("target_date must be after the creation date")]
    TargetDateNotAfterCreation,
    #[error("target_date is more than {MAX_GOAL_HORIZON_DAYS} days out")]
    TargetDateTooFar,
    #[error("a target value must be finite")]
    NonFiniteTarget,
    #[error("a target value must be > 0")]
    NonPositiveTarget,
    #[error("a target value exceeds its magnitude cap")]
    TargetTooLarge,
    #[error("a body-fat target must lie inside (0, {MAX_TARGET_BODY_FAT_PCT})")]
    BodyFatOutOfRange,
    #[error("a body goal needs at least one target field")]
    BodyTargetEmpty,
    #[error("the lift name is blank")]
    BlankLift,
    #[error("the lift name exceeds {MAX_LIFT_CHARS} characters")]
    LiftTooLong,
    #[error("sessions_per_week must be >= 1")]
    ZeroSessions,
    #[error("sessions_per_week exceeds {MAX_SESSIONS_PER_WEEK}")]
    TooManySessions,
}

/// Check a goal's kind and dates before it is stored.
///
/// # Errors
/// A [`GoalError`] for a non-future or too-distant `target_date`, a
/// non-finite / non-positive / over-cap target value, an out-of-range
/// body-fat target, a `Body` goal with no target field, a blank or
/// over-length lift name, or a zero / absurd session rate.
pub fn validate(
    kind: &GoalKind,
    set_on: NaiveDate,
    target_date: NaiveDate,
) -> Result<(), GoalError> {
    if target_date <= set_on {
        return Err(GoalError::TargetDateNotAfterCreation);
    }
    if (target_date - set_on).num_days() > MAX_GOAL_HORIZON_DAYS {
        return Err(GoalError::TargetDateTooFar);
    }
    match kind {
        GoalKind::Strength {
            lift,
            target_e1rm_kg,
        } => {
            check_lift(lift)?;
            check_target(*target_e1rm_kg, MAX_TARGET_E1RM_KG)
        }
        GoalKind::Body {
            target_weight_kg,
            target_body_fat_pct,
        } => {
            if target_weight_kg.is_none() && target_body_fat_pct.is_none() {
                return Err(GoalError::BodyTargetEmpty);
            }
            if let Some(w) = target_weight_kg {
                check_target(*w, MAX_TARGET_WEIGHT_KG)?;
            }
            if let Some(bf) = target_body_fat_pct {
                check_body_fat(*bf)?;
            }
            Ok(())
        }
        GoalKind::Consistency { sessions_per_week } => {
            if *sessions_per_week == 0 {
                return Err(GoalError::ZeroSessions);
            }
            if *sessions_per_week > MAX_SESSIONS_PER_WEEK {
                return Err(GoalError::TooManySessions);
            }
            Ok(())
        }
    }
}

fn check_lift(lift: &str) -> Result<(), GoalError> {
    if lift.trim().is_empty() {
        return Err(GoalError::BlankLift);
    }
    if lift.chars().count() > MAX_LIFT_CHARS {
        return Err(GoalError::LiftTooLong);
    }
    Ok(())
}

fn check_target(value: f64, cap: f64) -> Result<(), GoalError> {
    if !value.is_finite() {
        return Err(GoalError::NonFiniteTarget);
    }
    if value <= 0.0 {
        return Err(GoalError::NonPositiveTarget);
    }
    if value > cap {
        return Err(GoalError::TargetTooLarge);
    }
    Ok(())
}

fn check_body_fat(bf: f64) -> Result<(), GoalError> {
    if !bf.is_finite() {
        return Err(GoalError::NonFiniteTarget);
    }
    if bf <= 0.0 || bf >= MAX_TARGET_BODY_FAT_PCT {
        return Err(GoalError::BodyFatOutOfRange);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Baseline capture (SPEC-0042 §2.6)
// ---------------------------------------------------------------------------

impl Baseline {
    /// Capture what is measurable *now* for `kind` from an aggregated summary
    /// — the server-side baseline stored at creation (AC1). Fields with no
    /// data stay `None`; creation still succeeds (§2.2.3 covers the gap).
    #[must_use]
    pub fn capture(kind: &GoalKind, summary: &TrainingSummary) -> Self {
        match kind {
            GoalKind::Strength { lift, .. } => Baseline::Strength {
                e1rm_kg: find_lift(&summary.lifts, lift).map(|l| l.current_e1rm),
            },
            GoalKind::Body {
                target_weight_kg,
                target_body_fat_pct,
            } => Baseline::Body {
                // Only the *targeted* fields are relevant — an untargeted
                // metric never pairs with a baseline in assessment.
                weight_kg: target_weight_kg.and_then(|_| latest(&summary.body.weight)),
                body_fat_pct: target_body_fat_pct.and_then(|_| latest(&summary.body.body_fat_pct)),
            },
            GoalKind::Consistency { .. } => Baseline::Consistency,
        }
    }
}

// ---------------------------------------------------------------------------
// Assessment (SPEC-0042 §2.2)
// ---------------------------------------------------------------------------

/// Assess a goal's pace against the measured signal, as of `today`.
///
/// Pure and **total**: every input produces a report; degradation is
/// [`PaceStatus::InsufficientData`], never an error (AC5, AC9). `today` is
/// the only clock. Post-deadline (`today > target_date`) the report is
/// terminal — `Achieved` or `Missed` — and carries no projection (AC6); the
/// caller anchors the signal at the deadline so the terminal verdict is a
/// stable function of history (§2.2.4).
#[must_use]
pub fn assess(goal: &Goal, signal: &GoalSignal<'_>, today: NaiveDate) -> PaceReport {
    let metrics: Vec<MetricPace> = match &goal.kind {
        GoalKind::Strength {
            lift,
            target_e1rm_kg,
        } => {
            let found = find_lift(signal.lifts, lift);
            vec![assess_scalar(
                ScalarMetric {
                    metric: Metric::E1rmKg { lift: lift.clone() },
                    target: *target_e1rm_kg,
                    stored: stored_strength(&goal.baseline),
                    current: found.map(|l| l.current_e1rm),
                    observed: found.and_then(|l| established_slope(&l.e1rm, l.slope_kg_per_week)),
                    earliest: found.and_then(|l| earliest(&l.e1rm)),
                    enough: found.is_some_and(|l| l.sessions >= MIN_STRENGTH_SESSIONS),
                },
                goal,
                today,
            )]
        }
        GoalKind::Body {
            target_weight_kg,
            target_body_fat_pct,
        } => {
            let (stored_weight, stored_bf) = stored_body(&goal.baseline);
            let mut rows = Vec::with_capacity(2);
            if let Some(target) = target_weight_kg {
                let points = &signal.body.weight;
                rows.push(assess_scalar(
                    ScalarMetric {
                        metric: Metric::WeightKg,
                        target: *target,
                        stored: stored_weight,
                        current: latest(points),
                        observed: established_slope(points, signal.body.weight_slope_kg_per_week),
                        earliest: earliest(points),
                        enough: meets_body_floor(points),
                    },
                    goal,
                    today,
                ));
            }
            if let Some(target) = target_body_fat_pct {
                let points = &signal.body.body_fat_pct;
                rows.push(assess_scalar(
                    ScalarMetric {
                        metric: Metric::BodyFatPct,
                        target: *target,
                        stored: stored_bf,
                        current: latest(points),
                        observed: signal
                            .body
                            .body_fat_slope
                            .and_then(|s| established_slope(points, s)),
                        earliest: earliest(points),
                        enough: meets_body_floor(points),
                    },
                    goal,
                    today,
                ));
            }
            rows
        }
        GoalKind::Consistency { sessions_per_week } => vec![assess_consistency(
            *sessions_per_week,
            signal.adherence,
            goal,
            today,
        )],
    };

    // The pinned severity fold (§2.2.1): maximum severity wins, first such
    // metric breaking a tie. A (never-validated) goal tracking nothing has
    // no metrics and no signal.
    let status = metrics
        .iter()
        .fold(None::<&PaceStatus>, |acc, m| match acc {
            Some(s) if s.severity() >= m.status.severity() => Some(s),
            _ => Some(&m.status),
        })
        .cloned()
        .unwrap_or(PaceStatus::InsufficientData {
            reason: InsufficientReason::NoSignal,
        });

    PaceReport { status, metrics }
}

/// One scalar metric's inputs, all derivations already made by the caller.
struct ScalarMetric {
    metric: Metric,
    target: f64,
    /// The stored baseline for this metric, `None` when absent or when the
    /// stored [`Baseline`] variant does not match the goal kind.
    stored: Option<f64>,
    current: Option<f64>,
    /// The observed slope — `Some` only when a trend is established (§2.3).
    observed: Option<f64>,
    /// The earliest in-window observation — the trend's own starting position.
    /// For a late-bound goal this anchors *direction* (§2.2.3, amended): the
    /// trajectory earliest → current says which way the user is actually
    /// travelling, and unlike the slope's bare sign it carries the position
    /// reference needed to tell "overshot the target" from "never reached it".
    earliest: Option<f64>,
    /// Whether the per-kind observation floor is met (§2.3).
    enough: bool,
}

/// The §2.2.2 math for one scalar metric (strength, weight, body fat).
fn assess_scalar(m: ScalarMetric, goal: &Goal, today: NaiveDate) -> MetricPace {
    let weeks_remaining = weeks_between(today, goal.target_date).max(0.0);
    let base_eff = m.stored.or(m.current);
    let late_bound = m.stored.is_none();
    // Direction anchor (§2.2.3, amended after code review): a late-bound goal
    // must NOT infer direction from `current` — the moment current crosses the
    // target, `target > current` flips and the goal reinterprets itself as a
    // reduction goal, turning an overshoot into Behind and, at the deadline,
    // Missed. The earliest in-window observation is a stable position the
    // journey demonstrably started from, so direction survives the crossing.
    let base_dir = m.stored.or(m.earliest).or(m.current);

    // §2.2.4 — post-deadline is terminal: achieved iff the (deadline-
    // anchored) current demonstrably met the target; anything else is missed.
    if today > goal.target_date {
        let status = match (base_dir, m.current) {
            (Some(base), Some(current)) if target_met(m.target, base, current) => {
                PaceStatus::Achieved
            }
            _ => PaceStatus::Missed,
        };
        return MetricPace {
            metric: m.metric,
            status,
            baseline: m.stored,
            current: m.current,
            target: m.target,
            required_per_week: None,
            observed_per_week: m.observed,
            projected_at_deadline: None,
            weeks_remaining: 0.0,
        };
    }

    let row = |status: PaceStatus, observed: Option<f64>, projected: Option<f64>| MetricPace {
        metric: m.metric.clone(),
        status,
        baseline: m.stored,
        current: m.current,
        target: m.target,
        required_per_week: required_per_week(m.target, base_eff, late_bound, goal, weeks_remaining),
        observed_per_week: observed,
        projected_at_deadline: projected,
        weeks_remaining,
    };

    let Some(base) = base_eff else {
        return row(
            PaceStatus::InsufficientData {
                reason: InsufficientReason::NoSignal,
            },
            None,
            None,
        );
    };

    // Achieved is a current-state check and precedes the floors: an
    // already-met target needs no trend to be reported as met (§2.2.2).
    // Direction comes from `base_dir`, not `base` — see its definition.
    let dir_base = base_dir.unwrap_or(base);
    if let Some(current) = m.current {
        if target_met(m.target, dir_base, current) {
            let projected = m.observed.map(|o| o.mul_add(weeks_remaining, current));
            return row(PaceStatus::Achieved, m.observed, projected);
        }
    }

    // §2.3 — the data floors: below them, never a projection (AC5).
    let Some(current) = m.current.filter(|_| m.enough) else {
        return row(
            PaceStatus::InsufficientData {
                reason: InsufficientReason::TooFewObservations,
            },
            None,
            None,
        );
    };
    let Some(observed) = m.observed else {
        return row(
            PaceStatus::InsufficientData {
                reason: InsufficientReason::NoTrend,
            },
            None,
            None,
        );
    };

    // Direction by explicit comparison, never `signum` (§2.2.2), anchored at
    // `dir_base` (§2.2.3 amendment). Equality of target and anchor was decided
    // by the Achieved check above (with a current value present) or by the
    // floors (without one), so the two sides are genuinely apart here.
    let dir: f64 = if m.target > dir_base { 1.0 } else { -1.0 };
    let projected = observed.mul_add(weeks_remaining, current);
    let gap = dir * (projected - m.target);
    let required = required_per_week(m.target, base_eff, late_bound, goal, weeks_remaining);
    let band = required.map_or(0.0, |r| PACE_BAND_FRACTION * r.abs() * weeks_remaining);

    let status = if gap >= band {
        PaceStatus::Ahead
    } else if gap >= -band {
        PaceStatus::OnTrack
    } else {
        let stalled = (dir > 0.0 && observed <= 0.0) || (dir < 0.0 && observed >= 0.0);
        let most_remains = remaining_fraction(m.target, base, current, late_bound)
            .is_some_and(|f| f >= AT_RISK_REMAINING_FRACTION);
        if weeks_remaining < AT_RISK_FINAL_WEEKS || (stalled && most_remains) {
            PaceStatus::AtRisk
        } else {
            PaceStatus::Behind
        }
    };
    row(status, Some(observed), Some(projected))
}

/// §2.2.5 — the consistency metric: a trailing mean over completed ISO
/// weeks, no projection, `Achieved` only once the deadline has passed.
fn assess_consistency(
    sessions_per_week: u32,
    adherence: &Adherence,
    goal: &Goal,
    today: NaiveDate,
) -> MetricPace {
    let target = f64::from(sessions_per_week);
    let terminal = today > goal.target_date;
    let weeks_remaining = weeks_between(today, goal.target_date).max(0.0);
    // Anchor the week window at the deadline once it has passed, so the
    // terminal verdict is a stable function of history (§2.2.4).
    let anchor = today.min(goal.target_date);
    let mean = trailing_weekly_mean(adherence, goal.set_on, anchor);

    let status = match (mean, terminal) {
        (None, false) => PaceStatus::InsufficientData {
            reason: InsufficientReason::TooFewObservations,
        },
        (None, true) => PaceStatus::Missed,
        (Some(mean), true) => {
            if mean >= target {
                PaceStatus::Achieved
            } else {
                PaceStatus::Missed
            }
        }
        (Some(mean), false) => {
            let gap = mean - target;
            let band = PACE_BAND_FRACTION * target;
            if gap >= band {
                PaceStatus::Ahead
            } else if gap >= -band {
                PaceStatus::OnTrack
            } else if weeks_remaining < AT_RISK_FINAL_WEEKS {
                PaceStatus::AtRisk
            } else {
                PaceStatus::Behind
            }
        }
    };

    MetricPace {
        metric: Metric::SessionsPerWeek,
        status,
        baseline: None,
        current: mean,
        target,
        required_per_week: None,
        // The metric is itself a weekly rate: the trailing mean *is* the
        // observed pace, and there is nothing separate to project.
        observed_per_week: mean,
        projected_at_deadline: None,
        weeks_remaining,
    }
}

/// Mean training days per week over the trailing
/// `min(completed full ISO weeks since set_on, DEFAULT_WINDOW_WEEKS)` weeks,
/// counting weeks the aggregate omitted as zero. `None` before one completed
/// week exists. The week containing `anchor` is in progress and excluded, as
/// is a partial first week when `set_on` is not a Monday.
fn trailing_weekly_mean(
    adherence: &Adherence,
    set_on: NaiveDate,
    anchor: NaiveDate,
) -> Option<f64> {
    let current_monday = iso_week_monday(anchor);
    let first_full_monday = if iso_week_monday(set_on) == set_on {
        set_on
    } else {
        iso_week_monday(set_on) + Duration::weeks(1)
    };
    let completed = ((current_monday - first_full_monday).num_days() / 7).max(0);
    // Only weeks the aggregate window *fully* covers may be counted. The
    // window is the half-open (anchor − 7·W days, anchor], so the week
    // starting at `current_monday − k·7` is fully inside iff
    // `current_monday − k·7 > anchor − 7·W`, i.e. k ≤ (7·W − 1 − d) / 7 with
    // `d = anchor − current_monday ∈ [0, 6]`. Without this cap the oldest
    // counted week is partially (worst case almost entirely) outside the
    // window and its missing sessions read as zeros — a perfect 4/wk
    // sustainer showed a mean of 3.5 and, at a non-Monday deadline, Missed.
    let d = (anchor - current_monday).num_days();
    let fully_inside = (7 * i64::from(DEFAULT_WINDOW_WEEKS) - 1 - d) / 7;
    let n = completed.min(fully_inside);
    if n == 0 {
        return None;
    }

    let by_week: BTreeMap<NaiveDate, u32> = adherence.weekly_days.iter().copied().collect();
    let total: u32 = (1..=n)
        .map(|k| {
            by_week
                .get(&(current_monday - Duration::weeks(k)))
                .copied()
                .unwrap_or(0)
        })
        .sum();
    #[allow(clippy::cast_precision_loss)] // week counts are tiny
    Some(f64::from(total) / n as f64)
}

/// Whether `current` sits at or past `target`, with the direction inferred
/// from `base` by explicit comparison. A zero-change goal (`target == base`)
/// is met by construction — the "decides immediately" clause of §2.2.2.
fn target_met(target: f64, base: f64, current: f64) -> bool {
    if target > base {
        current >= target
    } else if target < base {
        current <= target
    } else {
        true
    }
}

/// The linear required pace (§2.2.2, OQ-1). Late-bound (§2.2.3): the pace
/// needed *from here* over the remaining weeks; stored: from the baseline
/// over the whole horizon. `None` when the divisor is not positive.
fn required_per_week(
    target: f64,
    base_eff: Option<f64>,
    late_bound: bool,
    goal: &Goal,
    weeks_remaining: f64,
) -> Option<f64> {
    let base = base_eff?;
    let weeks = if late_bound {
        weeks_remaining
    } else {
        weeks_between(goal.set_on, goal.target_date)
    };
    (weeks > 0.0).then(|| (target - base) / weeks)
}

/// `(target − current) / (target − baseline)` (§2.2.2, amended), or `None`
/// when the baseline is late-bound or the span is ~0 — the `AtRisk` arm it
/// feeds is then disabled rather than dividing by an unknown or zero span.
///
/// Signed over signed, deliberately: the original `dir × (target − current) /
/// span` reduced (since `dir == sign(span)`) to `(target − current) / |span|`,
/// which is *negative* whenever work remains on a downward goal — making the
/// stalled-`AtRisk` arm unreachable for every fat-loss/cut goal. Dividing the
/// signed remainder by the signed span yields the travelled-fraction
/// symmetrically in both directions.
fn remaining_fraction(target: f64, base: f64, current: f64, late_bound: bool) -> Option<f64> {
    if late_bound {
        return None;
    }
    let span = target - base;
    if span.abs() < SPAN_EPSILON {
        return None;
    }
    Some((target - current) / span)
}

/// The slope, but only when it is *established*: at least
/// [`MIN_TREND_DATES`] distinct-date points. R-0015's scalar alone cannot
/// distinguish a plateau from no data (§2.3).
fn established_slope(points: &[TrendPoint], slope: f64) -> Option<f64> {
    (distinct_dates(points) >= MIN_TREND_DATES).then_some(slope)
}

/// Distinct dates among date-sorted trend points.
fn distinct_dates(points: &[TrendPoint]) -> usize {
    let mut count = 0;
    let mut last: Option<NaiveDate> = None;
    for p in points {
        if last != Some(p.on) {
            count += 1;
            last = Some(p.on);
        }
    }
    count
}

/// The §2.3 body floor: enough points, spanning enough days.
fn meets_body_floor(points: &[TrendPoint]) -> bool {
    if points.len() < MIN_BODY_POINTS {
        return false;
    }
    match (points.first(), points.last()) {
        (Some(first), Some(last)) => (last.on - first.on).num_days() >= MIN_BODY_SPAN_DAYS,
        _ => false,
    }
}

/// The latest value of a date-sorted trend, if any.
fn latest(points: &[TrendPoint]) -> Option<f64> {
    points.last().map(|p| p.value)
}

/// The earliest in-window observation — the trend's starting position, used to
/// anchor a late-bound goal's direction (§2.2.3, amended). Trend vectors are
/// date-ascending by construction in `aggregate`.
fn earliest(points: &[TrendPoint]) -> Option<f64> {
    points.first().map(|p| p.value)
}

/// The goal's lift in the signal, matched by the canonical trimmed/lowercased
/// key every other lift lookup uses.
fn find_lift<'a>(lifts: &'a [LiftSummary], name: &str) -> Option<&'a LiftSummary> {
    let key = lift_key(name);
    lifts.iter().find(|l| lift_key(&l.name) == key)
}

/// The stored strength baseline, or `None` on a kind/baseline mismatch (a
/// state [`Baseline::capture`] never produces; `assess` stays total on it).
fn stored_strength(baseline: &Baseline) -> Option<f64> {
    match baseline {
        Baseline::Strength { e1rm_kg } => *e1rm_kg,
        Baseline::Body { .. } | Baseline::Consistency => None,
    }
}

/// The stored body baselines, or `(None, None)` on a kind mismatch.
fn stored_body(baseline: &Baseline) -> (Option<f64>, Option<f64>) {
    match baseline {
        Baseline::Body {
            weight_kg,
            body_fat_pct,
        } => (*weight_kg, *body_fat_pct),
        Baseline::Strength { .. } | Baseline::Consistency => (None, None),
    }
}

/// Days between two dates as (possibly fractional) weeks.
fn weeks_between(from: NaiveDate, to: NaiveDate) -> f64 {
    #[allow(clippy::cast_precision_loss)] // day counts are tiny
    let days = (to - from).num_days() as f64;
    days / 7.0
}

/// The Monday of the ISO week containing `d` (same derivation as R-0015).
fn iso_week_monday(d: NaiveDate) -> NaiveDate {
    d.week(Weekday::Mon).first_day()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    // A Monday; 20 weeks to the deadline; TODAY is 10 weeks in.
    const SET_ON: fn() -> NaiveDate = || d(2026, 1, 5);
    const DEADLINE: fn() -> NaiveDate = || d(2026, 5, 25);
    const TODAY: fn() -> NaiveDate = || d(2026, 3, 16);

    /// Owned signal parts a test borrows a [`GoalSignal`] from.
    struct Sig {
        lifts: Vec<LiftSummary>,
        body: BodyTrend,
        adherence: Adherence,
    }

    impl Sig {
        fn empty() -> Self {
            Self {
                lifts: vec![],
                body: BodyTrend {
                    weight: vec![],
                    weight_slope_kg_per_week: 0.0,
                    body_fat_pct: vec![],
                    body_fat_slope: None,
                    lean_mass: vec![],
                    lean_mass_slope: None,
                },
                adherence: Adherence {
                    weekly_days: vec![],
                    target_days_per_week: 0,
                    ratio: 0.0,
                },
            }
        }

        fn signal(&self) -> GoalSignal<'_> {
            GoalSignal {
                lifts: &self.lifts,
                body: &self.body,
                adherence: &self.adherence,
            }
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_precision_loss
    )] // tiny counts
    fn squat_lift(current: f64, slope: f64, n: usize) -> LiftSummary {
        let e1rm: Vec<TrendPoint> = (0..n)
            .map(|i| TrendPoint {
                on: d(2026, 3, 14) - Duration::weeks((n - 1 - i) as i64),
                value: slope.mul_add(-((n - 1 - i) as f64), current),
            })
            .collect();
        LiftSummary {
            name: "Squat".into(),
            current_e1rm: current,
            slope_kg_per_week: slope,
            sessions: n as u32,
            stalled: false,
            e1rm,
        }
    }

    /// `n` squat sessions on distinct weekly dates ending 2026-03-14.
    fn squat(current: f64, slope: f64, n: usize) -> Sig {
        let mut s = Sig::empty();
        s.lifts.push(squat_lift(current, slope, n));
        s
    }

    /// Three squat sessions on ONE date — observations without a trend.
    fn squat_same_day(current: f64) -> Sig {
        let mut s = Sig::empty();
        let on = d(2026, 3, 14);
        s.lifts.push(LiftSummary {
            name: "Squat".into(),
            e1rm: vec![
                TrendPoint { on, value: current },
                TrendPoint { on, value: current },
                TrendPoint { on, value: current },
            ],
            current_e1rm: current,
            slope_kg_per_week: 0.0,
            sessions: 3,
            stalled: false,
        });
        s
    }

    /// Weight points on the given dates; the trend slope is supplied, the
    /// current value is the last point.
    fn weight_sig_on(dates: &[NaiveDate], current: f64, slope: f64) -> Sig {
        let mut s = Sig::empty();
        s.body.weight = dates
            .iter()
            .map(|&on| TrendPoint { on, value: current })
            .collect();
        s.body.weight_slope_kg_per_week = slope;
        s
    }

    /// Three weight points spanning 28 days, ending 2026-03-14.
    fn weight_sig(current: f64, slope: f64) -> Sig {
        weight_sig_on(
            &[d(2026, 2, 14), d(2026, 2, 28), d(2026, 3, 14)],
            current,
            slope,
        )
    }

    fn adher(weeks: &[(NaiveDate, u32)]) -> Sig {
        let mut s = Sig::empty();
        s.adherence.weekly_days = weeks.to_vec();
        s
    }

    fn strength_goal(base: Option<f64>, target: f64) -> Goal {
        Goal {
            kind: GoalKind::Strength {
                lift: "Squat".into(),
                target_e1rm_kg: target,
            },
            baseline: Baseline::Strength { e1rm_kg: base },
            set_on: SET_ON(),
            target_date: DEADLINE(),
        }
    }

    fn weight_goal(base: Option<f64>, target: f64) -> Goal {
        Goal {
            kind: GoalKind::Body {
                target_weight_kg: Some(target),
                target_body_fat_pct: None,
            },
            baseline: Baseline::Body {
                weight_kg: base,
                body_fat_pct: None,
            },
            set_on: SET_ON(),
            target_date: DEADLINE(),
        }
    }

    fn consistency_goal(rate: u32) -> Goal {
        Goal {
            kind: GoalKind::Consistency {
                sessions_per_week: rate,
            },
            baseline: Baseline::Consistency,
            set_on: SET_ON(),
            target_date: DEADLINE(),
        }
    }

    fn insufficient(reason: InsufficientReason) -> PaceStatus {
        PaceStatus::InsufficientData { reason }
    }

    // -----------------------------------------------------------------------
    // Strength — the status matrix (§7). Base 100, target 140 over 20 weeks:
    // required 2 kg/wk; at TODAY (10 weeks left) the band is 0.25×2×10 = 5.
    // -----------------------------------------------------------------------

    #[test]
    #[allow(clippy::too_many_lines)]
    fn strength_status_matrix() {
        let cases: Vec<(&str, Goal, Sig, NaiveDate, PaceStatus)> = vec![
            (
                "ahead: projected 150 vs 140, gap 10 past the 5 band",
                strength_goal(Some(100.0), 140.0),
                squat(125.0, 2.5, 4),
                TODAY(),
                PaceStatus::Ahead,
            ),
            (
                "gap == +band exactly is Ahead (boundary)",
                strength_goal(Some(100.0), 140.0),
                squat(120.0, 2.5, 4),
                TODAY(),
                PaceStatus::Ahead,
            ),
            (
                "on track: projected dead on target",
                strength_goal(Some(100.0), 140.0),
                squat(120.0, 2.0, 4),
                TODAY(),
                PaceStatus::OnTrack,
            ),
            (
                "gap == -band exactly is OnTrack (boundary)",
                strength_goal(Some(100.0), 140.0),
                squat(120.0, 1.5, 4),
                TODAY(),
                PaceStatus::OnTrack,
            ),
            (
                "behind: right-signed pace, projected 120 vs 140",
                strength_goal(Some(100.0), 140.0),
                squat(110.0, 1.0, 4),
                TODAY(),
                PaceStatus::Behind,
            ),
            (
                "at risk: stalled with 75% of the span remaining",
                strength_goal(Some(100.0), 140.0),
                squat(110.0, 0.0, 4),
                TODAY(),
                PaceStatus::AtRisk,
            ),
            (
                "at risk: wrong-signed pace with most of the span remaining",
                strength_goal(Some(100.0), 140.0),
                squat(110.0, -0.5, 4),
                TODAY(),
                PaceStatus::AtRisk,
            ),
            (
                "stalled but only 20% remains (< 25%): Behind, not AtRisk",
                strength_goal(Some(100.0), 140.0),
                squat(132.0, 0.0, 4),
                TODAY(),
                PaceStatus::Behind,
            ),
            (
                "endgame: 10 kg short at 3 weeks out is Behind",
                strength_goal(Some(100.0), 140.0),
                squat(130.0, 1.0, 4),
                d(2026, 5, 4),
                PaceStatus::Behind,
            ),
            (
                "endgame: the same shortfall at 5 days out is AtRisk",
                strength_goal(Some(100.0), 140.0),
                squat(130.0, 1.0, 4),
                d(2026, 5, 20),
                PaceStatus::AtRisk,
            ),
            (
                "achieved early: current past the target",
                strength_goal(Some(100.0), 140.0),
                squat(141.0, 0.5, 4),
                TODAY(),
                PaceStatus::Achieved,
            ),
            (
                "achieved reverts on regression (current-state, not terminal)",
                strength_goal(Some(100.0), 140.0),
                squat(110.0, 0.0, 4),
                TODAY(),
                PaceStatus::AtRisk,
            ),
            (
                "achieved at creation: target == baseline == current",
                strength_goal(Some(140.0), 140.0),
                squat(140.0, 0.0, 1),
                SET_ON(),
                PaceStatus::Achieved,
            ),
            (
                "achieved bypasses the data floors (one session)",
                strength_goal(Some(100.0), 140.0),
                squat(141.0, 0.0, 1),
                TODAY(),
                PaceStatus::Achieved,
            ),
            (
                "missed: the deadline passed short of the target",
                strength_goal(Some(100.0), 140.0),
                squat(130.0, 1.0, 4),
                d(2026, 5, 26),
                PaceStatus::Missed,
            ),
            (
                "post-deadline achieved: the anchored current met the target",
                strength_goal(Some(100.0), 140.0),
                squat(141.0, 0.0, 4),
                d(2026, 6, 1),
                PaceStatus::Achieved,
            ),
            (
                "no signal: no stored baseline and no current value",
                strength_goal(None, 140.0),
                Sig::empty(),
                TODAY(),
                insufficient(InsufficientReason::NoSignal),
            ),
            (
                "baseline stored but the lift left the window: too few",
                strength_goal(Some(100.0), 140.0),
                Sig::empty(),
                TODAY(),
                insufficient(InsufficientReason::TooFewObservations),
            ),
            (
                "two sessions sit below the strength floor",
                strength_goal(Some(100.0), 140.0),
                squat(120.0, 2.0, 2),
                TODAY(),
                insufficient(InsufficientReason::TooFewObservations),
            ),
            (
                "three sessions on one date: observations without a trend",
                strength_goal(Some(100.0), 140.0),
                squat_same_day(120.0),
                TODAY(),
                insufficient(InsufficientReason::NoTrend),
            ),
            (
                "late-bound baseline: assessed from the current value",
                strength_goal(None, 140.0),
                squat(120.0, 2.0, 4),
                TODAY(),
                PaceStatus::OnTrack,
            ),
            (
                "late-bound: the remaining-fraction AtRisk arm is disabled",
                strength_goal(None, 140.0),
                squat(110.0, 0.0, 4),
                TODAY(),
                PaceStatus::Behind,
            ),
            (
                "late-bound below the floor is still insufficient",
                strength_goal(None, 140.0),
                squat(120.0, 2.0, 2),
                TODAY(),
                insufficient(InsufficientReason::TooFewObservations),
            ),
        ];

        for (name, goal, sig, today, expected) in cases {
            assert_eq!(
                assess(&goal, &sig.signal(), today).status,
                expected,
                "{name}"
            );
        }
    }

    /// The numbers behind an on-track verdict (AC4): every field of the row.
    #[test]
    fn strength_on_track_carries_the_numbers() {
        let goal = strength_goal(Some(100.0), 140.0);
        let sig = squat(120.0, 2.0, 4);
        let report = assess(&goal, &sig.signal(), TODAY());
        assert_eq!(report.metrics.len(), 1);
        let m = &report.metrics[0];
        assert_eq!(
            m.metric,
            Metric::E1rmKg {
                lift: "Squat".into()
            }
        );
        assert_eq!(m.status, PaceStatus::OnTrack);
        assert!((m.baseline.unwrap() - 100.0).abs() < 1e-9);
        assert!((m.current.unwrap() - 120.0).abs() < 1e-9);
        assert!((m.target - 140.0).abs() < 1e-9);
        assert!((m.required_per_week.unwrap() - 2.0).abs() < 1e-9);
        assert!((m.observed_per_week.unwrap() - 2.0).abs() < 1e-9);
        assert!((m.projected_at_deadline.unwrap() - 140.0).abs() < 1e-9);
        assert!((m.weeks_remaining - 10.0).abs() < 1e-9);
    }

    /// §2.2.3: a stored `None` baseline late-binds to the current value — the
    /// report carries `baseline: None` and the pace needed *from here*.
    #[test]
    fn late_bound_baseline_reports_none_and_pace_from_here() {
        let goal = strength_goal(None, 140.0);
        let sig = squat(120.0, 2.0, 4);
        let m = assess(&goal, &sig.signal(), TODAY()).metrics.remove(0);
        assert_eq!(m.baseline, None);
        assert!((m.current.unwrap() - 120.0).abs() < 1e-9);
        // (140 − 120) / 10 weeks remaining, not / 20 weeks total.
        assert!((m.required_per_week.unwrap() - 2.0).abs() < 1e-9);
    }

    /// §2.2.4: terminal reports carry no required pace and no projection, and
    /// the same expired goal assessed at two later dates is identical.
    #[test]
    fn post_deadline_is_terminal_and_stable() {
        let goal = strength_goal(Some(100.0), 140.0);
        let sig = squat(130.0, 1.0, 4);
        let first = assess(&goal, &sig.signal(), d(2026, 5, 26));
        let second = assess(&goal, &sig.signal(), d(2026, 6, 25));
        assert_eq!(first, second, "terminal status must not drift with time");
        let m = &first.metrics[0];
        assert_eq!(m.status, PaceStatus::Missed);
        assert_eq!(m.required_per_week, None);
        assert_eq!(m.projected_at_deadline, None);
        assert!(m.weeks_remaining.abs() < f64::EPSILON);
    }

    /// The goal's lift matches the signal case-insensitively (canonical key).
    #[test]
    fn lift_lookup_is_case_insensitive() {
        let mut goal = strength_goal(Some(100.0), 140.0);
        goal.kind = GoalKind::Strength {
            lift: "  sQuAt ".into(),
            target_e1rm_kg: 140.0,
        };
        let sig = squat(141.0, 0.5, 4);
        assert_eq!(
            assess(&goal, &sig.signal(), TODAY()).status,
            PaceStatus::Achieved
        );
    }

    // -----------------------------------------------------------------------
    // Body — direction-awareness (AC7) and the per-metric fold (§2.2.1).
    // Base 90, target 80 over 20 weeks: required −0.5 kg/wk; band at TODAY is
    // 0.25×0.5×10 = 1.25.
    // -----------------------------------------------------------------------

    #[test]
    fn body_status_matrix() {
        let cases: Vec<(&str, Goal, Sig, NaiveDate, PaceStatus)> = vec![
            (
                // Re-derived after the remaining_fraction sign fix (§9 code
                // review, finding 2): weight *rising* on a cut with more than
                // the whole journey remaining is wrong-signed pace + most-
                // remains — the AtRisk arm this case previously could not
                // reach. Behind would understate it.
                "fat loss: rising weight is AtRisk, not Ahead (dir = −1)",
                weight_goal(Some(90.0), 80.0),
                weight_sig(91.0, 0.3),
                TODAY(),
                PaceStatus::AtRisk,
            ),
            (
                "fat loss on track: falling at the required rate",
                weight_goal(Some(90.0), 80.0),
                weight_sig(85.0, -0.5),
                TODAY(),
                PaceStatus::OnTrack,
            ),
            (
                "fat loss ahead: falling faster than required",
                weight_goal(Some(90.0), 80.0),
                weight_sig(84.0, -0.6),
                TODAY(),
                PaceStatus::Ahead,
            ),
            (
                "fat loss achieved: current at or below the target",
                weight_goal(Some(90.0), 80.0),
                weight_sig(79.5, -0.5),
                TODAY(),
                PaceStatus::Achieved,
            ),
            (
                "three points inside 14 days sit below the body floor",
                weight_goal(Some(90.0), 80.0),
                weight_sig_on(&[d(2026, 3, 4), d(2026, 3, 9), d(2026, 3, 14)], 85.0, -0.5),
                TODAY(),
                insufficient(InsufficientReason::TooFewObservations),
            ),
        ];
        for (name, goal, sig, today, expected) in cases {
            assert_eq!(
                assess(&goal, &sig.signal(), today).status,
                expected,
                "{name}"
            );
        }
    }

    /// §2.2.1: a two-metric Body goal reports per metric and folds the overall
    /// status to the most severe — while the healthy metric keeps its numbers.
    #[test]
    fn body_two_metrics_fold_to_the_most_severe() {
        let goal = Goal {
            kind: GoalKind::Body {
                target_weight_kg: Some(80.0),
                target_body_fat_pct: Some(15.0),
            },
            baseline: Baseline::Body {
                weight_kg: Some(90.0),
                body_fat_pct: Some(20.0),
            },
            set_on: SET_ON(),
            target_date: DEADLINE(),
        };
        let mut sig = weight_sig(85.0, -0.5); // weight: OnTrack
        sig.body.body_fat_pct = vec![
            TrendPoint {
                on: d(2026, 3, 7),
                value: 19.0,
            },
            TrendPoint {
                on: d(2026, 3, 14),
                value: 18.8,
            },
        ]; // two bf points: below the floor
        sig.body.body_fat_slope = Some(-0.2);

        let report = assess(&goal, &sig.signal(), TODAY());
        assert_eq!(
            report.status,
            insufficient(InsufficientReason::TooFewObservations),
            "the fold takes the most severe metric"
        );
        assert_eq!(report.metrics.len(), 2);
        let weight = &report.metrics[0];
        assert_eq!(weight.metric, Metric::WeightKg);
        assert_eq!(weight.status, PaceStatus::OnTrack);
        assert!(
            weight.projected_at_deadline.is_some(),
            "the healthy metric still carries its full numbers"
        );
        let bf = &report.metrics[1];
        assert_eq!(bf.metric, Metric::BodyFatPct);
        assert_eq!(
            bf.status,
            insufficient(InsufficientReason::TooFewObservations)
        );
        assert_eq!(
            bf.projected_at_deadline, None,
            "never a fabricated projection"
        );
    }

    /// `assess` is total even for a (never-validated) Body goal with no
    /// target fields: an empty metrics list and an honest overall status.
    #[test]
    fn body_with_no_targets_is_total_not_a_panic() {
        let goal = Goal {
            kind: GoalKind::Body {
                target_weight_kg: None,
                target_body_fat_pct: None,
            },
            baseline: Baseline::Body {
                weight_kg: None,
                body_fat_pct: None,
            },
            set_on: SET_ON(),
            target_date: DEADLINE(),
        };
        let sig = Sig::empty();
        let report = assess(&goal, &sig.signal(), TODAY());
        assert!(report.metrics.is_empty());
        assert_eq!(report.status, insufficient(InsufficientReason::NoSignal));
    }

    // -----------------------------------------------------------------------
    // Consistency (§2.2.5) — trailing completed weeks, absent weeks zero.
    // -----------------------------------------------------------------------

    /// The in-progress ISO week is excluded even when it is the busiest one.
    #[test]
    fn consistency_excludes_the_partial_week() {
        let goal = consistency_goal(4);
        // Four completed 4-day weeks, then 7 days logged in the CURRENT week.
        let sig = adher(&[
            (d(2026, 1, 5), 4),
            (d(2026, 1, 12), 4),
            (d(2026, 1, 19), 4),
            (d(2026, 1, 26), 4),
            (d(2026, 2, 2), 7),
        ]);
        let today = d(2026, 2, 4); // Wednesday of the week of Feb 2
        let m = assess(&goal, &sig.signal(), today).metrics.remove(0);
        assert!(
            (m.current.unwrap() - 4.0).abs() < 1e-9,
            "the 7-day partial week must not raise the mean"
        );
        assert_eq!(m.status, PaceStatus::OnTrack);
    }

    /// Weeks absent from the aggregate (it omits empty weeks) count as zero.
    #[test]
    fn consistency_counts_absent_weeks_as_zero() {
        let goal = consistency_goal(4);
        let sig = adher(&[(d(2026, 1, 26), 4)]); // 1 of 4 completed weeks
        let today = d(2026, 2, 4);
        let m = assess(&goal, &sig.signal(), today).metrics.remove(0);
        assert!((m.current.unwrap() - 1.0).abs() < 1e-9, "mean is 4/4 weeks");
        assert_eq!(m.status, PaceStatus::Behind);
    }

    /// A goal older than the aggregate window means only the trailing
    /// `DEFAULT_WINDOW_WEEKS` completed weeks — a strong first month cannot
    /// mask a collapse.
    #[test]
    fn consistency_caps_at_the_window() {
        let goal = consistency_goal(4);
        // Strong January (outside the trailing 8 completed weeks), then nothing.
        let sig = adher(&[
            (d(2026, 1, 5), 4),
            (d(2026, 1, 12), 4),
            (d(2026, 1, 19), 4),
            (d(2026, 1, 26), 4),
        ]);
        let today = d(2026, 4, 1); // 12 completed weeks since set_on → cap at 8
        let m = assess(&goal, &sig.signal(), today).metrics.remove(0);
        assert!(
            m.current.unwrap().abs() < 1e-9,
            "the trailing 8 completed weeks are all empty"
        );
        assert_eq!(m.status, PaceStatus::Behind);
    }

    /// Sustaining a rate cannot be finished early: the pre-deadline ceiling
    /// is `Ahead`.
    #[test]
    fn consistency_never_achieves_early() {
        let goal = consistency_goal(4);
        let sig = adher(&[
            (d(2026, 1, 5), 6),
            (d(2026, 1, 12), 6),
            (d(2026, 1, 19), 6),
            (d(2026, 1, 26), 6),
        ]);
        let m = assess(&goal, &sig.signal(), d(2026, 2, 4))
            .metrics
            .remove(0);
        assert_eq!(
            m.status,
            PaceStatus::Ahead,
            "6/wk vs 4/wk is Ahead, never early Achieved"
        );
    }

    /// In the final stretch a below-band mean is `AtRisk`, not `Behind`.
    #[test]
    fn consistency_final_week_shortfall_is_at_risk() {
        let goal = consistency_goal(4);
        let weeks: Vec<(NaiveDate, u32)> = (0..8)
            .map(|k| (d(2026, 3, 23) + Duration::weeks(k), 2))
            .collect();
        let sig = adher(&weeks);
        let m = assess(&goal, &sig.signal(), d(2026, 5, 20))
            .metrics
            .remove(0);
        assert_eq!(m.status, PaceStatus::AtRisk);
    }

    /// Terminal consistency: achieved iff the deadline-anchored mean met the
    /// rate; stable at any later assessment date.
    #[test]
    fn consistency_terminal_achieved_and_missed() {
        let goal = consistency_goal(4);
        let full: Vec<(NaiveDate, u32)> = (0..8)
            .map(|k| (d(2026, 3, 30) + Duration::weeks(k), 4))
            .collect();
        let sustained = adher(&full);
        let first = assess(&goal, &sustained.signal(), d(2026, 6, 10));
        assert_eq!(first.status, PaceStatus::Achieved);
        assert_eq!(
            first,
            assess(&goal, &sustained.signal(), d(2026, 7, 1)),
            "terminal reports are stable"
        );

        let short: Vec<(NaiveDate, u32)> = (0..8)
            .map(|k| (d(2026, 3, 30) + Duration::weeks(k), 3))
            .collect();
        let lapsed = adher(&short);
        assert_eq!(
            assess(&goal, &lapsed.signal(), d(2026, 6, 10)).status,
            PaceStatus::Missed
        );
    }

    /// Before one completed ISO week exists there is nothing to average.
    #[test]
    fn consistency_needs_one_completed_week() {
        let mut goal = consistency_goal(4);
        goal.set_on = d(2026, 1, 7); // a Wednesday
        let sig = adher(&[(d(2026, 1, 5), 3)]);
        assert_eq!(
            assess(&goal, &sig.signal(), d(2026, 1, 9)).status,
            insufficient(InsufficientReason::TooFewObservations)
        );
    }

    // -----------------------------------------------------------------------
    // Severity order (§2.2.1) — pinned, encoded as an ordering.
    // -----------------------------------------------------------------------

    #[test]
    fn severity_order_is_pinned() {
        let order = [
            PaceStatus::Achieved,
            PaceStatus::Ahead,
            PaceStatus::OnTrack,
            PaceStatus::Behind,
            PaceStatus::AtRisk,
            PaceStatus::Missed,
            insufficient(InsufficientReason::NoSignal),
        ];
        assert!(
            order.windows(2).all(|w| w[0].severity() < w[1].severity()),
            "Achieved < Ahead < OnTrack < Behind < AtRisk < Missed < InsufficientData"
        );
        assert_eq!(
            insufficient(InsufficientReason::TooFewObservations).severity(),
            insufficient(InsufficientReason::NoTrend).severity(),
            "the reason does not change the severity"
        );
    }

    // -----------------------------------------------------------------------
    // Baseline capture (§2.6)
    // -----------------------------------------------------------------------

    fn summary_from(sig: &Sig) -> TrainingSummary {
        TrainingSummary {
            window_weeks: DEFAULT_WINDOW_WEEKS,
            generated_for: TODAY(),
            lifts: sig.lifts.clone(),
            muscle_volume: vec![],
            adherence: sig.adherence.clone(),
            body: sig.body.clone(),
        }
    }

    #[test]
    fn capture_takes_the_measured_values() {
        let sig = squat(120.0, 2.0, 4);
        let summary = summary_from(&sig);

        let kind = GoalKind::Strength {
            lift: "SQUAT".into(), // canonical-key match, not string equality
            target_e1rm_kg: 140.0,
        };
        assert_eq!(
            Baseline::capture(&kind, &summary),
            Baseline::Strength {
                e1rm_kg: Some(120.0)
            }
        );

        let unknown = GoalKind::Strength {
            lift: "Deadlift".into(),
            target_e1rm_kg: 200.0,
        };
        assert_eq!(
            Baseline::capture(&unknown, &summary),
            Baseline::Strength { e1rm_kg: None }
        );
    }

    #[test]
    fn capture_body_fills_only_the_targeted_fields() {
        let mut sig = weight_sig(85.0, -0.5);
        sig.body.body_fat_pct = vec![TrendPoint {
            on: d(2026, 3, 14),
            value: 18.0,
        }];
        let summary = summary_from(&sig);

        let weight_only = GoalKind::Body {
            target_weight_kg: Some(80.0),
            target_body_fat_pct: None,
        };
        assert_eq!(
            Baseline::capture(&weight_only, &summary),
            Baseline::Body {
                weight_kg: Some(85.0),
                body_fat_pct: None,
            }
        );

        let both = GoalKind::Body {
            target_weight_kg: Some(80.0),
            target_body_fat_pct: Some(15.0),
        };
        assert_eq!(
            Baseline::capture(&both, &summary),
            Baseline::Body {
                weight_kg: Some(85.0),
                body_fat_pct: Some(18.0),
            }
        );

        let consistency = GoalKind::Consistency {
            sessions_per_week: 3,
        };
        assert_eq!(
            Baseline::capture(&consistency, &summary),
            Baseline::Consistency
        );
    }

    // -----------------------------------------------------------------------
    // Validation (§2.4) — every variant reachable, each kind accepted.
    // -----------------------------------------------------------------------

    #[test]
    #[allow(clippy::too_many_lines)]
    fn every_goal_error_variant_is_reachable() {
        let strength = |lift: &str, target: f64| GoalKind::Strength {
            lift: lift.into(),
            target_e1rm_kg: target,
        };
        let body = |w: Option<f64>, bf: Option<f64>| GoalKind::Body {
            target_weight_kg: w,
            target_body_fat_pct: bf,
        };
        let set_on = SET_ON();
        let ok_date = DEADLINE();

        let cases: Vec<(&str, GoalKind, NaiveDate, GoalError)> = vec![
            (
                "target date on the creation date",
                strength("Squat", 140.0),
                set_on,
                GoalError::TargetDateNotAfterCreation,
            ),
            (
                "target date beyond the horizon",
                strength("Squat", 140.0),
                set_on + Duration::days(MAX_GOAL_HORIZON_DAYS + 1),
                GoalError::TargetDateTooFar,
            ),
            (
                "NaN target",
                strength("Squat", f64::NAN),
                ok_date,
                GoalError::NonFiniteTarget,
            ),
            (
                "infinite target",
                strength("Squat", f64::INFINITY),
                ok_date,
                GoalError::NonFiniteTarget,
            ),
            (
                "zero target",
                strength("Squat", 0.0),
                ok_date,
                GoalError::NonPositiveTarget,
            ),
            (
                "absurd e1RM target",
                strength("Squat", MAX_TARGET_E1RM_KG + 1.0),
                ok_date,
                GoalError::TargetTooLarge,
            ),
            (
                "absurd weight target",
                body(Some(MAX_TARGET_WEIGHT_KG + 1.0), None),
                ok_date,
                GoalError::TargetTooLarge,
            ),
            (
                "body fat at 75 is out of the open range",
                body(None, Some(MAX_TARGET_BODY_FAT_PCT)),
                ok_date,
                GoalError::BodyFatOutOfRange,
            ),
            (
                "body goal with neither field",
                body(None, None),
                ok_date,
                GoalError::BodyTargetEmpty,
            ),
            (
                "blank lift",
                strength("   ", 140.0),
                ok_date,
                GoalError::BlankLift,
            ),
            (
                "over-length lift",
                strength(&"x".repeat(MAX_LIFT_CHARS + 1), 140.0),
                ok_date,
                GoalError::LiftTooLong,
            ),
            (
                "zero sessions",
                GoalKind::Consistency {
                    sessions_per_week: 0,
                },
                ok_date,
                GoalError::ZeroSessions,
            ),
            (
                "absurd session rate",
                GoalKind::Consistency {
                    sessions_per_week: MAX_SESSIONS_PER_WEEK + 1,
                },
                ok_date,
                GoalError::TooManySessions,
            ),
        ];
        for (name, kind, target_date, expected) in cases {
            assert_eq!(
                validate(&kind, set_on, target_date),
                Err(expected),
                "{name}"
            );
        }

        assert!(validate(&strength("Squat", 140.0), set_on, ok_date).is_ok());
        assert!(validate(&body(Some(80.0), Some(15.0)), set_on, ok_date).is_ok());
        assert!(validate(
            &GoalKind::Consistency {
                sessions_per_week: MAX_SESSIONS_PER_WEEK
            },
            set_on,
            ok_date
        )
        .is_ok());
        assert!(
            validate(
                &strength("Squat", 140.0),
                set_on,
                set_on + Duration::days(1)
            )
            .is_ok(),
            "one day out is a valid horizon"
        );
    }

    // -----------------------------------------------------------------------
    // Serde — the wire contract round-trips.
    // -----------------------------------------------------------------------

    #[test]
    fn goal_and_report_round_trip_through_json() {
        let goal = strength_goal(Some(100.0), 140.0);
        let json = serde_json::to_string(&goal).unwrap();
        assert!(json.contains("\"kind\":\"strength\""));
        let back: Goal = serde_json::from_str(&json).unwrap();
        assert_eq!(back, goal);

        let sig = squat(110.0, 0.0, 4);
        let report = assess(&goal, &sig.signal(), TODAY());
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"status\":\"at_risk\""));
        let back: PaceReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back, report);

        let none = assess(&strength_goal(None, 140.0), &Sig::empty().signal(), TODAY());
        let json = serde_json::to_string(&none).unwrap();
        assert!(json.contains("\"status\":\"insufficient_data\""));
        assert!(json.contains("\"reason\":\"no_signal\""));
        let back: PaceReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back, none);
    }
    // -----------------------------------------------------------------------
    // Code-review round (SPEC-0042 §9 addendum): findings 1, 2, 3, 5
    // -----------------------------------------------------------------------

    /// Finding 1: a goal set before any data (`baseline: None`) whose user
    /// then trains PAST the target must be Achieved — not flip to a phantom
    /// reduction goal. Direction anchors at the earliest in-window point.
    #[test]
    fn late_bound_overshoot_is_achieved_not_behind() {
        let sig = squat(150.0, 5.0, 8); // earliest in window = 115
        let goal = strength_goal(None, 140.0);
        let report = assess(&goal, &sig.signal(), TODAY());
        assert_eq!(report.status, PaceStatus::Achieved);

        // And terminally: with the same (deadline-anchored) signal the
        // verdict stays Achieved — the crossing does not read as Missed.
        let report = assess(&goal, &sig.signal(), DEADLINE() + Duration::weeks(3));
        assert_eq!(report.status, PaceStatus::Achieved);
    }

    /// The guard the naive "direction = slope sign" rule would break: a user
    /// mid-cut (falling trend) with a target ABOVE them has not achieved
    /// anything — the trajectory never reached the target from below.
    #[test]
    fn late_bound_falling_trend_below_target_is_not_falsely_achieved() {
        let sig = squat(120.0, -2.0, 8); // earliest 134, all below target
        let goal = strength_goal(None, 140.0);
        let report = assess(&goal, &sig.signal(), TODAY());
        assert_eq!(report.status, PaceStatus::Behind);
    }

    /// Late-bound in the downward direction: journey started above the
    /// target (earliest 84) and crossed below it — Achieved.
    #[test]
    fn late_bound_downward_crossing_is_achieved() {
        let mut sig = Sig::empty();
        sig.body.weight = vec![
            TrendPoint {
                on: d(2026, 2, 14),
                value: 84.0,
            },
            TrendPoint {
                on: d(2026, 2, 28),
                value: 81.0,
            },
            TrendPoint {
                on: d(2026, 3, 14),
                value: 78.0,
            },
        ];
        sig.body.weight_slope_kg_per_week = -1.5;
        let goal = weight_goal(None, 80.0);
        let report = assess(&goal, &sig.signal(), TODAY());
        assert_eq!(report.status, PaceStatus::Achieved);
    }

    /// Finding 2: the downward mirror of the pinned upward stalled case.
    /// Base 90 → target 80, current 89, pace fully stalled, 90 % of the
    /// journey remaining: `AtRisk`, exactly as the upward twin is.
    #[test]
    fn downward_stalled_goal_with_most_remaining_is_at_risk() {
        let sig = weight_sig(89.0, 0.0);
        let goal = weight_goal(Some(90.0), 80.0);
        let report = assess(&goal, &sig.signal(), TODAY());
        assert_eq!(report.status, PaceStatus::AtRisk);
    }

    /// Finding 3: a perfect 4/wk sustainer whose deadline is a Friday. The
    /// oldest trailing week is only partially covered by the aggregate window,
    /// so counting it (as zero) diluted the mean to 3.5 and reported Missed.
    #[test]
    fn consistency_sustainer_with_non_monday_deadline_achieves() {
        let deadline = d(2026, 5, 22); // a Friday
        let mondays: Vec<(NaiveDate, u32)> = (0..7)
            .map(|k| (d(2026, 5, 11) - Duration::weeks(k), 4))
            .collect();
        let sig = adher(&mondays);
        let goal = Goal {
            kind: GoalKind::Consistency {
                sessions_per_week: 4,
            },
            baseline: Baseline::Consistency,
            set_on: SET_ON(),
            target_date: deadline,
        };
        let report = assess(&goal, &sig.signal(), deadline + Duration::days(4));
        assert_eq!(report.status, PaceStatus::Achieved);
        assert_eq!(report.metrics[0].current, Some(4.0));
    }

    /// Finding 5: Body rows the matrix was missing — terminal Missed.
    #[test]
    fn body_goal_past_deadline_short_of_target_is_missed() {
        let sig = weight_sig(85.0, -0.5);
        let goal = weight_goal(Some(90.0), 80.0);
        let report = assess(&goal, &sig.signal(), DEADLINE() + Duration::days(1));
        assert_eq!(report.status, PaceStatus::Missed);
    }

    /// Finding 5: Body with no observations at all — `NoSignal`, not a guess.
    #[test]
    fn body_goal_with_no_data_is_no_signal() {
        let sig = Sig::empty();
        let goal = weight_goal(None, 80.0);
        let report = assess(&goal, &sig.signal(), TODAY());
        assert_eq!(
            report.status,
            PaceStatus::InsufficientData {
                reason: InsufficientReason::NoSignal
            }
        );
    }

    /// Finding 5: enough body-fat observations but no computable slope —
    /// `NoTrend`, the reason R-0015's `body_fat_slope` is an `Option`.
    #[test]
    fn body_fat_goal_without_a_slope_is_no_trend() {
        let mut sig = Sig::empty();
        sig.body.body_fat_pct = vec![
            TrendPoint {
                on: d(2026, 2, 14),
                value: 20.0,
            },
            TrendPoint {
                on: d(2026, 2, 28),
                value: 19.0,
            },
            TrendPoint {
                on: d(2026, 3, 14),
                value: 18.0,
            },
        ];
        sig.body.body_fat_slope = None;
        let goal = Goal {
            kind: GoalKind::Body {
                target_weight_kg: None,
                target_body_fat_pct: Some(15.0),
            },
            baseline: Baseline::Body {
                weight_kg: None,
                body_fat_pct: Some(20.0),
            },
            set_on: SET_ON(),
            target_date: DEADLINE(),
        };
        let report = assess(&goal, &sig.signal(), TODAY());
        assert_eq!(
            report.status,
            PaceStatus::InsufficientData {
                reason: InsufficientReason::NoTrend
            }
        );
    }
}
