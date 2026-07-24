//! R-0039 / SPEC-0039 — program authoring model.
//!
//! Lets a trainer (or self-user) author a program: exercises with Low/Med/High
//! intensity classes (each a top-set + back-off list of work-set lines), split
//! into core lifts and accessories, placed on relative day indices. [`materialize`]
//! turns it + the user's e1RM into concrete prescribed sessions, reusing R-0038's
//! [`PrescribedSet`](crate::periodize::PrescribedSet) and load math. Pure: no I/O,
//! no clock, no ML. Calendars are client-relative (day *indices*, not weekdays).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::periodize::{lift_key, load, PrescribedSet};

// ---------------------------------------------------------------------------
// Authored model (AC1–AC4)
// ---------------------------------------------------------------------------

/// The intensity class a scheduled lift is trained at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntensityClass {
    Low,
    Medium,
    High,
}

/// One work-set line: `sets` × `reps` at `load_pct` of e1RM. A class can hold
/// several (a top set plus back-offs).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkSetLine {
    pub sets: u32,
    pub reps: u32,
    pub load_pct: f64,
}

/// A class prescription: a warm-up count plus one or more work-set lines.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClassPrescription {
    pub warmup_sets: u32,
    pub work: Vec<WorkSetLine>,
}

/// An authored exercise with a prescription for each intensity class.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuthoredExercise {
    pub name: String,
    pub low: ClassPrescription,
    pub medium: ClassPrescription,
    pub high: ClassPrescription,
}

impl AuthoredExercise {
    fn prescription(&self, class: IntensityClass) -> &ClassPrescription {
        match class {
            IntensityClass::Low => &self.low,
            IntensityClass::Medium => &self.medium,
            IntensityClass::High => &self.high,
        }
    }

    fn classes(&self) -> [&ClassPrescription; 3] {
        [&self.low, &self.medium, &self.high]
    }
}

/// A `(exercise, class)` placed on a day.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub exercise: String,
    pub class: IntensityClass,
}

/// A relative-day schedule. `days[i]` is day index `i + 1`; an empty inner vec
/// is a rest day. The cycle length is `days.len()`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Schedule {
    pub days: Vec<Vec<ScheduleEntry>>,
}

/// A complete authored program.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuthoredProgram {
    pub name: String,
    pub core: Vec<AuthoredExercise>,
    pub accessories: Vec<AuthoredExercise>,
    pub schedule: Schedule,
}

// ---------------------------------------------------------------------------
// Materialized output (AC5)
// ---------------------------------------------------------------------------

/// One materialized `(lift, class)` for a day: the warm-up count and the
/// expanded work sets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterializedEntry {
    pub lift: String,
    pub class: IntensityClass,
    pub warmup_sets: u32,
    pub work_sets: Vec<PrescribedSet>,
}

/// One day of a materialized cycle (empty `entries` = rest day).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterializedDay {
    pub day_index: u32,
    pub entries: Vec<MaterializedEntry>,
}

/// A concrete, ready-to-render cycle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterializedCycle {
    pub cycle_days: u32,
    pub days: Vec<MaterializedDay>,
}

// ---------------------------------------------------------------------------
// Errors (AC6)
// ---------------------------------------------------------------------------

/// Why an authored program could not be materialized (AC6 — never a panic).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AuthorError {
    #[error("a program needs at least one exercise")]
    NoExercises,
    #[error("an exercise name is blank")]
    BlankExercise,
    #[error("two exercises share a name (case-insensitive): {0}")]
    DuplicateExercise(String),
    #[error("the schedule has no days")]
    NoScheduleDays,
    #[error("the schedule has no entries on any day")]
    NoScheduledEntries,
    #[error("schedule references an unknown exercise: {0}")]
    UnknownExercise(String),
    #[error("a class prescription has no work-set lines")]
    EmptyWorkLines,
    #[error("load_pct must be in (0, 1]")]
    BadIntensity,
    #[error("reps must be >= 1")]
    BadReps,
    #[error("sets must be >= 1")]
    BadSets,
    #[error("plate_kg must be finite and > 0")]
    BadPlate,
}

// ---------------------------------------------------------------------------
// Materialization
// ---------------------------------------------------------------------------

/// Turn an authored program + per-lift e1RM into a concrete cycle. Pure and
/// deterministic. `plate_kg` rounds computed loads; a lift with no e1RM gets a
/// `None` load (reps × % still prescribed).
///
/// # Errors
/// [`AuthorError`] on any invalid input (validated up front — no panic).
pub fn materialize(
    program: &AuthoredProgram,
    e1rm: &E1rmMap,
    plate_kg: f64,
) -> Result<MaterializedCycle, AuthorError> {
    // key(lower/trim) -> exercise. A case-insensitive duplicate name is a typed
    // error (rather than silently shadowing one lift with another).
    let mut by_key: BTreeMap<String, &AuthoredExercise> = BTreeMap::new();
    for ex in program.core.iter().chain(&program.accessories) {
        if ex.name.trim().is_empty() {
            return Err(AuthorError::BlankExercise);
        }
        if by_key.insert(lift_key(&ex.name), ex).is_some() {
            return Err(AuthorError::DuplicateExercise(ex.name.clone()));
        }
    }
    validate(program, &by_key, plate_kg)?;

    // The lookup is fallible by construction (`get().ok_or`), so materialization
    // cannot panic on a stray reference even independently of `validate`.
    let days = program
        .schedule
        .days
        .iter()
        .enumerate()
        .map(|(i, entries)| -> Result<MaterializedDay, AuthorError> {
            let materialized = entries
                .iter()
                .map(|se| -> Result<MaterializedEntry, AuthorError> {
                    let ex = by_key
                        .get(&lift_key(&se.exercise))
                        .ok_or_else(|| AuthorError::UnknownExercise(se.exercise.clone()))?;
                    let p = ex.prescription(se.class);
                    let e = e1rm.get(&lift_key(&ex.name)).copied();
                    let work_sets = p
                        .work
                        .iter()
                        .flat_map(|l| {
                            let set = PrescribedSet {
                                reps: l.reps,
                                intensity_pct: l.load_pct,
                                target_load_kg: load(e, l.load_pct, plate_kg),
                            };
                            std::iter::repeat_n(set, l.sets as usize)
                        })
                        .collect();
                    Ok(MaterializedEntry {
                        lift: ex.name.clone(),
                        class: se.class,
                        warmup_sets: p.warmup_sets,
                        work_sets,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(MaterializedDay {
                day_index: u32::try_from(i + 1).unwrap_or(u32::MAX),
                entries: materialized,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(MaterializedCycle {
        cycle_days: u32::try_from(program.schedule.days.len()).unwrap_or(u32::MAX),
        days,
    })
}

/// Per-lift current estimated 1RM, keyed by [`lift_key`].
pub type E1rmMap = BTreeMap<String, f64>;

fn validate(
    program: &AuthoredProgram,
    by_key: &BTreeMap<String, &AuthoredExercise>,
    plate_kg: f64,
) -> Result<(), AuthorError> {
    if !(plate_kg.is_finite() && plate_kg > 0.0) {
        return Err(AuthorError::BadPlate);
    }
    if by_key.is_empty() {
        return Err(AuthorError::NoExercises);
    }
    if program.schedule.days.is_empty() {
        return Err(AuthorError::NoScheduleDays);
    }
    if program.schedule.days.iter().all(Vec::is_empty) {
        return Err(AuthorError::NoScheduledEntries);
    }

    // Every scheduled exercise must resolve.
    for day in &program.schedule.days {
        for se in day {
            if !by_key.contains_key(&lift_key(&se.exercise)) {
                return Err(AuthorError::UnknownExercise(se.exercise.clone()));
            }
        }
    }

    // Validate all three class prescriptions of every exercise (surface an
    // authoring mistake even on a class not yet scheduled).
    for ex in program.core.iter().chain(&program.accessories) {
        for class in ex.classes() {
            if class.work.is_empty() {
                return Err(AuthorError::EmptyWorkLines);
            }
            for line in &class.work {
                if !(line.load_pct > 0.0 && line.load_pct <= 1.0) {
                    return Err(AuthorError::BadIntensity);
                }
                if line.reps < 1 {
                    return Err(AuthorError::BadReps);
                }
                if line.sets < 1 {
                    return Err(AuthorError::BadSets);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(sets: u32, reps: u32, pct: f64) -> WorkSetLine {
        WorkSetLine {
            sets,
            reps,
            load_pct: pct,
        }
    }

    /// An exercise whose three classes are simple single lines (L/M/H).
    fn ex(name: &str) -> AuthoredExercise {
        AuthoredExercise {
            name: name.into(),
            low: ClassPrescription {
                warmup_sets: 2,
                work: vec![line(3, 8, 0.70)],
            },
            medium: ClassPrescription {
                warmup_sets: 2,
                work: vec![line(4, 5, 0.80)],
            },
            high: ClassPrescription {
                warmup_sets: 3,
                work: vec![line(5, 3, 0.88)],
            },
        }
    }

    fn e1rm() -> E1rmMap {
        E1rmMap::from([("squat".into(), 200.0), ("bench press".into(), 100.0)])
    }

    /// Squat L/M/H on days 1/3/6 of a 7-day cycle; bench Medium on day 1.
    fn program() -> AuthoredProgram {
        let mut days = vec![Vec::new(); 7];
        days[0] = vec![
            ScheduleEntry {
                exercise: "Squat".into(),
                class: IntensityClass::Low,
            },
            ScheduleEntry {
                exercise: "Bench Press".into(),
                class: IntensityClass::Medium,
            },
        ];
        days[2] = vec![ScheduleEntry {
            exercise: "squat".into(),
            class: IntensityClass::Medium,
        }];
        days[5] = vec![ScheduleEntry {
            exercise: "SQUAT".into(),
            class: IntensityClass::High,
        }];
        AuthoredProgram {
            name: "My Program".into(),
            core: vec![ex("Squat"), ex("Bench Press"), ex("Deadlift")],
            accessories: vec![ex("Curl"), ex("Face Pull")],
            schedule: Schedule { days },
        }
    }

    #[test]
    fn materializes_shape_and_client_relative_days() {
        let c = materialize(&program(), &e1rm(), 2.5).unwrap();
        assert_eq!(c.cycle_days, 7);
        assert_eq!(c.days.len(), 7);
        assert_eq!(c.days[0].day_index, 1);
        assert_eq!(c.days[0].entries.len(), 2); // squat + bench
        assert!(c.days[1].entries.is_empty()); // rest day
        assert_eq!(c.days[6].day_index, 7);
    }

    #[test]
    fn squat_appears_at_three_classes_across_the_cycle() {
        let c = materialize(&program(), &e1rm(), 2.5).unwrap();
        assert_eq!(c.days[0].entries[0].class, IntensityClass::Low);
        assert_eq!(c.days[2].entries[0].class, IntensityClass::Medium);
        assert_eq!(c.days[5].entries[0].class, IntensityClass::High);
    }

    #[test]
    fn loads_are_pct_of_e1rm_plate_rounded_with_warmups() {
        let c = materialize(&program(), &e1rm(), 2.5).unwrap();
        // Day 6 squat High = 5 sets × 3 reps @ 88% of 200 = 176 → plate 175.0.
        let high = &c.days[5].entries[0];
        assert_eq!(high.warmup_sets, 3);
        assert_eq!(high.work_sets.len(), 5); // expanded from sets:5
        let set = &high.work_sets[0];
        assert_eq!(set.reps, 3);
        assert!((set.intensity_pct - 0.88).abs() < 1e-9);
        assert_eq!(set.target_load_kg, Some(175.0));
    }

    #[test]
    fn top_set_plus_backoffs_expand_in_order() {
        let mut p = program();
        // Squat High: 1×3 @90% (top) then 3×5 @80% (back-offs) = 4 sets.
        p.core[0].high = ClassPrescription {
            warmup_sets: 3,
            work: vec![line(1, 3, 0.90), line(3, 5, 0.80)],
        };
        let c = materialize(&p, &e1rm(), 2.5).unwrap();
        let sets = &c.days[5].entries[0].work_sets;
        assert_eq!(sets.len(), 4);
        assert_eq!((sets[0].reps, sets[0].target_load_kg), (3, Some(180.0))); // 90% × 200
        assert_eq!((sets[3].reps, sets[3].target_load_kg), (5, Some(160.0))); // 80% × 200
    }

    #[test]
    fn missing_e1rm_prescribes_reps_and_pct_without_load() {
        // Deadlift is authored but has no e1RM in the map.
        let mut p = program();
        p.schedule.days[3] = vec![ScheduleEntry {
            exercise: "Deadlift".into(),
            class: IntensityClass::Low,
        }];
        let c = materialize(&p, &e1rm(), 2.5).unwrap();
        let dl = &c.days[3].entries[0].work_sets[0];
        assert_eq!(dl.target_load_kg, None);
        assert_eq!(dl.reps, 8);
        assert!(dl.intensity_pct > 0.0);
    }

    #[test]
    fn validation_errors_are_typed_never_panic() {
        assert_eq!(
            materialize(&program(), &e1rm(), 0.0),
            Err(AuthorError::BadPlate)
        );

        let mut no_ex = program();
        no_ex.core.clear();
        no_ex.accessories.clear();
        assert_eq!(
            materialize(&no_ex, &e1rm(), 2.5),
            Err(AuthorError::NoExercises)
        );

        let mut blank = program();
        blank.accessories.push(ex("   "));
        assert_eq!(
            materialize(&blank, &e1rm(), 2.5),
            Err(AuthorError::BlankExercise)
        );

        let mut unknown = program();
        unknown.schedule.days[0] = vec![ScheduleEntry {
            exercise: "Ghost Lift".into(),
            class: IntensityClass::Low,
        }];
        assert_eq!(
            materialize(&unknown, &e1rm(), 2.5),
            Err(AuthorError::UnknownExercise("Ghost Lift".into()))
        );

        let mut empty_sched = program();
        empty_sched.schedule.days = vec![Vec::new(); 3];
        assert_eq!(
            materialize(&empty_sched, &e1rm(), 2.5),
            Err(AuthorError::NoScheduledEntries)
        );

        // Case-insensitive duplicate exercise names are rejected.
        let mut dup = program();
        dup.accessories.push(ex("SQUAT")); // core already has "Squat"
        assert_eq!(
            materialize(&dup, &e1rm(), 2.5),
            Err(AuthorError::DuplicateExercise("SQUAT".into()))
        );

        let mut bad_pct = program();
        bad_pct.core[0].low.work = vec![line(3, 5, 1.5)];
        assert_eq!(
            materialize(&bad_pct, &e1rm(), 2.5),
            Err(AuthorError::BadIntensity)
        );

        let mut no_lines = program();
        no_lines.core[0].low.work.clear();
        assert_eq!(
            materialize(&no_lines, &e1rm(), 2.5),
            Err(AuthorError::EmptyWorkLines)
        );
    }

    #[test]
    fn deterministic_and_serde_round_trips() {
        let a = materialize(&program(), &e1rm(), 2.5).unwrap();
        let b = materialize(&program(), &e1rm(), 2.5).unwrap();
        assert_eq!(a, b);
        let json = serde_json::to_string(&a).unwrap();
        let back: MaterializedCycle = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
        // The authored program itself round-trips too.
        let pj = serde_json::to_string(&program()).unwrap();
        let pp: AuthoredProgram = serde_json::from_str(&pj).unwrap();
        assert_eq!(pp, program());
        assert!(json.contains("\"class\":\"high\""));
    }
}
