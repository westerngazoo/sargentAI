//! Unit tests for `core::program::instantiate` (SPEC-0014 §3.1).
//!
//! Authored by the qa agent during R-0014 step 3 (TDD red). Every test
//! references a concrete acceptance criterion from R-0014 and a spec section
//! from SPEC-0014 §2.2. All tests FAIL before step-5 implementation because
//! the helper functions are `todo!()` stubs.
//!
//! AC-coverage:
//! - AC2 (template instantiation, body-weight + goal applied as parameters):
//!   all tests below.
//! - AC10 (unit tests for template instantiation): this file.

use chrono::NaiveDate;
use uuid::Uuid;

use crate::{
    archetype::{
        Confidence, DietTemplate, FrameProfile, HeightBand, LengthBand, MacroEmphasis,
        ProgramTemplate, Provenance, Somatotype, StructureTag, TrainingPhilosophy, VolumeBand,
        WidthBand,
    },
    profile::{Goal, Goals, HeightCm, Profile, Sex, WeightKg},
    program::instantiate,
    user::UserId,
    Archetype,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Build a minimal valid [`Archetype`] with the supplied split string and
/// volume band. All other fields are filled with deterministic but
/// uninteresting values so each test can focus on one dimension.
fn archetype_fixture(split: &str, volume: VolumeBand, emphasis: MacroEmphasis) -> Archetype {
    Archetype::new(
        "test-archetype",
        "TestAthlete-99",
        "Test Archetype".to_string(),
        "A fixture archetype for unit testing.".to_string(),
        FrameProfile::new(
            1.5,
            HeightBand::Average,
            WidthBand::Average,
            LengthBand::Average,
            Somatotype::Meso,
            vec![StructureTag::DenseMuscle],
        )
        .expect("fixture frame must validate"),
        ProgramTemplate::new(
            TrainingPhilosophy::Hit,
            split.to_string(),
            2,
            volume,
            "intensity guidance text".to_string(),
            "rest guidance text".to_string(),
            "progression guidance text".to_string(),
        )
        .expect("fixture program template must validate"),
        DietTemplate::new(
            "approach text".to_string(),
            "calorie strategy text".to_string(),
            emphasis,
            "meal structure text".to_string(),
        ),
        Provenance::new(Confidence::Documented, vec!["Fixture source"]),
        vec![Goal::BuildMuscle],
    )
    .expect("fixture archetype must validate")
}

/// Build a profile for a 30-year-old male, 80 kg, 180 cm, goal = Maintain.
/// All numeric values chosen so expected TDEE arithmetic is straightforward.
///
/// DOB chosen so that `age_on(dob, TODAY)` == 30 exactly.
fn male_maintain_profile() -> Profile {
    use chrono::Utc;
    Profile {
        user_id: UserId(Uuid::nil()),
        // 30 years old on TODAY (2026-06-20)
        date_of_birth: NaiveDate::from_ymd_opt(1996, 6, 20).expect("valid date"),
        height_cm: HeightCm::try_new(180).expect("valid height"),
        weight_kg: WeightKg::try_new(80.0).expect("valid weight"),
        sex: Some(Sex::Male),
        body_fat_percentage: None,
        goals: Goals::new(vec![Goal::Maintain]).expect("valid goals"),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Same base profile but with `LoseFat` as the primary goal.
fn male_lose_fat_profile() -> Profile {
    let mut p = male_maintain_profile();
    p.goals = Goals::new(vec![Goal::LoseFat]).expect("valid goals");
    p
}

/// Same base profile but with `BuildMuscle` as the primary goal.
fn male_build_muscle_profile() -> Profile {
    let mut p = male_maintain_profile();
    p.goals = Goals::new(vec![Goal::BuildMuscle]).expect("valid goals");
    p
}

/// Same base profile but female (drives `sex_offset = -161`).
fn female_maintain_profile() -> Profile {
    let mut p = male_maintain_profile();
    p.sex = Some(Sex::Female);
    p
}

/// Profile with no sex set (drives `sex_offset = 0`).
fn no_sex_maintain_profile() -> Profile {
    let mut p = male_maintain_profile();
    p.sex = None;
    p
}

/// Today as used in all calculations, so tests that compare against manually
/// computed expected values stay consistent.
const TODAY: NaiveDate = {
    // 2026-06-20 — matches the project's currentDate memory.
    // Using a const-capable date construction.
    match NaiveDate::from_ymd_opt(2026, 6, 20) {
        Some(d) => d,
        None => panic!("invalid TODAY fixture"),
    }
};

// ---------------------------------------------------------------------------
// §2.2.1 — days_per_week derivation (AC2, SPEC-0014 §2.2.1)
// ---------------------------------------------------------------------------

/// AC2 / SPEC-0014 §2.2.1: "Upper/Lower" split → `days_per_week == 4`.
#[test]
fn instantiate_upper_lower_split_gives_4_days() {
    let arch = archetype_fixture(
        "Upper/Lower split",
        VolumeBand::Moderate,
        MacroEmphasis::Balanced,
    );
    let profile = male_maintain_profile();
    let proposal = instantiate(&arch, &profile, 0.9, 0.1, TODAY);
    assert_eq!(
        proposal.program.days_per_week, 4,
        "Upper/Lower split must yield days_per_week = 4"
    );
}

/// AC2 / SPEC-0014 §2.2.1: "PPL" split → `days_per_week == 6`.
#[test]
fn instantiate_ppl_split_gives_6_days() {
    let arch = archetype_fixture(
        "PPL rotation (push/pull/legs)",
        VolumeBand::High,
        MacroEmphasis::HighCarb,
    );
    let profile = male_maintain_profile();
    let proposal = instantiate(&arch, &profile, 0.8, 0.2, TODAY);
    assert_eq!(
        proposal.program.days_per_week, 6,
        "PPL split must yield days_per_week = 6"
    );
}

/// AC2 / SPEC-0014 §2.2.1: "Full Body" split → `days_per_week == 3`.
#[test]
fn instantiate_full_body_split_gives_3_days() {
    let arch = archetype_fixture(
        "Full Body 3x per week",
        VolumeBand::Low,
        MacroEmphasis::Balanced,
    );
    let profile = male_maintain_profile();
    let proposal = instantiate(&arch, &profile, 0.7, 0.3, TODAY);
    assert_eq!(
        proposal.program.days_per_week, 3,
        "Full Body split must yield days_per_week = 3"
    );
}

/// AC2 / SPEC-0014 §2.2.1: unknown / unrecognised split → default 4.
#[test]
fn instantiate_unknown_split_gives_4_days() {
    let arch = archetype_fixture(
        "Something entirely different",
        VolumeBand::Moderate,
        MacroEmphasis::Balanced,
    );
    let profile = male_maintain_profile();
    let proposal = instantiate(&arch, &profile, 0.6, 0.4, TODAY);
    assert_eq!(
        proposal.program.days_per_week, 4,
        "Unknown split must fall back to days_per_week = 4"
    );
}

// ---------------------------------------------------------------------------
// §2.2.1 — estimated_session_duration_min (AC2, SPEC-0014 §2.2.1)
// ---------------------------------------------------------------------------

/// AC2 / SPEC-0014 §2.2.1: `VolumeBand::Low` → 45 minutes.
#[test]
fn instantiate_volume_low_gives_45_min() {
    let arch = archetype_fixture("4-day split", VolumeBand::Low, MacroEmphasis::Balanced);
    let profile = male_maintain_profile();
    let proposal = instantiate(&arch, &profile, 0.9, 0.1, TODAY);
    assert_eq!(
        proposal.program.estimated_session_duration_min, 45,
        "Low volume must yield 45-min session"
    );
}

/// AC2 / SPEC-0014 §2.2.1: `VolumeBand::Moderate` → 60 minutes.
#[test]
fn instantiate_volume_moderate_gives_60_min() {
    let arch = archetype_fixture("4-day split", VolumeBand::Moderate, MacroEmphasis::Balanced);
    let profile = male_maintain_profile();
    let proposal = instantiate(&arch, &profile, 0.9, 0.1, TODAY);
    assert_eq!(
        proposal.program.estimated_session_duration_min, 60,
        "Moderate volume must yield 60-min session"
    );
}

/// AC2 / SPEC-0014 §2.2.1: `VolumeBand::High` → 75 minutes.
#[test]
fn instantiate_volume_high_gives_75_min() {
    let arch = archetype_fixture("4-day split", VolumeBand::High, MacroEmphasis::Balanced);
    let profile = male_maintain_profile();
    let proposal = instantiate(&arch, &profile, 0.9, 0.1, TODAY);
    assert_eq!(
        proposal.program.estimated_session_duration_min, 75,
        "High volume must yield 75-min session"
    );
}

// ---------------------------------------------------------------------------
// §2.2.2 — kcal target (AC2, SPEC-0014 §2.2.2)
//
// Manual calculation for the fixture:
//   weight = 80 kg, height = 180 cm, age = 30, sex_offset(male) = 5
//   bmr  = 10 * 80 + 6.25 * 180 - 5 * 30 + 5 = 800 + 1125 - 150 + 5 = 1780
//   tdee = 1780 * 1.55 = 2759
//   kcal(Maintain)      = 2759     → ≈ 2759
//   kcal(LoseFat)       = 2759 × 0.80 = 2207.2
//   kcal(BuildMuscle)   = 2759 × 1.15 = 3172.85
// ---------------------------------------------------------------------------

const EXPECTED_TDEE: f64 = 2759.0; // 1780 * 1.55

/// AC2 / SPEC-0014 §2.2.2: male 30yo 80kg 180cm + Maintain → kcal ≈ TDEE ± 5.
#[test]
fn instantiate_kcal_tracks_mifflin_st_jeor_male_moderate() {
    let arch = archetype_fixture("4-day split", VolumeBand::Moderate, MacroEmphasis::Balanced);
    let profile = male_maintain_profile();
    let proposal = instantiate(&arch, &profile, 0.9, 0.1, TODAY);
    // `estimated_kcal` is recomputed from rounded macros; allow ±10 for rounding.
    let kcal = f64::from(proposal.diet.estimated_kcal);
    assert!(
        (kcal - EXPECTED_TDEE).abs() <= 10.0,
        "Maintain kcal {kcal} should be within 10 of expected {EXPECTED_TDEE}"
    );
}

/// AC2 / SPEC-0014 §2.2.2: `LoseFat` goal → kcal ≈ TDEE × 0.80 ± 10.
#[test]
fn instantiate_kcal_deficit_for_lose_fat() {
    let arch = archetype_fixture("4-day split", VolumeBand::Moderate, MacroEmphasis::Balanced);
    let profile = male_lose_fat_profile();
    let proposal = instantiate(&arch, &profile, 0.9, 0.1, TODAY);
    let expected = EXPECTED_TDEE * 0.80;
    let kcal = f64::from(proposal.diet.estimated_kcal);
    assert!(
        (kcal - expected).abs() <= 10.0,
        "LoseFat kcal {kcal} should be within 10 of {expected}"
    );
}

/// AC2 / SPEC-0014 §2.2.2: `BuildMuscle` goal → kcal ≈ TDEE × 1.15 ± 10.
#[test]
fn instantiate_kcal_surplus_for_build_muscle() {
    let arch = archetype_fixture("4-day split", VolumeBand::Moderate, MacroEmphasis::Balanced);
    let profile = male_build_muscle_profile();
    let proposal = instantiate(&arch, &profile, 0.9, 0.1, TODAY);
    let expected = EXPECTED_TDEE * 1.15;
    let kcal = f64::from(proposal.diet.estimated_kcal);
    assert!(
        (kcal - expected).abs() <= 10.0,
        "BuildMuscle kcal {kcal} should be within 10 of {expected}"
    );
}

// ---------------------------------------------------------------------------
// §2.2.2 — macro split (AC2, SPEC-0014 §2.2.2)
// ---------------------------------------------------------------------------

/// AC2 / SPEC-0014 §2.2.2: `HighProtein` → protein ≈ weight × 2.2 g (± 2).
#[test]
fn instantiate_high_protein_split_protein_correct() {
    let arch = archetype_fixture(
        "4-day split",
        VolumeBand::Moderate,
        MacroEmphasis::HighProtein,
    );
    let profile = male_maintain_profile(); // weight = 80 kg
    let proposal = instantiate(&arch, &profile, 0.9, 0.1, TODAY);
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let expected_protein = (80.0_f64 * 2.2).round() as u32; // 176 g; positive by construction
    let diff = i64::from(proposal.diet.protein_g).abs_diff(i64::from(expected_protein));
    assert!(
        diff <= 2,
        "HighProtein protein {} should be ≈ {} (weight × 2.2)",
        proposal.diet.protein_g,
        expected_protein
    );
}

/// AC2 / SPEC-0014 §2.2.2: `carbs_g` is non-negative and the macro consistency
/// invariant holds even for the most extreme valid profile (minimum weight,
/// female, `LowCarb` + `LoseFat`).
///
/// With valid `Profile` inputs, Mifflin-St Jeor always yields kcal well above
/// the point where fat+protein could exceed total kcal (minimum ~1060 kcal at
/// 20 kg), so the `.max(0.0)` guard is defensive for future formula changes.
/// This test verifies the macro consistency invariant across this extreme case.
#[test]
fn instantiate_carbs_never_negative() {
    use chrono::Utc;
    let arch = archetype_fixture("4-day split", VolumeBand::Low, MacroEmphasis::LowCarb);
    let profile = Profile {
        user_id: UserId(Uuid::nil()),
        date_of_birth: NaiveDate::from_ymd_opt(1996, 6, 20).expect("valid date"),
        height_cm: HeightCm::try_new(155).expect("valid height"),
        weight_kg: WeightKg::try_new(20.0).expect("valid weight — at minimum"),
        sex: Some(Sex::Female),
        body_fat_percentage: None,
        goals: Goals::new(vec![Goal::LoseFat]).expect("valid goals"),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let proposal = instantiate(&arch, &profile, 0.5, 0.5, TODAY);
    // `carbs_g` is `u32` so it can't be negative; verify the macro consistency
    // invariant holds (no wrap, no truncation artefacts).
    let recomputed =
        proposal.diet.protein_g * 4 + proposal.diet.carbs_g * 4 + proposal.diet.fat_g * 9;
    assert_eq!(
        proposal.diet.estimated_kcal, recomputed,
        "macro consistency must hold even at minimum-weight extreme"
    );
}

/// AC2 / SPEC-0014 §2.2.2: `estimated_kcal` must equal
/// `protein_g * 4 + carbs_g * 4 + fat_g * 9` — the recompute step guarantees
/// consistency regardless of intermediate rounding.
#[test]
fn instantiate_kcal_consistent_with_macros() {
    // Run across several profiles to catch rounding edge cases.
    let profiles = [
        male_maintain_profile(),
        male_lose_fat_profile(),
        male_build_muscle_profile(),
        female_maintain_profile(),
        no_sex_maintain_profile(),
    ];
    let emphases = [
        MacroEmphasis::HighProtein,
        MacroEmphasis::Balanced,
        MacroEmphasis::HighCarb,
        MacroEmphasis::LowCarb,
    ];
    for profile in &profiles {
        for &emphasis in &emphases {
            let arch = archetype_fixture("4-day split", VolumeBand::Moderate, emphasis);
            let proposal = instantiate(&arch, profile, 0.9, 0.1, TODAY);
            let recomputed =
                proposal.diet.protein_g * 4 + proposal.diet.carbs_g * 4 + proposal.diet.fat_g * 9;
            assert_eq!(
                proposal.diet.estimated_kcal, recomputed,
                "estimated_kcal must equal protein*4 + carbs*4 + fat*9 \
                 (got {}, recomputed {})",
                proposal.diet.estimated_kcal, recomputed
            );
        }
    }
}

// ---------------------------------------------------------------------------
// §2.2 — proposal wire shape invariants (AC2, AC1)
// ---------------------------------------------------------------------------

/// The returned `archetype_id` must be the archetype's slug, not `internal_name`.
#[test]
fn instantiate_archetype_id_is_slug_never_internal_name() {
    let arch = archetype_fixture("4-day split", VolumeBand::Moderate, MacroEmphasis::Balanced);
    let profile = male_maintain_profile();
    let proposal = instantiate(&arch, &profile, 0.9, 0.1, TODAY);
    assert_eq!(proposal.archetype_id, "test-archetype");
    assert_ne!(
        proposal.archetype_id, "TestAthlete-99",
        "archetype_id must be the slug, never the internal_name"
    );
}

/// `score` and `distance` are passed through unchanged.
#[test]
fn instantiate_score_and_distance_passed_through() {
    let arch = archetype_fixture("4-day split", VolumeBand::Moderate, MacroEmphasis::Balanced);
    let profile = male_maintain_profile();
    let proposal = instantiate(&arch, &profile, 0.75, 0.25, TODAY);
    assert!(
        (proposal.score - 0.75).abs() < f64::EPSILON,
        "score must be passed through unchanged"
    );
    assert!(
        (proposal.distance - 0.25).abs() < f64::EPSILON,
        "distance must be passed through unchanged"
    );
}

/// Template string fields are copied into the generated output unmodified.
#[test]
fn instantiate_template_fields_copied_verbatim() {
    let split = "4-day rotation (delts/triceps, back, chest/biceps, legs)";
    let arch = archetype_fixture(split, VolumeBand::Low, MacroEmphasis::HighProtein);
    let profile = male_maintain_profile();
    let proposal = instantiate(&arch, &profile, 0.9, 0.1, TODAY);
    assert_eq!(proposal.program.split, split);
    assert_eq!(
        proposal.program.intensity_guidance,
        "intensity guidance text"
    );
    assert_eq!(proposal.program.rest_guidance, "rest guidance text");
    assert_eq!(
        proposal.program.progression_guidance,
        "progression guidance text"
    );
    assert_eq!(proposal.diet.approach, "approach text");
    assert_eq!(proposal.diet.calorie_strategy, "calorie strategy text");
    assert_eq!(proposal.diet.meal_structure, "meal structure text");
}

/// `whole body` (lowercase, alternate phrase) also maps to 3 days.
#[test]
fn instantiate_whole_body_split_gives_3_days() {
    let arch = archetype_fixture(
        "whole body 3 times a week",
        VolumeBand::Low,
        MacroEmphasis::Balanced,
    );
    let profile = male_maintain_profile();
    let proposal = instantiate(&arch, &profile, 0.7, 0.3, TODAY);
    assert_eq!(
        proposal.program.days_per_week, 3,
        "'whole body' split must yield days_per_week = 3"
    );
}

/// PPL match is case-insensitive: "ppl" (lowercase) must also yield 6 days.
#[test]
fn instantiate_ppl_split_case_insensitive() {
    let arch = archetype_fixture(
        "ppl (push pull legs)",
        VolumeBand::High,
        MacroEmphasis::HighCarb,
    );
    let profile = male_maintain_profile();
    let proposal = instantiate(&arch, &profile, 0.8, 0.2, TODAY);
    assert_eq!(
        proposal.program.days_per_week, 6,
        "lowercase 'ppl' must yield days_per_week = 6"
    );
}

/// Female sex offset (−161) produces a lower BMR and therefore a lower kcal
/// target than the male profile with identical weight/height/age/goal.
#[test]
fn instantiate_female_kcal_lower_than_male_same_stats() {
    let arch = archetype_fixture("4-day split", VolumeBand::Moderate, MacroEmphasis::Balanced);
    let male_proposal = instantiate(&arch, &male_maintain_profile(), 0.9, 0.1, TODAY);
    let female_proposal = instantiate(&arch, &female_maintain_profile(), 0.9, 0.1, TODAY);
    assert!(
        female_proposal.diet.estimated_kcal < male_proposal.diet.estimated_kcal,
        "female kcal {} must be less than male kcal {} (same weight/height/age/goal)",
        female_proposal.diet.estimated_kcal,
        male_proposal.diet.estimated_kcal
    );
}

/// No-sex (None) kcal target falls between male and female (offset = 0).
#[test]
fn instantiate_no_sex_kcal_between_male_and_female() {
    let arch = archetype_fixture("4-day split", VolumeBand::Moderate, MacroEmphasis::Balanced);
    let male_kcal = instantiate(&arch, &male_maintain_profile(), 0.9, 0.1, TODAY)
        .diet
        .estimated_kcal;
    let female_kcal = instantiate(&arch, &female_maintain_profile(), 0.9, 0.1, TODAY)
        .diet
        .estimated_kcal;
    let no_sex_kcal = instantiate(&arch, &no_sex_maintain_profile(), 0.9, 0.1, TODAY)
        .diet
        .estimated_kcal;
    assert!(
        no_sex_kcal > female_kcal && no_sex_kcal < male_kcal,
        "no-sex kcal {no_sex_kcal} must be between female {female_kcal} and male {male_kcal}"
    );
}
