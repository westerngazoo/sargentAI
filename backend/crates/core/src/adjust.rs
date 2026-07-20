//! R-0017 / SPEC-0017 — heuristic program-adjustment engine.
//!
//! Deterministic rules over the R-0015 [`TrainingSummary`] that emit typed
//! **suggestions** — never mutations. The [`Adjustment`] wire shape is designed
//! to outlive the heuristics: the learned model (R-0016) later slots behind the
//! same interface. No I/O, no clock, no archetype data, no medical claims.

use serde::{Deserialize, Serialize};

use crate::aggregate::{TrainingSummary, TrendPoint};
use crate::archetype::VolumeBand;
use crate::program::{DietIntent, GeneratedDiet, GeneratedProgram};
use crate::workout::MuscleGroup;

// Rule thresholds (SPEC-0017 §2.3) — named so QA tunes them in one place.
/// A lift needs this many in-window sessions before rules 1–2 speak.
const MIN_SESSIONS: u32 = 4;
/// Deload size suggested for a stalled lift.
const DELOAD_PCT: f64 = 0.10;
/// e1RM slope (kg/week) at or above which a lift earns a progress nudge.
const PROGRESS_SLOPE: f64 = 0.5;
/// A lift's points must span at least this many days for its slope to count —
/// four sessions inside one week measure noise, not a trend (architect review).
const MIN_LIFT_SPAN_DAYS: i64 = 14;
/// The next-load increment suggested for a progressing lift.
const INCREMENT_KG: f64 = 2.5;
/// Mean weekly working sets below this reads as a volume gap — conditioned on
/// the program's own volume band so low-volume HIT programs aren't nagged to
/// add sets their philosophy forbids (architect review).
const LOW_SETS_MODERATE: f64 = 6.0;
/// The volume-gap floor for high-volume programs.
const LOW_SETS_HIGH: f64 = 8.0;
/// Adherence at or above this qualifies the user for more volume.
const ADHERENT: f64 = 0.75;
/// Adherence below this triggers right-sizing.
const LOW_ADHERENCE: f64 = 0.6;
/// Weeks with logged sessions required before judging adherence or volume.
const MIN_WEEKS: usize = 3;
/// Weight drift (kg/week) against the diet's intent that triggers a kcal nudge.
const WEIGHT_DRIFT: f64 = 0.25;
/// Weight points must span at least this many days — daily fluctuation is
/// ±1–2 kg, so a few close-together points measure water, not trend.
const MIN_WEIGHT_SPAN_DAYS: i64 = 21;
/// Size of the suggested kcal correction, percent.
const KCAL_STEP_PCT: i32 = 10;
/// Body measurements required before the diet rule speaks.
const MIN_BODY_POINTS: usize = 3;
/// Rules 1–2 only look at the first N lifts (summary is sorted by sessions).
const TOP_LIFTS: usize = 5;
/// Hard cap on suggestions per run, highest severity first.
const MAX_SUGGESTIONS: usize = 4;

/// Days between the first and last point of a series (0 for < 2 points).
fn span_days(points: &[TrendPoint]) -> i64 {
    match (points.first(), points.last()) {
        (Some(a), Some(b)) => (b.on - a.on).num_days(),
        _ => 0,
    }
}

/// What a suggestion proposes, machine-readably. A closed vocabulary — the
/// change is data, never prose.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Change {
    DeloadLift { lift: String, pct: f64 },
    ProgressLift { lift: String, add_kg: f64 },
    AddWeeklySets { group: MuscleGroup, sets: u32 },
    ReduceDaysPerWeek { from: u8, to: u8 },
    AdjustKcal { delta_pct: i32 },
}

/// How urgent a suggestion is. `Action` sorts before `Info`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Action,
    Info,
}

/// One suggestion: a machine-readable change plus the human "why".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Adjustment {
    pub change: Change,
    pub severity: Severity,
    pub rationale: String,
}

/// Run the rule set. Pure and deterministic; an empty result means "on track".
#[must_use]
pub fn suggest(
    summary: &TrainingSummary,
    program: &GeneratedProgram,
    diet: &GeneratedDiet,
) -> Vec<Adjustment> {
    let mut out = Vec::new();
    out.extend(lift_rules(summary));
    out.extend(volume_rule(summary, program));
    out.extend(adherence_rule(summary, program));
    out.extend(diet_rule(summary, diet));
    out.sort_by_key(|a| a.severity);
    out.truncate(MAX_SUGGESTIONS);
    out
}

/// Rules 1–2: stalled lifts get a deload; steadily-gaining lifts get the next
/// increment. `stalled` excludes rule 2, so a lift emits at most one.
fn lift_rules(summary: &TrainingSummary) -> Vec<Adjustment> {
    let mut out = Vec::new();
    for lift in summary.lifts.iter().take(TOP_LIFTS) {
        if lift.sessions < MIN_SESSIONS {
            continue;
        }
        if lift.stalled {
            out.push(Adjustment {
                change: Change::DeloadLift {
                    lift: lift.name.clone(),
                    pct: DELOAD_PCT,
                },
                severity: Severity::Action,
                rationale: format!(
                    "{} hasn't set a new estimated-1RM peak in its last sessions \
                     ({} logged this window). A one-session ~{:.0}% deload often \
                     restarts progress.",
                    lift.name,
                    lift.sessions,
                    DELOAD_PCT * 100.0
                ),
            });
        } else if lift.slope_kg_per_week >= PROGRESS_SLOPE
            && span_days(&lift.e1rm) >= MIN_LIFT_SPAN_DAYS
        {
            out.push(Adjustment {
                change: Change::ProgressLift {
                    lift: lift.name.clone(),
                    add_kg: INCREMENT_KG,
                },
                severity: Severity::Info,
                rationale: format!(
                    "{} is trending up about {:.1} kg/week over {} sessions — \
                     you've earned the next {:.1} kg.",
                    lift.name, lift.slope_kg_per_week, lift.sessions, INCREMENT_KG
                ),
            });
        }
    }
    out
}

/// Rule 3: an adherent user with a low-volume muscle group gets +1 weekly set.
/// The floor follows the PROGRAM's volume band — a low-volume HIT program is
/// intentionally sparse, so the rule stays silent rather than contradicting it.
fn volume_rule(summary: &TrainingSummary, program: &GeneratedProgram) -> Vec<Adjustment> {
    let floor = match program.volume {
        VolumeBand::Low => return Vec::new(),
        VolumeBand::Moderate => LOW_SETS_MODERATE,
        VolumeBand::High => LOW_SETS_HIGH,
    };
    if summary.adherence.ratio < ADHERENT {
        return Vec::new();
    }
    summary
        .muscle_volume
        .iter()
        .filter(|v| v.mean_weekly_sets < floor && v.weekly_sets.len() >= MIN_WEEKS)
        .map(|v| Adjustment {
            change: Change::AddWeeklySets {
                group: v.group,
                sets: 1,
            },
            severity: Severity::Info,
            rationale: format!(
                "{} is averaging {:.1} working sets/week — below the {floor:.0} \
                 your program's volume aims for. You're consistent, so add one set.",
                v.group.as_str(),
                v.mean_weekly_sets
            ),
        })
        .collect()
}

/// Rule 4: chronic under-adherence → suggest right-sizing `days_per_week`
/// toward what actually happens (never below 2).
fn adherence_rule(summary: &TrainingSummary, program: &GeneratedProgram) -> Vec<Adjustment> {
    let a = &summary.adherence;
    if a.ratio >= LOW_ADHERENCE
        || program.days_per_week < 3
        || a.weekly_days.len() < MIN_WEEKS
        || a.target_days_per_week == 0
    {
        return Vec::new();
    }
    // Observed frequency over the WHOLE window (ratio × target = total days /
    // window weeks). `weekly_days` alone lists only active weeks and would
    // OVERSTATE frequency for someone who trains hard but rarely — e.g. 6 days
    // in one of eight weeks must not read as "6/week" (architect review).
    // If the result wouldn't actually reduce the plan, stay silent.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let observed = (a.ratio * f64::from(a.target_days_per_week)).round() as u8;
    let to = observed.max(2);
    if to >= program.days_per_week {
        return Vec::new();
    }
    vec![Adjustment {
        change: Change::ReduceDaysPerWeek {
            from: program.days_per_week,
            to,
        },
        severity: Severity::Action,
        rationale: format!(
            "You're hitting about {:.0}% of your {}-day plan. A {to}-day split \
             you complete beats a {}-day split you don't.",
            a.ratio * 100.0,
            program.days_per_week,
            program.days_per_week
        ),
    }]
}

/// Rule 5: the scale moving against the diet's TYPED intent → a corrective
/// kcal nudge. Intent comes from `GeneratedDiet.intent` (derived from the same
/// goal branch as the kcal math) — never parsed from the strategy prose, whose
/// "surplus" wording survives even on a fat-loss plan (architect review). Older
/// stored diets deserialize as `Maintain` and stay silent.
fn diet_rule(summary: &TrainingSummary, diet: &GeneratedDiet) -> Vec<Adjustment> {
    if summary.body.weight.len() < MIN_BODY_POINTS
        || span_days(&summary.body.weight) < MIN_WEIGHT_SPAN_DAYS
    {
        return Vec::new();
    }
    let slope = summary.body.weight_slope_kg_per_week;
    let (delta_pct, direction, aim) = match diet.intent {
        // Meant to gain but losing faster than the drift band → eat more.
        DietIntent::Surplus if slope <= -WEIGHT_DRIFT => (KCAL_STEP_PCT, "losing", "gaining"),
        // Meant to lose but gaining faster than the drift band → eat less.
        DietIntent::Deficit if slope >= WEIGHT_DRIFT => (-KCAL_STEP_PCT, "gaining", "losing"),
        _ => return Vec::new(),
    };
    vec![Adjustment {
        change: Change::AdjustKcal { delta_pct },
        severity: Severity::Action,
        rationale: format!(
            "Your plan aims at {aim} weight, but the scale shows you {direction} \
             about {:.2} kg/week. Adjusting intake ~{}% should re-align it.",
            slope.abs(),
            delta_pct.abs()
        ),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate::{Adherence, BodyTrend, LiftSummary, MuscleVolume, TrendPoint};
    use crate::archetype::{MacroEmphasis, VolumeBand};
    use chrono::NaiveDate;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 7, day).unwrap()
    }

    /// Weekly-spaced sessions: 4 sessions span 21 days (≥ `MIN_LIFT_SPAN_DAYS`).
    fn lift(name: &str, sessions: u32, slope: f64, stalled: bool) -> LiftSummary {
        LiftSummary {
            name: name.into(),
            e1rm: (0..sessions)
                .map(|i| TrendPoint {
                    on: d(1 + i * 7),
                    value: 100.0,
                })
                .collect(),
            current_e1rm: 100.0,
            slope_kg_per_week: slope,
            sessions,
            stalled,
        }
    }

    /// The same session count crammed into consecutive days — too short a span
    /// for a slope to mean anything.
    fn clustered_lift(name: &str, sessions: u32, slope: f64) -> LiftSummary {
        LiftSummary {
            e1rm: (0..sessions)
                .map(|i| TrendPoint {
                    on: d(1 + i),
                    value: 100.0,
                })
                .collect(),
            ..lift(name, sessions, slope, false)
        }
    }

    fn base_summary() -> TrainingSummary {
        TrainingSummary {
            window_weeks: 8,
            generated_for: d(9),
            lifts: Vec::new(),
            muscle_volume: Vec::new(),
            adherence: Adherence {
                weekly_days: vec![(d(1), 3), (d(8), 3), (d(15), 3), (d(22), 3)],
                target_days_per_week: 3,
                ratio: 0.9,
            },
            body: BodyTrend {
                weight: Vec::new(),
                weight_slope_kg_per_week: 0.0,
                body_fat_pct: Vec::new(),
                body_fat_slope: None,
                lean_mass: Vec::new(),
                lean_mass_slope: None,
            },
        }
    }

    fn program(days: u8) -> GeneratedProgram {
        GeneratedProgram {
            split: "test split".into(),
            days_per_week: days,
            weekly_frequency_per_muscle: 2,
            volume: VolumeBand::Moderate,
            intensity_guidance: String::new(),
            rest_guidance: String::new(),
            progression_guidance: String::new(),
            estimated_session_duration_min: 60,
            highlight_exercises: Vec::new(),
        }
    }

    fn diet(intent: DietIntent) -> GeneratedDiet {
        GeneratedDiet {
            approach: "test".into(),
            // Deliberately misleading prose: intent must come from the typed
            // field, never from this string.
            calorie_strategy: "moderate surplus".into(),
            intent,
            macro_emphasis: MacroEmphasis::HighProtein,
            meal_structure: String::new(),
            estimated_kcal: 3000,
            protein_g: 180,
            carbs_g: 300,
            fat_g: 90,
        }
    }

    #[test]
    fn stalled_lift_with_enough_sessions_gets_deload() {
        let mut s = base_summary();
        s.lifts = vec![lift("Bench press", MIN_SESSIONS, 0.0, true)];
        let out = suggest(&s, &program(4), &diet(DietIntent::Surplus));
        assert!(matches!(
            &out[0].change,
            Change::DeloadLift { lift, pct } if lift == "Bench press" && (*pct - DELOAD_PCT).abs() < 1e-9
        ));
        assert_eq!(out[0].severity, Severity::Action);
        assert!(out[0].rationale.contains("Bench press"));
    }

    #[test]
    fn stalled_lift_below_min_sessions_stays_silent() {
        let mut s = base_summary();
        s.lifts = vec![lift("Bench", MIN_SESSIONS - 1, 0.0, true)];
        assert!(suggest(&s, &program(4), &diet(DietIntent::Surplus)).is_empty());
    }

    #[test]
    fn progressing_lift_gets_increment_but_stalled_excludes_it() {
        let mut s = base_summary();
        s.lifts = vec![lift("Squat", MIN_SESSIONS, PROGRESS_SLOPE, false)];
        let out = suggest(&s, &program(4), &diet(DietIntent::Surplus));
        assert!(matches!(&out[0].change, Change::ProgressLift { lift, .. } if lift == "Squat"));
        // Boundary: slope just below threshold → silent.
        s.lifts = vec![lift("Squat", MIN_SESSIONS, PROGRESS_SLOPE - 0.01, false)];
        assert!(suggest(&s, &program(4), &diet(DietIntent::Surplus)).is_empty());
    }

    #[test]
    fn volume_gap_fires_only_when_adherent() {
        // Present in MIN_WEEKS distinct weeks — a real pattern, not one stray
        // session (architect review).
        let vol = MuscleVolume {
            group: MuscleGroup::Legs,
            weekly_sets: vec![(d(1), 3), (d(8), 3), (d(15), 3)],
            weekly_tonnage: vec![(d(1), 900.0), (d(8), 900.0), (d(15), 900.0)],
            mean_weekly_sets: 3.0,
            mean_weekly_tonnage: 900.0,
        };
        let mut s = base_summary();
        s.muscle_volume = vec![vol.clone()];
        let out = suggest(&s, &program(4), &diet(DietIntent::Surplus));
        assert!(
            matches!(&out[0].change, Change::AddWeeklySets { group, sets: 1 } if *group == MuscleGroup::Legs)
        );
        // Same gap but poor adherence → silent (fix the plan first).
        s.adherence.ratio = ADHERENT - 0.01;
        s.muscle_volume = vec![vol];
        let out = suggest(&s, &program(4), &diet(DietIntent::Surplus));
        assert!(!out
            .iter()
            .any(|a| matches!(a.change, Change::AddWeeklySets { .. })));
    }

    #[test]
    fn volume_gap_respects_low_volume_programs_and_sparse_groups() {
        let vol = MuscleVolume {
            group: MuscleGroup::Legs,
            weekly_sets: vec![(d(1), 2), (d(8), 2), (d(15), 2)],
            weekly_tonnage: vec![(d(1), 600.0), (d(8), 600.0), (d(15), 600.0)],
            mean_weekly_sets: 2.0,
            mean_weekly_tonnage: 600.0,
        };
        let mut s = base_summary();
        s.muscle_volume = vec![vol];
        // A low-volume HIT program is SUPPOSED to be sparse → silent, never
        // contradict the program the engine was handed (architect review).
        let mut hit = program(4);
        hit.volume = VolumeBand::Low;
        assert!(suggest(&s, &hit, &diet(DietIntent::Surplus)).is_empty());
        // A group seen in fewer than MIN_WEEKS weeks (one stray curl session)
        // is not a pattern → silent.
        s.muscle_volume = vec![MuscleVolume {
            group: MuscleGroup::Arms,
            weekly_sets: vec![(d(1), 3)],
            weekly_tonnage: vec![(d(1), 300.0)],
            mean_weekly_sets: 3.0,
            mean_weekly_tonnage: 300.0,
        }];
        assert!(suggest(&s, &program(4), &diet(DietIntent::Surplus)).is_empty());
    }

    #[test]
    fn clustered_sessions_do_not_earn_a_progress_nudge() {
        // Steep slope but all sessions within one week → span guard silences
        // rule 2 (four points in five days measure noise, not a trend).
        let mut s = base_summary();
        s.lifts = vec![clustered_lift("Bench", MIN_SESSIONS, 5.0)];
        assert!(suggest(&s, &program(4), &diet(DietIntent::Surplus)).is_empty());
    }

    #[test]
    fn gaining_on_a_deficit_gets_kcal_reduction() {
        // Sign convention: Deficit + gaining beyond the drift band → −KCAL_STEP.
        let mut s = base_summary();
        s.body.weight = (0..4)
            .map(|i| TrendPoint {
                on: d(1 + i * 7),
                value: 80.0 + f64::from(i),
            })
            .collect();
        s.body.weight_slope_kg_per_week = 1.0;
        let out = suggest(&s, &program(4), &diet(DietIntent::Deficit));
        assert!(
            matches!(out[0].change, Change::AdjustKcal { delta_pct } if delta_pct == -KCAL_STEP_PCT)
        );
        // Same data under a Surplus plan → gaining is the goal → silent.
        assert!(suggest(&s, &program(4), &diet(DietIntent::Surplus)).is_empty());
    }

    #[test]
    fn short_span_weight_data_stays_silent() {
        // Three points across five days: enough COUNT, not enough SPAN — daily
        // fluctuation would masquerade as a trend (architect review).
        let mut s = base_summary();
        s.body.weight = (0..3)
            .map(|i| TrendPoint {
                on: d(1 + i * 2),
                value: 84.0 - f64::from(i),
            })
            .collect();
        s.body.weight_slope_kg_per_week = -2.0;
        assert!(suggest(&s, &program(4), &diet(DietIntent::Surplus)).is_empty());
    }

    #[test]
    fn under_adherence_suggests_right_sizing_with_floor() {
        let mut s = base_summary();
        s.adherence = Adherence {
            weekly_days: vec![(d(1), 2), (d(8), 2), (d(15), 2)],
            target_days_per_week: 5,
            ratio: 0.4,
        };
        let out = suggest(&s, &program(5), &diet(DietIntent::Surplus));
        assert!(matches!(
            out[0].change,
            Change::ReduceDaysPerWeek { from: 5, to: 2 }
        ));
        // Fewer than MIN_WEEKS of data → silent.
        s.adherence.weekly_days.truncate(MIN_WEEKS - 1);
        assert!(suggest(&s, &program(5), &diet(DietIntent::Surplus)).is_empty());
    }

    #[test]
    fn losing_weight_on_a_surplus_gets_kcal_bump() {
        let mut s = base_summary();
        // Four weekly points → span 21 days, satisfying MIN_WEIGHT_SPAN_DAYS.
        s.body.weight = (0..4)
            .map(|i| TrendPoint {
                on: d(1 + i * 7),
                value: 84.0 - f64::from(i),
            })
            .collect();
        s.body.weight_slope_kg_per_week = -1.0; // well past the drift band
        let out = suggest(&s, &program(4), &diet(DietIntent::Surplus));
        assert!(
            matches!(out[0].change, Change::AdjustKcal { delta_pct } if delta_pct == KCAL_STEP_PCT)
        );
        // Maintain intent (incl. pre-intent stored diets) stays silent.
        let out = suggest(&s, &program(4), &diet(DietIntent::Maintain));
        assert!(!out
            .iter()
            .any(|a| matches!(a.change, Change::AdjustKcal { .. })));
        // Too few measurements → silent.
        s.body.weight.truncate(MIN_BODY_POINTS - 1);
        let out = suggest(&s, &program(4), &diet(DietIntent::Surplus));
        assert!(!out
            .iter()
            .any(|a| matches!(a.change, Change::AdjustKcal { .. })));
    }

    #[test]
    fn output_is_bounded_and_action_sorts_first() {
        let mut s = base_summary();
        // 5 stalled lifts + a volume gap → more candidates than MAX_SUGGESTIONS.
        s.lifts = (0..5)
            .map(|i| lift(&format!("Lift {i}"), MIN_SESSIONS, 0.0, true))
            .collect();
        s.muscle_volume = vec![MuscleVolume {
            group: MuscleGroup::Arms,
            weekly_sets: vec![(d(1), 2)],
            weekly_tonnage: vec![(d(1), 200.0)],
            mean_weekly_sets: 2.0,
            mean_weekly_tonnage: 200.0,
        }];
        let out = suggest(&s, &program(4), &diet(DietIntent::Surplus));
        assert_eq!(out.len(), MAX_SUGGESTIONS);
        assert!(out.iter().all(|a| a.severity == Severity::Action)); // Info truncated
    }

    #[test]
    fn empty_summary_is_on_track() {
        assert!(suggest(&base_summary(), &program(4), &diet(DietIntent::Surplus)).is_empty());
    }

    #[test]
    fn deterministic_and_serde_round_trips() {
        let mut s = base_summary();
        s.lifts = vec![lift("Bench", MIN_SESSIONS, 0.0, true)];
        let a = suggest(&s, &program(4), &diet(DietIntent::Surplus));
        let b = suggest(&s, &program(4), &diet(DietIntent::Surplus));
        assert_eq!(a, b);
        let json = serde_json::to_string(&a).unwrap();
        let back: Vec<Adjustment> = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
        assert!(json.contains("\"kind\":\"deload_lift\"")); // tagged wire shape
    }
}
