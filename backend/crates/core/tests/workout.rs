//! Unit tests for the `fitai_core::workout` domain — the single validation
//! authority for the workout-log write model (SPEC-0004 §2.2, §3.3–§3.5).
//!
//! Authored by the qa agent during R-0004 step 3 (test planning), BEFORE the
//! `core::workout` module exists. Pre-implementation red state = compile
//! failure (the module / types are absent). Implementation step 5 makes these
//! green.
//!
//! Coverage:
//! - SAC11 → AC11/AC8: every validation branch of `Reps`, `LoadKg`, `Rpe`,
//!   `ExerciseName`, and the `NewSet`/`NewExercise`/`NewWorkoutSession`
//!   constructors — including boundary cases (min/max in-range vs just-out),
//!   the RPE half-step rule, and name trim/blank/too-long;
//! - SAC9 → AC9: the exhaustive `MuscleGroup` dual-encoding agreement
//!   (`as_str`/`parse` ↔ serde JSON), pinning the controlled set;
//! - SAC7 → AC7 (core slice): the read aggregates serialize with the literal
//!   AC7 JSON keys, with nullable `muscle_group`/`weight_kg`/`rpe`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Boundary checks round-trip values through transparent newtypes unchanged (no
// arithmetic) — `==` is the correct assertion here.
#![allow(clippy::float_cmp)]
// Test doc comments quote JSON/array literals as prose, not code.
#![allow(clippy::doc_markdown)]

use chrono::{DateTime, NaiveDate, Utc};
use fitai_core::{
    ExerciseName, LoadKg, MuscleGroup, NewExercise, NewSet, NewWorkoutSession, Reps, Rpe, UserId,
    WorkoutError, WorkoutExercise, WorkoutSession, WorkoutSet,
};
use uuid::Uuid;

/// A fixed "today" so every future-date assertion is deterministic.
fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 5, 30).unwrap()
}

/// A valid, in-range set (10 reps, 100 kg, RPE 8).
fn valid_set() -> NewSet {
    NewSet::new(10, Some(100.0), Some(8.0)).expect("a fully valid set must construct")
}

// ---------------------------------------------------------------------------
// Golden path: the full aggregate constructs from valid input.
// ---------------------------------------------------------------------------

#[test]
fn new_session_accepts_fully_valid_input() {
    let set = valid_set();
    let exercise = NewExercise::new("Bench Press", Some(MuscleGroup::Chest), vec![set.clone()])
        .expect("a valid exercise must construct");
    let session = NewWorkoutSession::new(today(), vec![exercise.clone()], today())
        .expect("a valid session must construct");

    assert_eq!(session.performed_on, today());
    assert_eq!(session.exercises, vec![exercise]);
    assert_eq!(session.exercises[0].sets, vec![set]);
    assert_eq!(session.exercises[0].name, ExerciseName::try_new("Bench Press").unwrap());
    assert_eq!(session.exercises[0].muscle_group, Some(MuscleGroup::Chest));
}

#[test]
fn new_set_accepts_omitted_optionals() {
    let set = NewSet::new(5, None, None).expect("a bodyweight set with no weight/rpe is valid");
    assert_eq!(set.weight_kg, None);
    assert_eq!(set.rpe, None);
    assert_eq!(set.reps, Reps::try_new(5).unwrap());
}

#[test]
fn new_exercise_accepts_omitted_muscle_group() {
    let exercise = NewExercise::new("Plank", None, vec![valid_set()])
        .expect("an exercise without a muscle group is valid");
    assert_eq!(exercise.muscle_group, None);
}

// ---------------------------------------------------------------------------
// AC8: performed_on must not be in the future.
// ---------------------------------------------------------------------------

#[test]
fn performed_on_today_is_accepted() {
    NewWorkoutSession::new(today(), vec![NewExercise::new("Squat", None, vec![valid_set()]).unwrap()], today())
        .expect("performed_on == today must be accepted");
}

#[test]
fn performed_on_in_the_future_is_rejected() {
    let tomorrow = NaiveDate::from_ymd_opt(2026, 5, 31).unwrap();
    let err = NewWorkoutSession::new(
        tomorrow,
        vec![NewExercise::new("Squat", None, vec![valid_set()]).unwrap()],
        today(),
    )
    .expect_err("a future performed_on must be rejected");
    assert_eq!(err, WorkoutError::PerformedOnInFuture);
    assert_eq!(err.field(), "performed_on");
}

// ---------------------------------------------------------------------------
// AC8: a session needs >= 1 exercise; an exercise needs >= 1 set.
// ---------------------------------------------------------------------------

#[test]
fn empty_exercises_is_rejected() {
    let err = NewWorkoutSession::new(today(), vec![], today())
        .expect_err("a session with no exercises must be rejected");
    assert_eq!(err, WorkoutError::ExercisesEmpty);
    assert_eq!(err.field(), "exercises");
}

#[test]
fn empty_sets_is_rejected() {
    let err = NewExercise::new("Bench Press", Some(MuscleGroup::Chest), vec![])
        .expect_err("an exercise with no sets must be rejected");
    assert_eq!(err, WorkoutError::SetsEmpty);
    assert_eq!(err.field(), "sets");
}

// ---------------------------------------------------------------------------
// AC8: ExerciseName — trimmed, non-empty, <= 100 chars.
// ---------------------------------------------------------------------------

#[test]
fn name_blank_is_rejected() {
    let err = ExerciseName::try_new("   ").expect_err("a whitespace-only name must be rejected");
    assert_eq!(err, WorkoutError::NameBlank);
    assert_eq!(err.field(), "name");
}

#[test]
fn name_empty_is_rejected() {
    let err = ExerciseName::try_new("").expect_err("an empty name must be rejected");
    assert_eq!(err, WorkoutError::NameBlank);
}

#[test]
fn name_is_trimmed() {
    let name = ExerciseName::try_new("  Bench Press  ").expect("surrounding space must trim");
    assert_eq!(name.as_str(), "Bench Press");
}

#[test]
fn name_at_maximum_length_is_accepted() {
    let max = "a".repeat(ExerciseName::MAX_CHARS);
    let name = ExerciseName::try_new(&max).expect("exactly 100 chars must be accepted");
    assert_eq!(name.as_str().chars().count(), ExerciseName::MAX_CHARS);
}

#[test]
fn name_over_maximum_length_is_rejected() {
    let too_long = "a".repeat(ExerciseName::MAX_CHARS + 1);
    let err = ExerciseName::try_new(&too_long).expect_err("101 chars must be rejected");
    assert_eq!(err, WorkoutError::NameTooLong);
    assert_eq!(err.field(), "name");
}

#[test]
fn name_length_is_measured_after_trimming() {
    // 100 visible chars plus surrounding whitespace must pass (trim first).
    let padded = format!("  {}  ", "a".repeat(ExerciseName::MAX_CHARS));
    ExerciseName::try_new(&padded).expect("trimmed length of 100 must be accepted");
}

#[test]
fn blank_name_is_rejected_through_new_exercise() {
    let err = NewExercise::new("  ", None, vec![valid_set()])
        .expect_err("a blank name must fail NewExercise::new");
    assert_eq!(err, WorkoutError::NameBlank);
    assert_eq!(err.field(), "name");
}

// ---------------------------------------------------------------------------
// AC8: Reps — range [1, 10000].
// ---------------------------------------------------------------------------

#[test]
fn reps_below_minimum_is_rejected() {
    let err = Reps::try_new(0).expect_err("0 reps must be rejected");
    assert_eq!(err, WorkoutError::RepsOutOfRange);
    assert_eq!(err.field(), "reps");
}

#[test]
fn reps_negative_is_rejected() {
    let err = Reps::try_new(-1).expect_err("negative reps must be rejected");
    assert_eq!(err, WorkoutError::RepsOutOfRange);
}

#[test]
fn reps_at_minimum_is_accepted() {
    assert_eq!(Reps::try_new(Reps::MIN).unwrap().get(), 1);
}

#[test]
fn reps_at_maximum_is_accepted() {
    assert_eq!(Reps::try_new(Reps::MAX).unwrap().get(), 10_000);
}

#[test]
fn reps_above_maximum_is_rejected() {
    let err = Reps::try_new(Reps::MAX + 1).expect_err("10001 reps must be rejected");
    assert_eq!(err, WorkoutError::RepsOutOfRange);
}

#[test]
fn out_of_range_reps_is_rejected_through_new_set() {
    let err = NewSet::new(0, None, None).expect_err("0 reps must fail NewSet::new");
    assert_eq!(err, WorkoutError::RepsOutOfRange);
    assert_eq!(err.field(), "reps");
}

// ---------------------------------------------------------------------------
// AC8: LoadKg — range (0, 1000].
// ---------------------------------------------------------------------------

#[test]
fn weight_zero_is_rejected() {
    let err = LoadKg::try_new(0.0).expect_err("0 kg must be rejected (exclusive lower bound)");
    assert_eq!(err, WorkoutError::WeightOutOfRange);
    assert_eq!(err.field(), "weight_kg");
}

#[test]
fn weight_negative_is_rejected() {
    let err = LoadKg::try_new(-5.0).expect_err("negative weight must be rejected");
    assert_eq!(err, WorkoutError::WeightOutOfRange);
}

#[test]
fn weight_just_above_zero_is_accepted() {
    // A 2.5 kg dumbbell is valid (the LoadKg-vs-WeightKg distinction, decision log).
    assert_eq!(LoadKg::try_new(2.5).unwrap().get(), 2.5);
}

#[test]
fn weight_at_maximum_is_accepted() {
    assert_eq!(LoadKg::try_new(LoadKg::MAX).unwrap().get(), 1000.0);
}

#[test]
fn weight_above_maximum_is_rejected() {
    let err = LoadKg::try_new(1000.1).expect_err("1000.1 kg must be rejected");
    assert_eq!(err, WorkoutError::WeightOutOfRange);
}

#[test]
fn weight_non_finite_is_rejected() {
    assert_eq!(
        LoadKg::try_new(f64::NAN).expect_err("NaN must be rejected"),
        WorkoutError::WeightOutOfRange
    );
    assert_eq!(
        LoadKg::try_new(f64::INFINITY).expect_err("inf must be rejected"),
        WorkoutError::WeightOutOfRange
    );
}

#[test]
fn out_of_range_weight_is_rejected_through_new_set() {
    let err = NewSet::new(10, Some(0.0), None).expect_err("0 kg must fail NewSet::new");
    assert_eq!(err, WorkoutError::WeightOutOfRange);
    assert_eq!(err.field(), "weight_kg");
}

// ---------------------------------------------------------------------------
// AC8: Rpe — [6.0, 10.0] in 0.5 steps.
// ---------------------------------------------------------------------------

#[test]
fn rpe_below_minimum_is_rejected() {
    let err = Rpe::try_new(5.5).expect_err("RPE 5.5 must be rejected");
    assert_eq!(err, WorkoutError::RpeInvalid);
    assert_eq!(err.field(), "rpe");
}

#[test]
fn rpe_at_minimum_is_accepted() {
    assert_eq!(Rpe::try_new(Rpe::MIN).unwrap().get(), 6.0);
}

#[test]
fn rpe_at_maximum_is_accepted() {
    assert_eq!(Rpe::try_new(Rpe::MAX).unwrap().get(), 10.0);
}

#[test]
fn rpe_above_maximum_is_rejected() {
    let err = Rpe::try_new(10.5).expect_err("RPE 10.5 must be rejected");
    assert_eq!(err, WorkoutError::RpeInvalid);
}

#[test]
fn rpe_half_step_is_accepted() {
    assert_eq!(Rpe::try_new(7.5).unwrap().get(), 7.5);
    assert_eq!(Rpe::try_new(8.5).unwrap().get(), 8.5);
}

#[test]
fn rpe_not_a_half_step_is_rejected() {
    let err = Rpe::try_new(7.3).expect_err("RPE 7.3 is not a multiple of 0.5");
    assert_eq!(err, WorkoutError::RpeInvalid);
    let err = Rpe::try_new(8.25).expect_err("RPE 8.25 is not a multiple of 0.5");
    assert_eq!(err, WorkoutError::RpeInvalid);
}

#[test]
fn rpe_non_finite_is_rejected() {
    assert_eq!(
        Rpe::try_new(f64::NAN).expect_err("NaN must be rejected"),
        WorkoutError::RpeInvalid
    );
}

#[test]
fn out_of_range_rpe_is_rejected_through_new_set() {
    let err = NewSet::new(10, None, Some(5.0)).expect_err("RPE 5 must fail NewSet::new");
    assert_eq!(err, WorkoutError::RpeInvalid);
    assert_eq!(err.field(), "rpe");
}

// ---------------------------------------------------------------------------
// AC9 / SAC9: MuscleGroup — the single authority. Exhaustive as_str / parse /
// serde agreement, pinning the controlled set together.
// ---------------------------------------------------------------------------

/// Every `MuscleGroup` variant — used to keep the agreement tests exhaustive.
const ALL_MUSCLE_GROUPS: [MuscleGroup; 6] = [
    MuscleGroup::Chest,
    MuscleGroup::Back,
    MuscleGroup::Shoulders,
    MuscleGroup::Arms,
    MuscleGroup::Legs,
    MuscleGroup::Core,
];

#[test]
fn muscle_group_as_str_and_parse_round_trip_for_every_variant() {
    for mg in ALL_MUSCLE_GROUPS {
        assert_eq!(
            MuscleGroup::parse(mg.as_str()),
            Ok(mg),
            "parse(as_str(v)) must equal v for {mg:?}"
        );
    }
}

#[test]
fn muscle_group_serde_matches_as_str_for_every_variant() {
    for mg in ALL_MUSCLE_GROUPS {
        let json = serde_json::to_string(&mg).unwrap();
        // A string-valued enum variant serializes as a quoted string.
        assert_eq!(
            json,
            format!("\"{}\"", mg.as_str()),
            "serde and as_str must agree for {mg:?}"
        );
        let decoded: MuscleGroup = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, mg, "serde round-trip must be identity for {mg:?}");
    }
}

#[test]
fn muscle_group_controlled_set_is_exactly_the_six_canonical_strings() {
    let canonical: Vec<&str> = ALL_MUSCLE_GROUPS.iter().map(|g| g.as_str()).collect();
    assert_eq!(
        canonical,
        vec!["chest", "back", "shoulders", "arms", "legs", "core"]
    );
}

#[test]
fn muscle_group_parse_rejects_unknown_value() {
    let err = MuscleGroup::parse("biceps").expect_err("an unknown muscle group must be rejected");
    assert_eq!(err, WorkoutError::MuscleGroupUnknown);
    assert_eq!(err.field(), "muscle_group");
}

#[test]
fn muscle_group_serde_rejects_unknown_value() {
    let decoded: Result<MuscleGroup, _> = serde_json::from_str("\"biceps\"");
    assert!(
        decoded.is_err(),
        "serde must reject a muscle group outside the controlled set"
    );
}

// ---------------------------------------------------------------------------
// AC7 / SAC7 (core slice): the read aggregates serialize with the literal AC7
// JSON keys, with nullable muscle_group / weight_kg / rpe. The handlers
// serialize the core aggregate directly (SPEC-0004 §2.4 / OQ-C1), so the wire
// contract is pinned here at the type level; the integration suite re-asserts
// the same keys end-to-end.
// ---------------------------------------------------------------------------

fn sample_session() -> WorkoutSession {
    WorkoutSession {
        id: Uuid::nil(),
        user_id: UserId(Uuid::nil()),
        performed_on: NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
        created_at: "2026-05-30T12:00:00Z".parse::<DateTime<Utc>>().unwrap(),
        updated_at: "2026-05-30T12:00:00Z".parse::<DateTime<Utc>>().unwrap(),
        exercises: vec![WorkoutExercise {
            id: Uuid::nil(),
            position: 0,
            name: ExerciseName::try_new("Bench Press").unwrap(),
            muscle_group: Some(MuscleGroup::Chest),
            sets: vec![WorkoutSet {
                id: Uuid::nil(),
                position: 0,
                reps: Reps::try_new(10).unwrap(),
                weight_kg: Some(LoadKg::try_new(100.0).unwrap()),
                rpe: Some(Rpe::try_new(8.0).unwrap()),
            }],
        }],
    }
}

#[test]
fn session_serializes_with_the_ac7_keys_and_transparent_newtypes() {
    let json = serde_json::to_value(sample_session()).unwrap();

    // Session-level keys.
    for key in ["id", "user_id", "performed_on", "created_at", "updated_at", "exercises"] {
        assert!(json.get(key).is_some(), "session JSON must carry `{key}`");
    }
    assert_eq!(json["performed_on"], serde_json::json!("2026-05-30"));

    // Exercise-level keys.
    let exercise = &json["exercises"][0];
    for key in ["id", "position", "name", "muscle_group", "sets"] {
        assert!(exercise.get(key).is_some(), "exercise JSON must carry `{key}`");
    }
    assert_eq!(exercise["name"], serde_json::json!("Bench Press"));
    assert_eq!(exercise["muscle_group"], serde_json::json!("chest"));

    // Set-level keys, transparent newtypes (reps as int, weight/rpe as numbers).
    let set = &exercise["sets"][0];
    for key in ["id", "position", "reps", "weight_kg", "rpe"] {
        assert!(set.get(key).is_some(), "set JSON must carry `{key}`");
    }
    assert_eq!(set["reps"], serde_json::json!(10));
    assert_eq!(set["weight_kg"], serde_json::json!(100.0));
    assert_eq!(set["rpe"], serde_json::json!(8.0));
}

#[test]
fn session_serializes_nullable_fields_as_null() {
    let mut session = sample_session();
    session.exercises[0].muscle_group = None;
    session.exercises[0].sets[0].weight_kg = None;
    session.exercises[0].sets[0].rpe = None;

    let json = serde_json::to_value(session).unwrap();
    let set = &json["exercises"][0]["sets"][0];

    assert!(
        json["exercises"][0]["muscle_group"].is_null(),
        "an absent muscle_group must serialize as null"
    );
    assert!(set["weight_kg"].is_null(), "an absent weight_kg must serialize as null");
    assert!(set["rpe"].is_null(), "an absent rpe must serialize as null");
}
