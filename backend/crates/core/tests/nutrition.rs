//! Unit tests for the `fitai_core::nutrition` domain — the single validation
//! authority for the nutrition-log write model and the AC9 calorie derivation
//! (SPEC-0005 §2.2, §3.3–§3.5).
//!
//! Authored by the qa agent during R-0005 step 3 (test planning), BEFORE the
//! `core::nutrition` module exists. Pre-implementation red state = compile
//! failure (the module / types are absent). Implementation step 5 makes these
//! green.
//!
//! Coverage:
//! - SAC11 → AC11/AC8: every validation branch of `Grams`, `Macros::new`, and
//!   `NewNutritionLog::new` — boundary cases (0 and 2000 in-range, just-out,
//!   negative, non-finite), the per-macro field attribution, and the future
//!   `performed_on` rule with its documented precedence over macro validation;
//! - SAC9 → AC9: `Macros::calories()` pins the exact 4/4/9 formula, including a
//!   fractional-gram case and the delegating `NutritionLog::calories()`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Boundary checks round-trip values through the transparent newtype unchanged,
// and the calorie formula is exact integer-valued / terminating-decimal f64
// arithmetic — `==` is the correct assertion here.
#![allow(clippy::float_cmp)]
// Test doc comments quote JSON literals and the calorie formula as prose.
#![allow(clippy::doc_markdown)]

use chrono::{DateTime, NaiveDate, Utc};
use fitai_core::{Grams, Macros, NewNutritionLog, NutritionError, NutritionLog, UserId};
use uuid::Uuid;

/// A fixed "today" so every future-date assertion is deterministic.
fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 5, 30).unwrap()
}

// ---------------------------------------------------------------------------
// Golden path: the write model constructs from valid input.
// ---------------------------------------------------------------------------

#[test]
fn new_nutrition_log_accepts_fully_valid_input() {
    let log = NewNutritionLog::new(today(), 150.0, 300.0, 80.0, today())
        .expect("a fully valid nutrition log must construct");
    assert_eq!(log.performed_on, today());
    assert_eq!(
        log.macros.protein,
        Grams::try_new(150.0, "protein_g").unwrap()
    );
    assert_eq!(log.macros.carbs, Grams::try_new(300.0, "carbs_g").unwrap());
    assert_eq!(log.macros.fat, Grams::try_new(80.0, "fat_g").unwrap());
}

#[test]
fn new_nutrition_log_accepts_all_zero_macros() {
    let log = NewNutritionLog::new(today(), 0.0, 0.0, 0.0, today())
        .expect("a zero-everything day is valid (0 g is in range)");
    assert_eq!(log.macros.calories(), 0.0);
}

// ---------------------------------------------------------------------------
// AC8: performed_on must not be in the future.
// ---------------------------------------------------------------------------

#[test]
fn performed_on_today_is_accepted() {
    NewNutritionLog::new(today(), 100.0, 100.0, 50.0, today())
        .expect("performed_on == today must be accepted");
}

#[test]
fn performed_on_in_the_future_is_rejected() {
    let tomorrow = NaiveDate::from_ymd_opt(2026, 5, 31).unwrap();
    let err = NewNutritionLog::new(tomorrow, 100.0, 100.0, 50.0, today())
        .expect_err("a future performed_on must be rejected");
    assert_eq!(err, NutritionError::PerformedOnInFuture);
    assert_eq!(err.field(), "performed_on");
}

#[test]
fn future_date_precedes_macro_validation_in_field_attribution() {
    // A future date AND an out-of-range macro: the future-date check runs first
    // (SPEC-0005 §3.4), so the error names "performed_on", not the bad macro.
    let tomorrow = NaiveDate::from_ymd_opt(2026, 5, 31).unwrap();
    let err = NewNutritionLog::new(tomorrow, -1.0, 100.0, 50.0, today())
        .expect_err("future date with a bad macro must still be rejected");
    assert_eq!(err, NutritionError::PerformedOnInFuture);
    assert_eq!(err.field(), "performed_on");
}

// ---------------------------------------------------------------------------
// AC8: Grams — range [0, 2000], finite. `0` is valid; negatives, >2000, and
// non-finite are rejected, each tagged with the caller-supplied field.
// ---------------------------------------------------------------------------

#[test]
fn grams_at_minimum_zero_is_accepted() {
    assert_eq!(Grams::try_new(0.0, "protein_g").unwrap().get(), 0.0);
}

#[test]
fn grams_at_maximum_is_accepted() {
    assert_eq!(
        Grams::try_new(Grams::MAX, "protein_g").unwrap().get(),
        2000.0
    );
}

#[test]
fn grams_negative_is_rejected() {
    let err =
        Grams::try_new(-0.1, "protein_g").expect_err("a negative gram value must be rejected");
    assert_eq!(err, NutritionError::MacroOutOfRange { field: "protein_g" });
    assert_eq!(err.field(), "protein_g");
}

#[test]
fn grams_above_maximum_is_rejected() {
    let err = Grams::try_new(2000.1, "carbs_g").expect_err("> 2000 g must be rejected");
    assert_eq!(err, NutritionError::MacroOutOfRange { field: "carbs_g" });
    assert_eq!(err.field(), "carbs_g");
}

#[test]
fn grams_non_finite_is_rejected() {
    assert_eq!(
        Grams::try_new(f64::NAN, "fat_g").expect_err("NaN must be rejected"),
        NutritionError::MacroOutOfRange { field: "fat_g" }
    );
    assert_eq!(
        Grams::try_new(f64::INFINITY, "fat_g").expect_err("inf must be rejected"),
        NutritionError::MacroOutOfRange { field: "fat_g" }
    );
}

#[test]
fn grams_tags_failures_with_the_caller_supplied_field() {
    assert_eq!(
        Grams::try_new(-1.0, "fat_g").expect_err("bad value"),
        NutritionError::MacroOutOfRange { field: "fat_g" }
    );
}

// ---------------------------------------------------------------------------
// AC8: Macros::new — validates all three, attributing range failures to the
// offending macro field.
// ---------------------------------------------------------------------------

#[test]
fn macros_new_accepts_in_range_values() {
    let macros = Macros::new(150.0, 300.0, 80.0).expect("in-range macros must construct");
    assert_eq!(macros.protein.get(), 150.0);
    assert_eq!(macros.carbs.get(), 300.0);
    assert_eq!(macros.fat.get(), 80.0);
}

#[test]
fn macros_new_attributes_out_of_range_protein() {
    let err = Macros::new(-1.0, 300.0, 80.0).expect_err("negative protein must be rejected");
    assert_eq!(err, NutritionError::MacroOutOfRange { field: "protein_g" });
    assert_eq!(err.field(), "protein_g");
}

#[test]
fn macros_new_attributes_out_of_range_carbs() {
    let err = Macros::new(150.0, 2000.1, 80.0).expect_err("carbs > 2000 must be rejected");
    assert_eq!(err, NutritionError::MacroOutOfRange { field: "carbs_g" });
    assert_eq!(err.field(), "carbs_g");
}

#[test]
fn macros_new_attributes_out_of_range_fat() {
    let err = Macros::new(150.0, 300.0, -5.0).expect_err("negative fat must be rejected");
    assert_eq!(err, NutritionError::MacroOutOfRange { field: "fat_g" });
    assert_eq!(err.field(), "fat_g");
}

// ---------------------------------------------------------------------------
// AC9 / SAC9: the calorie derivation is exactly 4·protein + 4·carbs + 9·fat,
// defined once on `Macros` and delegated by the read aggregate.
// ---------------------------------------------------------------------------

#[test]
fn calories_formula_is_four_four_nine() {
    // 4·150 + 4·300 + 9·80 = 600 + 1200 + 720 = 2520.
    let macros = Macros::new(150.0, 300.0, 80.0).unwrap();
    assert_eq!(macros.calories(), 2520.0);
}

#[test]
fn calories_isolates_each_macro_coefficient() {
    // Only protein -> 4 kcal/g.
    assert_eq!(Macros::new(10.0, 0.0, 0.0).unwrap().calories(), 40.0);
    // Only carbs -> 4 kcal/g.
    assert_eq!(Macros::new(0.0, 10.0, 0.0).unwrap().calories(), 40.0);
    // Only fat -> 9 kcal/g (distinguishes fat from the 4/4 macros).
    assert_eq!(Macros::new(0.0, 0.0, 10.0).unwrap().calories(), 90.0);
}

#[test]
fn calories_supports_fractional_grams() {
    // 4·0.5 + 4·0.5 + 9·0.5 = 2 + 2 + 4.5 = 8.5 (fractional-gram case, AC9).
    let macros = Macros::new(0.5, 0.5, 0.5).unwrap();
    assert_eq!(macros.calories(), 8.5);
}

#[test]
fn nutrition_log_calories_delegates_to_macros() {
    let log = NutritionLog {
        id: Uuid::nil(),
        user_id: UserId(Uuid::nil()),
        performed_on: today(),
        macros: Macros::new(150.0, 300.0, 80.0).unwrap(),
        created_at: "2026-05-30T12:00:00Z".parse::<DateTime<Utc>>().unwrap(),
        updated_at: "2026-05-30T12:00:00Z".parse::<DateTime<Utc>>().unwrap(),
    };
    assert_eq!(
        log.calories(),
        log.macros.calories(),
        "the aggregate must delegate to the single Macros authority"
    );
    assert_eq!(log.calories(), 2520.0);
}
