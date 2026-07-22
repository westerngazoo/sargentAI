//! R-0015 / SPEC-0015 — training-log aggregation.
//!
//! Pure functions that turn a user's raw logs into a typed, deterministic
//! [`TrainingSummary`] — the feature layer the adjustment engine (R-0017) and
//! the future learned model (R-0016) both read. No I/O, no clock (today's date
//! is a parameter), no recommendations, no model.

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

use crate::workout::{LoadKg, MuscleGroup, WorkoutSession};

/// No new e1RM peak within this many trailing sessions ⇒ a stall.
const STALL_N: usize = 3;
/// A gain at or below this (kg) doesn't count as progress.
const EPS_KG: f64 = 0.5;

/// A body-measurement sample, mapped from the R-0034 rows at the API edge so
/// `core` need not know the database shape.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyPoint {
    pub on: NaiveDate,
    pub weight_kg: f64,
    pub body_fat_pct: Option<f64>,
}

impl BodyPoint {
    /// Lean mass = weight × (1 − bf/100); `None` when body fat is unknown.
    #[must_use]
    pub fn lean_mass_kg(&self) -> Option<f64> {
        self.body_fat_pct
            .map(|bf| self.weight_kg * (1.0 - bf / 100.0))
    }
}

/// One `(date, value)` sample of a trend line, oldest first.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrendPoint {
    pub on: NaiveDate,
    pub value: f64,
}

/// One lift's in-window strength picture.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiftSummary {
    pub name: String,
    pub e1rm: Vec<TrendPoint>,
    pub current_e1rm: f64,
    pub slope_kg_per_week: f64,
    pub sessions: u32,
    pub stalled: bool,
}

/// Weekly training volume for one muscle group (ISO-week Monday → totals).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MuscleVolume {
    pub group: MuscleGroup,
    pub weekly_sets: Vec<(NaiveDate, u32)>,
    pub weekly_tonnage: Vec<(NaiveDate, f64)>,
    pub mean_weekly_sets: f64,
    pub mean_weekly_tonnage: f64,
}

/// Training frequency vs the program's target.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Adherence {
    pub weekly_days: Vec<(NaiveDate, u32)>,
    pub target_days_per_week: u32,
    pub ratio: f64,
}

/// Body-composition trends over the window.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyTrend {
    pub weight: Vec<TrendPoint>,
    pub weight_slope_kg_per_week: f64,
    pub body_fat_pct: Vec<TrendPoint>,
    pub body_fat_slope: Option<f64>,
    pub lean_mass: Vec<TrendPoint>,
    pub lean_mass_slope: Option<f64>,
}

/// The complete feature summary for one user over one window.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrainingSummary {
    pub window_weeks: u32,
    pub generated_for: NaiveDate,
    pub lifts: Vec<LiftSummary>,
    pub muscle_volume: Vec<MuscleVolume>,
    pub adherence: Adherence,
    pub body: BodyTrend,
}

/// Aggregate a user's logs into a [`TrainingSummary`] over the last
/// `window_weeks`. Pure and deterministic: same inputs ⇒ same output.
#[must_use]
pub fn summarize(
    today: NaiveDate,
    window_weeks: u32,
    sessions: &[WorkoutSession],
    measurements: &[BodyPoint],
    target_days_per_week: u32,
) -> TrainingSummary {
    let start = today - Duration::weeks(i64::from(window_weeks));
    let in_window: Vec<&WorkoutSession> = sessions
        .iter()
        .filter(|s| s.performed_on > start && s.performed_on <= today)
        .collect();

    TrainingSummary {
        window_weeks,
        generated_for: today,
        lifts: per_lift(&in_window),
        muscle_volume: per_muscle(&in_window),
        adherence: adherence(&in_window, target_days_per_week, window_weeks),
        body: body_trend(measurements, today, start),
    }
}

fn epley(reps: i32, weight_kg: f64) -> f64 {
    weight_kg * (1.0 + f64::from(reps) / 30.0)
}

/// Ordinary-least-squares slope of `value` on `x` (already in the desired unit,
/// e.g. weeks). `0.0` when fewer than two points or `x` has no spread.
#[allow(clippy::cast_precision_loss)] // point counts are tiny
fn slope(points: &[(f64, f64)]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }
    let n = points.len() as f64;
    let (mut sx, mut sy, mut sxx, mut sxy) = (0.0, 0.0, 0.0, 0.0);
    for &(x, y) in points {
        sx += x;
        sy += y;
        sxx += x * x;
        sxy += x * y;
    }
    let denom = n * sxx - sx * sx;
    if denom.abs() < f64::EPSILON {
        return 0.0;
    }
    (n * sxy - sx * sy) / denom
}

/// Slope of a trend line in value-units per week (x = days-since-epoch / 7).
fn weekly_slope(points: &[TrendPoint]) -> f64 {
    let xy: Vec<(f64, f64)> = points
        .iter()
        .map(|p| (f64::from(p.on.num_days_from_ce()) / 7.0, p.value))
        .collect();
    slope(&xy)
}

fn iso_week_monday(d: NaiveDate) -> NaiveDate {
    d.week(Weekday::Mon).first_day()
}

#[allow(clippy::cast_possible_truncation)] // session counts are tiny
fn per_lift(sessions: &[&WorkoutSession]) -> Vec<LiftSummary> {
    use std::collections::BTreeMap;
    let mut sorted: Vec<&&WorkoutSession> = sessions.iter().collect();
    sorted.sort_by_key(|s| s.performed_on);

    // key (lowercased name) -> (display, points)
    let mut by_lift: BTreeMap<String, (String, Vec<TrendPoint>)> = BTreeMap::new();
    for s in &sorted {
        // Best e1RM for each lift within this session.
        let mut best: BTreeMap<String, (String, f64)> = BTreeMap::new();
        for ex in &s.exercises {
            let name = ex.name.as_str().trim();
            if name.is_empty() {
                continue;
            }
            let key = name.to_lowercase();
            for set in &ex.sets {
                let Some(w) = set.weight_kg.map(LoadKg::get) else {
                    continue;
                };
                if w <= 0.0 {
                    continue;
                }
                let e = epley(set.reps.get(), w);
                let entry = best.entry(key.clone()).or_insert((name.to_string(), e));
                if e > entry.1 {
                    entry.1 = e;
                }
            }
        }
        for (key, (display, value)) in best {
            let bucket = by_lift.entry(key).or_insert((display, Vec::new()));
            bucket.1.push(TrendPoint {
                on: s.performed_on,
                value,
            });
        }
    }

    let mut out: Vec<LiftSummary> = by_lift
        .into_iter()
        .filter(|(_, (_, pts))| !pts.is_empty())
        .map(|(_, (name, e1rm))| {
            let current = e1rm.last().map_or(0.0, |p| p.value);
            LiftSummary {
                slope_kg_per_week: weekly_slope(&e1rm),
                stalled: is_stalled(&e1rm),
                current_e1rm: current,
                sessions: e1rm.len() as u32,
                name,
                e1rm,
            }
        })
        .collect();
    // Most sessions first, then biggest gain.
    out.sort_by(|a, b| {
        b.sessions
            .cmp(&a.sessions)
            .then(gain(b).total_cmp(&gain(a)))
    });
    out
}

fn gain(l: &LiftSummary) -> f64 {
    match (l.e1rm.first(), l.e1rm.last()) {
        (Some(f), Some(t)) if l.e1rm.len() >= 2 => t.value - f.value,
        _ => 0.0,
    }
}

/// A stall = the latest session set no new peak within the last `STALL_N`.
fn is_stalled(e1rm: &[TrendPoint]) -> bool {
    if e1rm.len() < STALL_N + 1 {
        return false;
    }
    let current = e1rm[e1rm.len() - 1].value;
    let prior_peak = e1rm[e1rm.len() - 1 - STALL_N..e1rm.len() - 1]
        .iter()
        .map(|p| p.value)
        .fold(f64::MIN, f64::max);
    current - prior_peak <= EPS_KG
}

#[allow(clippy::cast_precision_loss)] // week counts are tiny
fn per_muscle(sessions: &[&WorkoutSession]) -> Vec<MuscleVolume> {
    use std::collections::BTreeMap;
    // canonical-name (Ord, for deterministic order) -> (group, week -> (sets, tonnage))
    type Weeks = BTreeMap<NaiveDate, (u32, f64)>;
    let mut acc: BTreeMap<&'static str, (MuscleGroup, Weeks)> = BTreeMap::new();
    for s in sessions {
        let week = iso_week_monday(s.performed_on);
        for ex in &s.exercises {
            let Some(group) = ex.muscle_group else {
                continue;
            };
            for set in &ex.sets {
                let Some(w) = set.weight_kg.map(LoadKg::get) else {
                    continue;
                };
                if w <= 0.0 {
                    continue;
                }
                let (_, weeks) = acc.entry(group.as_str()).or_insert((group, Weeks::new()));
                let cell = weeks.entry(week).or_insert((0, 0.0));
                cell.0 += 1;
                cell.1 += w * f64::from(set.reps.get());
            }
        }
    }

    let mut out: Vec<MuscleVolume> = acc
        .into_values()
        .map(|(group, weeks)| {
            let weekly_sets: Vec<(NaiveDate, u32)> = weeks.iter().map(|(k, v)| (*k, v.0)).collect();
            let weekly_tonnage: Vec<(NaiveDate, f64)> =
                weeks.iter().map(|(k, v)| (*k, v.1)).collect();
            let n = weeks.len() as f64;
            let mean_weekly_sets = weekly_sets.iter().map(|(_, s)| f64::from(*s)).sum::<f64>() / n;
            let mean_weekly_tonnage = weekly_tonnage.iter().map(|(_, t)| t).sum::<f64>() / n;
            MuscleVolume {
                group,
                weekly_sets,
                weekly_tonnage,
                mean_weekly_sets,
                mean_weekly_tonnage,
            }
        })
        .collect();
    out.sort_by(|a, b| b.mean_weekly_tonnage.total_cmp(&a.mean_weekly_tonnage));
    out
}

#[allow(clippy::cast_possible_truncation)] // distinct-day counts are tiny
fn adherence(sessions: &[&WorkoutSession], target: u32, window_weeks: u32) -> Adherence {
    use std::collections::{BTreeMap, BTreeSet};
    let mut weeks: BTreeMap<NaiveDate, BTreeSet<NaiveDate>> = BTreeMap::new();
    for s in sessions {
        weeks
            .entry(iso_week_monday(s.performed_on))
            .or_default()
            .insert(s.performed_on);
    }
    // `weekly_days` lists only weeks that had sessions (the breakdown), but the
    // ratio is over the WHOLE window — absent weeks count as zero days (AC4), so
    // training one of eight weeks scores low, not 100%.
    let weekly_days: Vec<(NaiveDate, u32)> = weeks
        .iter()
        .map(|(k, days)| (*k, days.len() as u32))
        .collect();
    let ratio = if target == 0 || window_weeks == 0 {
        0.0
    } else {
        let total_days: u32 = weekly_days.iter().map(|(_, d)| *d).sum();
        (f64::from(total_days) / (f64::from(target) * f64::from(window_weeks))).min(1.0)
    };
    Adherence {
        weekly_days,
        target_days_per_week: target,
        ratio,
    }
}

fn body_trend(measurements: &[BodyPoint], today: NaiveDate, start: NaiveDate) -> BodyTrend {
    let mut pts: Vec<&BodyPoint> = measurements
        .iter()
        .filter(|m| m.on > start && m.on <= today)
        .collect();
    pts.sort_by_key(|m| m.on);

    let weight: Vec<TrendPoint> = pts
        .iter()
        .map(|m| TrendPoint {
            on: m.on,
            value: m.weight_kg,
        })
        .collect();
    let body_fat_pct: Vec<TrendPoint> = pts
        .iter()
        .filter_map(|m| {
            m.body_fat_pct.map(|bf| TrendPoint {
                on: m.on,
                value: bf,
            })
        })
        .collect();
    let lean_mass: Vec<TrendPoint> = pts
        .iter()
        .filter_map(|m| {
            m.lean_mass_kg().map(|lm| TrendPoint {
                on: m.on,
                value: lm,
            })
        })
        .collect();

    let opt_slope = |p: &[TrendPoint]| (p.len() >= 2).then(|| weekly_slope(p));

    BodyTrend {
        weight_slope_kg_per_week: weekly_slope(&weight),
        body_fat_slope: opt_slope(&body_fat_pct),
        lean_mass_slope: opt_slope(&lean_mass),
        weight,
        body_fat_pct,
        lean_mass,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::UserId;
    use crate::workout::{ExerciseName, LoadKg, Reps, WorkoutExercise, WorkoutSet};
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }
    fn ts() -> DateTime<Utc> {
        DateTime::from_timestamp(0, 0).unwrap()
    }
    fn wset(reps: i32, kg: Option<f64>) -> WorkoutSet {
        WorkoutSet {
            id: Uuid::nil(),
            position: 0,
            reps: Reps::try_new(reps).unwrap(),
            weight_kg: kg.map(|k| LoadKg::try_new(k).unwrap()),
            rpe: None,
        }
    }
    fn wex(name: &str, g: Option<MuscleGroup>, sets: Vec<WorkoutSet>) -> WorkoutExercise {
        WorkoutExercise {
            id: Uuid::nil(),
            position: 0,
            name: ExerciseName::try_new(name).unwrap(),
            muscle_group: g,
            sets,
        }
    }
    fn sess(on: NaiveDate, ex: Vec<WorkoutExercise>) -> WorkoutSession {
        WorkoutSession {
            id: Uuid::nil(),
            user_id: UserId::new(),
            performed_on: on,
            exercises: ex,
            created_at: ts(),
            updated_at: ts(),
        }
    }
    const TODAY: fn() -> NaiveDate = || d(2026, 7, 9);

    #[test]
    fn deterministic_same_inputs_same_output() {
        let s = vec![sess(
            d(2026, 7, 1),
            vec![wex("Bench", None, vec![wset(5, Some(100.0))])],
        )];
        let m = vec![BodyPoint {
            on: d(2026, 7, 1),
            weight_kg: 80.0,
            body_fat_pct: Some(20.0),
        }];
        assert_eq!(
            summarize(TODAY(), 8, &s, &m, 3),
            summarize(TODAY(), 8, &s, &m, 3)
        );
    }

    #[test]
    fn per_lift_progressive_has_trend_and_positive_slope() {
        let s = vec![
            sess(
                d(2026, 6, 18),
                vec![wex("Bench press", None, vec![wset(5, Some(100.0))])],
            ),
            sess(
                d(2026, 6, 25),
                vec![wex("bench press", None, vec![wset(5, Some(102.5))])],
            ),
            sess(
                d(2026, 7, 2),
                vec![wex("Bench press", None, vec![wset(5, Some(105.0))])],
            ),
        ];
        let out = summarize(TODAY(), 8, &s, &[], 3);
        let lift = &out.lifts[0];
        assert_eq!(lift.name, "Bench press"); // first-seen display, case-insensitive key
        assert_eq!(lift.sessions, 3);
        assert_eq!(lift.e1rm.len(), 3);
        assert!(lift.slope_kg_per_week > 0.0);
        assert!(!lift.stalled);
        assert!((lift.current_e1rm - 105.0 * (1.0 + 5.0 / 30.0)).abs() < 1e-9);
    }

    #[test]
    fn stall_flagged_when_no_new_peak() {
        let s = vec![
            sess(
                d(2026, 6, 11),
                vec![wex("Squat", None, vec![wset(5, Some(140.0))])],
            ),
            sess(
                d(2026, 6, 18),
                vec![wex("Squat", None, vec![wset(5, Some(140.0))])],
            ),
            sess(
                d(2026, 6, 25),
                vec![wex("Squat", None, vec![wset(5, Some(140.0))])],
            ),
            sess(
                d(2026, 7, 2),
                vec![wex("Squat", None, vec![wset(5, Some(140.0))])],
            ),
        ];
        assert!(summarize(TODAY(), 8, &s, &[], 3).lifts[0].stalled);
    }

    #[test]
    fn same_date_sessions_give_zero_slope_not_nan() {
        // Two sessions on the SAME day → two e1RM points with identical x.
        let day = d(2026, 7, 1);
        let s = vec![
            sess(day, vec![wex("Bench", None, vec![wset(5, Some(100.0))])]),
            sess(day, vec![wex("Bench", None, vec![wset(5, Some(120.0))])]),
        ];
        let lift = &summarize(TODAY(), 8, &s, &[], 3).lifts[0];
        assert_eq!(lift.sessions, 2);
        assert!(lift.slope_kg_per_week.is_finite());
        assert!(lift.slope_kg_per_week.abs() < f64::EPSILON);
    }

    #[test]
    fn muscle_volume_sums_and_excludes_untagged() {
        let s = vec![sess(
            d(2026, 7, 7),
            vec![
                wex(
                    "Squat",
                    Some(MuscleGroup::Legs),
                    vec![wset(5, Some(100.0)), wset(5, Some(100.0))],
                ),
                wex("Bench", Some(MuscleGroup::Chest), vec![wset(5, Some(60.0))]),
                wex("Mystery", None, vec![wset(5, Some(50.0))]),
            ],
        )];
        let out = summarize(TODAY(), 8, &s, &[], 3);
        assert_eq!(out.muscle_volume.len(), 2); // untagged excluded
        let legs = &out.muscle_volume[0]; // highest tonnage first
        assert_eq!(legs.group, MuscleGroup::Legs);
        assert_eq!(legs.weekly_sets[0].1, 2);
        assert!((legs.weekly_tonnage[0].1 - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn adherence_ratio_is_over_the_whole_window() {
        // Trained 3 days in ONE week; window is 8 weeks, target 3/week.
        let s = vec![
            sess(
                d(2026, 7, 6),
                vec![wex("A", None, vec![wset(5, Some(50.0))])],
            ),
            sess(
                d(2026, 7, 7),
                vec![wex("A", None, vec![wset(5, Some(50.0))])],
            ),
            sess(
                d(2026, 7, 8),
                vec![wex("A", None, vec![wset(5, Some(50.0))])],
            ),
        ];
        let a = summarize(TODAY(), 8, &s, &[], 3).adherence;
        assert!((a.ratio - 3.0 / (3.0 * 8.0)).abs() < 1e-9); // 0.125, NOT 1.0
    }

    #[test]
    fn body_trend_slopes_track_recomposition() {
        let m = vec![
            BodyPoint {
                on: d(2026, 6, 25),
                weight_kg: 84.0,
                body_fat_pct: Some(24.0),
            },
            BodyPoint {
                on: d(2026, 7, 2),
                weight_kg: 82.0,
                body_fat_pct: Some(20.0),
            },
            BodyPoint {
                on: d(2026, 7, 9),
                weight_kg: 80.0,
                body_fat_pct: Some(16.0),
            },
        ];
        let b = summarize(TODAY(), 8, &[], &m, 3).body;
        assert_eq!(b.weight.len(), 3);
        assert!(b.weight_slope_kg_per_week < 0.0); // losing weight
        assert!(b.body_fat_slope.unwrap() < 0.0); // losing fat
        assert_eq!(b.lean_mass.len(), 3); // derived where bf% present
    }

    #[test]
    fn window_parameter_changes_membership() {
        let s = vec![
            sess(
                d(2026, 4, 30),
                vec![wex("Bench", None, vec![wset(5, Some(100.0))])],
            ), // ~10 wk ago
            sess(
                d(2026, 7, 2),
                vec![wex("Bench", None, vec![wset(5, Some(105.0))])],
            ),
        ];
        assert_eq!(summarize(TODAY(), 4, &s, &[], 3).lifts[0].sessions, 1);
        assert_eq!(summarize(TODAY(), 12, &s, &[], 3).lifts[0].sessions, 2);
    }

    #[test]
    fn empty_inputs_are_well_formed() {
        let out = summarize(TODAY(), 8, &[], &[], 3);
        assert!(out.lifts.is_empty());
        assert!(out.muscle_volume.is_empty());
        assert!(out.adherence.ratio.abs() < f64::EPSILON);
        assert!(out.body.weight.is_empty());
        assert!(out.body.weight_slope_kg_per_week.abs() < f64::EPSILON);
        assert!(out.body.body_fat_slope.is_none());
    }

    #[test]
    fn summary_round_trips_through_json() {
        // Include a same-date lift (the zero-x-variance / potential-NaN case): a
        // NaN would serialize as `null` and fail to deserialize back into f64.
        let day = d(2026, 7, 1);
        let s = vec![
            sess(
                day,
                vec![wex(
                    "Bench",
                    Some(MuscleGroup::Chest),
                    vec![wset(5, Some(100.0))],
                )],
            ),
            sess(
                day,
                vec![wex(
                    "Bench",
                    Some(MuscleGroup::Chest),
                    vec![wset(5, Some(120.0))],
                )],
            ),
        ];
        let out = summarize(TODAY(), 8, &s, &[], 3);
        let json = serde_json::to_string(&out).unwrap();
        // A NaN slope would serialize as `null` and fail to deserialize into the
        // required (non-Option) f64 fields — so a successful parse proves no NaN.
        let back: TrainingSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back.lifts.len(), out.lifts.len());
        assert!((back.lifts[0].current_e1rm - out.lifts[0].current_e1rm).abs() < 1e-6);
        assert!(back.lifts[0].slope_kg_per_week.abs() < f64::EPSILON);
    }
}
