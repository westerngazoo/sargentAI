//! Unit tests for the `fitai_core::profile` domain — the single validation
//! authority for the user-profile write model (SPEC-0003 §2.2, §3.3–§3.5).
//!
//! Authored by the qa agent during R-0003 step 3 (test planning), BEFORE the
//! `core::profile` module exists. Pre-implementation red state = compile
//! failure (the module / types are absent). Implementation step 5 makes these
//! green.
//!
//! Coverage (SAC8 → AC8): every AC5/AC6 validation rule, the derived-age
//! computation across a birthday boundary, the `Goals` non-empty/dedup
//! invariant, and the exhaustive `Sex`/`Goal` `as_str`↔`parse`↔serde agreement
//! that pins the two encodings together (SPEC-0003 §2.4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::NaiveDate;
use fitai_core::{
    BodyFatPercentage, Goal, Goals, HeightCm, NewProfile, ProfileError, Sex, WeightKg,
};

/// A fixed "today" so every age assertion is deterministic.
fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 5, 30).unwrap()
}

/// A DOB that is comfortably valid (age 30 on `today`).
fn valid_dob() -> NaiveDate {
    NaiveDate::from_ymd_opt(1996, 1, 1).unwrap()
}

/// Build a `NewProfile` with all-valid inputs except the overrides a test sets
/// by calling `NewProfile::new` directly. Used to assert the golden path.
fn new_valid_profile() -> Result<NewProfile, ProfileError> {
    NewProfile::new(
        valid_dob(),
        180,
        80.0,
        vec![Goal::BuildMuscle],
        Some(Sex::Male),
        Some(20.0),
        today(),
    )
}

// ---------------------------------------------------------------------------
// Golden path.
// ---------------------------------------------------------------------------

#[test]
fn new_profile_accepts_fully_valid_input() {
    let profile = new_valid_profile().expect("a fully valid profile must construct");
    assert_eq!(profile.date_of_birth, valid_dob());
    assert_eq!(profile.height_cm, HeightCm::try_new(180).unwrap());
    assert_eq!(profile.weight_kg, WeightKg::try_new(80.0).unwrap());
    assert_eq!(profile.sex, Some(Sex::Male));
    assert_eq!(
        profile.body_fat_percentage,
        Some(BodyFatPercentage::try_new(20.0).unwrap())
    );
    assert_eq!(profile.goals, Goals::new(vec![Goal::BuildMuscle]).unwrap());
}

#[test]
fn new_profile_accepts_omitted_optionals() {
    let profile = NewProfile::new(
        valid_dob(),
        180,
        80.0,
        vec![Goal::Maintain],
        None,
        None,
        today(),
    )
    .expect("omitted sex/body_fat must be allowed");
    assert_eq!(profile.sex, None);
    assert_eq!(profile.body_fat_percentage, None);
}

// ---------------------------------------------------------------------------
// AC5: date_of_birth / age.
// ---------------------------------------------------------------------------

#[test]
fn date_of_birth_in_the_future_is_rejected() {
    let tomorrow = NaiveDate::from_ymd_opt(2026, 5, 31).unwrap();
    let err = NewProfile::new(
        tomorrow,
        180,
        80.0,
        vec![Goal::Maintain],
        None,
        None,
        today(),
    )
    .expect_err("a future DOB must be rejected");
    assert_eq!(err, ProfileError::DateOfBirthInFuture);
    assert_eq!(err.field(), "date_of_birth");
}

#[test]
fn age_below_minimum_is_rejected() {
    // Born 12 years ago exactly → age 12 < MIN_AGE (13).
    let dob = NaiveDate::from_ymd_opt(2014, 5, 30).unwrap();
    let err = NewProfile::new(dob, 150, 45.0, vec![Goal::Maintain], None, None, today())
        .expect_err("age 12 must be rejected");
    assert_eq!(err, ProfileError::AgeOutOfRange);
    assert_eq!(err.field(), "date_of_birth");
}

#[test]
fn age_exactly_minimum_is_accepted() {
    // Born 13 years ago to the day → age 13 == MIN_AGE.
    let dob = NaiveDate::from_ymd_opt(2013, 5, 30).unwrap();
    NewProfile::new(dob, 150, 45.0, vec![Goal::Maintain], None, None, today())
        .expect("age exactly 13 must be accepted");
}

#[test]
fn age_above_maximum_is_rejected() {
    // Born 121 years ago → age 121 > MAX_AGE (120).
    let dob = NaiveDate::from_ymd_opt(1905, 5, 30).unwrap();
    let err = NewProfile::new(dob, 170, 70.0, vec![Goal::Maintain], None, None, today())
        .expect_err("age 121 must be rejected");
    assert_eq!(err, ProfileError::AgeOutOfRange);
    assert_eq!(err.field(), "date_of_birth");
}

#[test]
fn age_exactly_maximum_is_accepted() {
    // Born 120 years ago to the day → age 120 == MAX_AGE.
    let dob = NaiveDate::from_ymd_opt(1906, 5, 30).unwrap();
    NewProfile::new(dob, 170, 70.0, vec![Goal::Maintain], None, None, today())
        .expect("age exactly 120 must be accepted");
}

// ---------------------------------------------------------------------------
// AC5: height_cm range [50, 300].
// ---------------------------------------------------------------------------

#[test]
fn height_below_minimum_is_rejected() {
    let err = HeightCm::try_new(49).expect_err("49 cm must be rejected");
    assert_eq!(err, ProfileError::HeightOutOfRange);
    assert_eq!(err.field(), "height_cm");
}

#[test]
fn height_at_minimum_is_accepted() {
    assert_eq!(HeightCm::try_new(50).unwrap().get(), 50);
}

#[test]
fn height_at_maximum_is_accepted() {
    assert_eq!(HeightCm::try_new(300).unwrap().get(), 300);
}

#[test]
fn height_above_maximum_is_rejected() {
    let err = HeightCm::try_new(301).expect_err("301 cm must be rejected");
    assert_eq!(err, ProfileError::HeightOutOfRange);
}

// ---------------------------------------------------------------------------
// AC5: weight_kg range [20, 500].
// ---------------------------------------------------------------------------

#[test]
fn weight_below_minimum_is_rejected() {
    let err = WeightKg::try_new(19.9).expect_err("19.9 kg must be rejected");
    assert_eq!(err, ProfileError::WeightOutOfRange);
    assert_eq!(err.field(), "weight_kg");
}

#[test]
fn weight_at_minimum_is_accepted() {
    assert_eq!(WeightKg::try_new(20.0).unwrap().get(), 20.0);
}

#[test]
fn weight_at_maximum_is_accepted() {
    assert_eq!(WeightKg::try_new(500.0).unwrap().get(), 500.0);
}

#[test]
fn weight_above_maximum_is_rejected() {
    let err = WeightKg::try_new(500.1).expect_err("500.1 kg must be rejected");
    assert_eq!(err, ProfileError::WeightOutOfRange);
}

#[test]
fn weight_non_finite_is_rejected() {
    assert_eq!(
        WeightKg::try_new(f64::NAN).expect_err("NaN must be rejected"),
        ProfileError::WeightOutOfRange
    );
    assert_eq!(
        WeightKg::try_new(f64::INFINITY).expect_err("inf must be rejected"),
        ProfileError::WeightOutOfRange
    );
}

// ---------------------------------------------------------------------------
// AC5: body_fat_percentage range [1, 75] (optional).
// ---------------------------------------------------------------------------

#[test]
fn body_fat_below_minimum_is_rejected() {
    let err = BodyFatPercentage::try_new(0.9).expect_err("0.9% must be rejected");
    assert_eq!(err, ProfileError::BodyFatOutOfRange);
    assert_eq!(err.field(), "body_fat_percentage");
}

#[test]
fn body_fat_at_minimum_is_accepted() {
    assert_eq!(BodyFatPercentage::try_new(1.0).unwrap().get(), 1.0);
}

#[test]
fn body_fat_at_maximum_is_accepted() {
    assert_eq!(BodyFatPercentage::try_new(75.0).unwrap().get(), 75.0);
}

#[test]
fn body_fat_above_maximum_is_rejected() {
    let err = BodyFatPercentage::try_new(75.1).expect_err("75.1% must be rejected");
    assert_eq!(err, ProfileError::BodyFatOutOfRange);
}

#[test]
fn body_fat_non_finite_is_rejected() {
    assert_eq!(
        BodyFatPercentage::try_new(f64::NAN).expect_err("NaN must be rejected"),
        ProfileError::BodyFatOutOfRange
    );
}

#[test]
fn out_of_range_body_fat_is_rejected_through_new_profile() {
    let err = NewProfile::new(
        valid_dob(),
        180,
        80.0,
        vec![Goal::Maintain],
        None,
        Some(0.5),
        today(),
    )
    .expect_err("an out-of-range body_fat must fail NewProfile::new");
    assert_eq!(err, ProfileError::BodyFatOutOfRange);
    assert_eq!(err.field(), "body_fat_percentage");
}

// ---------------------------------------------------------------------------
// AC5/AC6: goals — non-empty, no duplicates, controlled set.
// ---------------------------------------------------------------------------

#[test]
fn empty_goals_is_rejected() {
    let err = Goals::new(vec![]).expect_err("empty goals must be rejected");
    assert_eq!(err, ProfileError::GoalsEmpty);
    assert_eq!(err.field(), "goals");
}

#[test]
fn duplicate_goals_are_rejected() {
    let err = Goals::new(vec![Goal::BuildMuscle, Goal::BuildMuscle])
        .expect_err("duplicate goals must be rejected");
    assert_eq!(err, ProfileError::GoalsDuplicate);
    assert_eq!(err.field(), "goals");
}

#[test]
fn multi_select_goals_are_accepted_and_order_preserved() {
    let goals = Goals::new(vec![Goal::BuildMuscle, Goal::LoseFat]).unwrap();
    assert_eq!(goals.as_slice(), &[Goal::BuildMuscle, Goal::LoseFat]);
}

#[test]
fn empty_goals_rejected_through_new_profile() {
    let err = NewProfile::new(valid_dob(), 180, 80.0, vec![], None, None, today())
        .expect_err("empty goals must fail NewProfile::new");
    assert_eq!(err, ProfileError::GoalsEmpty);
    assert_eq!(err.field(), "goals");
}

// ---------------------------------------------------------------------------
// AC6: the controlled vocabularies — exhaustive as_str/parse/serde agreement.
// ---------------------------------------------------------------------------

#[test]
fn goal_as_str_and_parse_round_trip_for_every_variant() {
    for goal in [
        Goal::LoseFat,
        Goal::BuildMuscle,
        Goal::Recomp,
        Goal::Maintain,
        Goal::GainStrength,
    ] {
        assert_eq!(
            Goal::parse(goal.as_str()),
            Ok(goal),
            "parse(as_str(v)) must equal v for {goal:?}"
        );
    }
}

#[test]
fn goal_serde_matches_as_str_for_every_variant() {
    for goal in [
        Goal::LoseFat,
        Goal::BuildMuscle,
        Goal::Recomp,
        Goal::Maintain,
        Goal::GainStrength,
    ] {
        let json = serde_json::to_string(&goal).unwrap();
        // serde serializes a string-valued enum variant as a quoted string.
        assert_eq!(
            json,
            format!("\"{}\"", goal.as_str()),
            "serde and as_str must agree for {goal:?}"
        );
        // ...and deserialize is the inverse.
        let decoded: Goal = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, goal);
    }
}

#[test]
fn goal_controlled_set_is_exactly_the_five_canonical_strings() {
    let canonical: Vec<&str> = [
        Goal::LoseFat,
        Goal::BuildMuscle,
        Goal::Recomp,
        Goal::Maintain,
        Goal::GainStrength,
    ]
    .iter()
    .map(|g| g.as_str())
    .collect();
    assert_eq!(
        canonical,
        vec!["lose_fat", "build_muscle", "recomp", "maintain", "gain_strength"]
    );
}

#[test]
fn goal_parse_rejects_unknown_value() {
    let err = Goal::parse("bulk").expect_err("an unknown goal must be rejected");
    assert_eq!(err, ProfileError::GoalUnknown);
    assert_eq!(err.field(), "goals");
}

#[test]
fn sex_as_str_and_parse_round_trip_for_every_variant() {
    for sex in [Sex::Male, Sex::Female] {
        assert_eq!(Sex::parse(sex.as_str()), Ok(sex));
    }
}

#[test]
fn sex_serde_matches_as_str_for_every_variant() {
    for sex in [Sex::Male, Sex::Female] {
        let json = serde_json::to_string(&sex).unwrap();
        assert_eq!(json, format!("\"{}\"", sex.as_str()));
        let decoded: Sex = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, sex);
    }
}

#[test]
fn sex_parse_rejects_unknown_value() {
    let err = Sex::parse("other").expect_err("an unknown sex must be rejected");
    assert_eq!(err, ProfileError::SexUnknown);
    assert_eq!(err.field(), "sex");
}

// ---------------------------------------------------------------------------
// Derived age (AC4) — exercised through the read aggregate.
// ---------------------------------------------------------------------------

#[test]
fn age_on_is_correct_before_the_birthday() {
    // Born 1996-07-01; on 2026-05-30 the birthday has NOT yet passed → 29.
    let dob = NaiveDate::from_ymd_opt(1996, 7, 1).unwrap();
    assert_eq!(fitai_core::profile::age_on(dob, today()), 29);
}

#[test]
fn age_on_increments_on_the_birthday() {
    // Born 1996-05-30. The day before the 2026 birthday → 29; on it → 30.
    let dob = NaiveDate::from_ymd_opt(1996, 5, 30).unwrap();
    let day_before = NaiveDate::from_ymd_opt(2026, 5, 29).unwrap();
    let on_birthday = NaiveDate::from_ymd_opt(2026, 5, 30).unwrap();

    assert_eq!(fitai_core::profile::age_on(dob, day_before), 29);
    assert_eq!(fitai_core::profile::age_on(dob, on_birthday), 30);
}
