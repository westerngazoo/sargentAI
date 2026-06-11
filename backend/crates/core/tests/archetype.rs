//! Unit tests for the `fitai_core::archetype` domain — the curated archetype
//! library (the matching **prior**): the validated `Archetype` model, its value
//! types, and the embedded six-record library (SPEC-0012 §2.2/§2.3/§3).
//!
//! Authored by the qa agent during R-0012 step 3 (test planning), BEFORE the
//! `core::archetype` module exists. Pre-implementation red state = compile
//! failure (the module / types / `library()` / `find()` are absent).
//! Implementation step 5 makes these green.
//!
//! SAC → test traceability (the full table lives in the qa sign-off report):
//! - SAC1 → AC1: every validator (`FrameProfile`/`ProgramTemplate`/`Provenance`/
//!   `Archetype`) accepts in-range/non-empty input and rejects the out-of-range
//!   ratio, the out-of-range frequency, empty goals, and empty names — each with
//!   the right `ArchetypeError::field()`.
//! - SAC2 → AC2/AC7: `library()` returns EXACTLY SIX records; every record
//!   re-validates (internal consistency); the ids are unique kebab slugs; each
//!   `goals_served` is non-empty; provenance is honest (Yates-96/Mentzer are
//!   `Documented`, Arnold/Columbu/Cutler/Heath are `Reconstructed`).
//! - SAC5 → AC5: `library()`/`find()` are the only read path (the prior lives in
//!   `core::archetype`); `find` resolves known ids and rejects unknown.
//! - SAC6 → AC6: `FrameProfile` exposes the numeric `shoulder_to_waist`, the
//!   banded/enum fields, and a `Vec<StructureTag>`; the controlled enums encode
//!   on the wire as the documented lowercase vocabulary (e.g. confidence).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Boundary checks round-trip f64 ratios through the validator unchanged — `==`
// is the correct assertion here.
#![allow(clippy::float_cmp)]
// Test doc comments quote JSON literals and slug strings as prose.
#![allow(clippy::doc_markdown)]

use std::collections::HashSet;

use fitai_core::archetype::{
    find, library, Archetype, ArchetypeError, Confidence, DietTemplate, FrameProfile, HeightBand,
    LengthBand, MacroEmphasis, ProgramTemplate, Provenance, Somatotype, StructureTag,
    TrainingPhilosophy, VolumeBand, WidthBand,
};
use fitai_core::Goal;

// ===========================================================================
// Builders for valid value parts — the golden input each rejection test mutates
// off, mirroring `valid_body()` in the integration suites.
// ===========================================================================

/// A valid frame profile (shoulder_to_waist mid-range, every enum populated).
fn valid_frame(shoulder_to_waist: f64) -> Result<FrameProfile, ArchetypeError> {
    FrameProfile::new(
        shoulder_to_waist,
        HeightBand::Average,
        WidthBand::Wide,
        LengthBand::Average,
        Somatotype::Meso,
        vec![StructureTag::WideClavicles, StructureTag::NarrowHips],
    )
}

/// A valid program template at the given weekly frequency per muscle.
fn valid_program(weekly_frequency_per_muscle: u8) -> Result<ProgramTemplate, ArchetypeError> {
    ProgramTemplate::new(
        TrainingPhilosophy::Hit,
        "full-body split".to_string(),
        weekly_frequency_per_muscle,
        VolumeBand::Low,
        "1-2 working sets to failure".to_string(),
        "2-4 min".to_string(),
        "add load when reps target is met".to_string(),
    )
}

/// A valid diet template.
fn valid_diet() -> DietTemplate {
    DietTemplate::new(
        "high-protein surplus".to_string(),
        "lean bulk, +300 kcal".to_string(),
        MacroEmphasis::HighProtein,
        "4 meals".to_string(),
    )
}

/// A valid provenance with the given confidence.
fn valid_provenance(confidence: Confidence) -> Provenance {
    Provenance::new(confidence, vec!["Blood & Guts (1996)"])
}

/// Assemble a valid `Archetype` over the given parts, with overridable names and
/// goals so the rejection tests can pierce one field at a time.
fn archetype_with(
    id: &'static str,
    internal_name: &'static str,
    display_name: &str,
    goals: Vec<Goal>,
) -> Result<Archetype, ArchetypeError> {
    Archetype::new(
        id,
        internal_name,
        display_name.to_string(),
        "a starting prior to personalize from".to_string(),
        valid_frame(1.6)?,
        valid_program(3)?,
        valid_diet(),
        valid_provenance(Confidence::Documented),
        goals,
    )
}

/// A fully valid archetype — the golden case.
fn valid_archetype() -> Archetype {
    archetype_with(
        "heavy-duty-mass",
        "Yates-96",
        "heavy-duty-mass",
        vec![Goal::BuildMuscle, Goal::GainStrength],
    )
    .expect("the golden archetype must construct")
}

// ===========================================================================
// SAC1 / AC1: FrameProfile::new — shoulder_to_waist range [1.0, 2.5].
// The numeric V-taper proxy is the field R-0013 matches on; out-of-range
// ratios are rejected with field "shoulder_to_waist".
// ===========================================================================

#[test]
fn frame_profile_accepts_a_mid_range_ratio() {
    let frame = valid_frame(1.6).expect("a mid-range ratio must be accepted");
    assert_eq!(frame.shoulder_to_waist, 1.6);
    assert_eq!(frame.height_band, HeightBand::Average);
    assert_eq!(frame.clavicle_width, WidthBand::Wide);
    assert_eq!(frame.limb_length, LengthBand::Average);
    assert_eq!(frame.build, Somatotype::Meso);
    assert_eq!(
        frame.structure_tags,
        vec![StructureTag::WideClavicles, StructureTag::NarrowHips]
    );
}

#[test]
fn frame_profile_accepts_the_lower_bound() {
    let frame = valid_frame(1.0).expect("exactly 1.0 is in range");
    assert_eq!(frame.shoulder_to_waist, 1.0);
}

#[test]
fn frame_profile_accepts_the_upper_bound() {
    let frame = valid_frame(2.5).expect("exactly 2.5 is in range");
    assert_eq!(frame.shoulder_to_waist, 2.5);
}

#[test]
fn frame_profile_rejects_a_ratio_below_the_lower_bound() {
    let err = valid_frame(0.9).expect_err("0.9 is below 1.0 and must be rejected");
    assert_eq!(err.field(), "shoulder_to_waist");
}

#[test]
fn frame_profile_rejects_a_ratio_above_the_upper_bound() {
    let err = valid_frame(2.6).expect_err("2.6 is above 2.5 and must be rejected");
    assert_eq!(err.field(), "shoulder_to_waist");
}

#[test]
fn frame_profile_rejects_a_non_finite_ratio() {
    assert_eq!(
        valid_frame(f64::NAN)
            .expect_err("NaN must be rejected")
            .field(),
        "shoulder_to_waist"
    );
    assert_eq!(
        valid_frame(f64::INFINITY)
            .expect_err("inf must be rejected")
            .field(),
        "shoulder_to_waist"
    );
}

// ===========================================================================
// SAC1 / AC1: ProgramTemplate::new — weekly_frequency_per_muscle range [1, 7].
// ===========================================================================

#[test]
fn program_template_accepts_a_mid_range_frequency() {
    let program = valid_program(3).expect("3x/week is in range");
    assert_eq!(program.weekly_frequency_per_muscle, 3);
    assert_eq!(program.philosophy, TrainingPhilosophy::Hit);
    assert_eq!(program.volume, VolumeBand::Low);
}

#[test]
fn program_template_accepts_the_lower_bound_frequency() {
    assert_eq!(
        valid_program(1)
            .expect("exactly 1x/week is in range")
            .weekly_frequency_per_muscle,
        1
    );
}

#[test]
fn program_template_accepts_the_upper_bound_frequency() {
    assert_eq!(
        valid_program(7)
            .expect("exactly 7x/week is in range")
            .weekly_frequency_per_muscle,
        7
    );
}

#[test]
fn program_template_rejects_a_zero_frequency() {
    let err = valid_program(0).expect_err("0x/week must be rejected");
    assert_eq!(err.field(), "weekly_frequency_per_muscle");
}

#[test]
fn program_template_rejects_a_frequency_above_seven() {
    let err = valid_program(8).expect_err("8x/week must be rejected");
    assert_eq!(err.field(), "weekly_frequency_per_muscle");
}

// ===========================================================================
// SAC1 / AC1: Archetype::new — non-empty goals_served and non-empty names.
// ===========================================================================

#[test]
fn archetype_accepts_fully_valid_input() {
    let a = valid_archetype();
    assert_eq!(a.id, "heavy-duty-mass");
    assert_eq!(a.internal_name, "Yates-96");
    assert_eq!(a.display_name, "heavy-duty-mass");
    assert!(!a.summary.is_empty());
    assert_eq!(a.goals_served, vec![Goal::BuildMuscle, Goal::GainStrength]);
    assert_eq!(a.provenance.confidence, Confidence::Documented);
}

#[test]
fn archetype_rejects_empty_goals_served() {
    let err = archetype_with("heavy-duty-mass", "Yates-96", "heavy-duty-mass", vec![])
        .expect_err("an empty goals_served must be rejected");
    assert_eq!(err.field(), "goals_served");
}

#[test]
fn archetype_rejects_an_empty_display_name() {
    let err = archetype_with("heavy-duty-mass", "Yates-96", "", vec![Goal::BuildMuscle])
        .expect_err("an empty display_name must be rejected");
    assert_eq!(err.field(), "display_name");
}

#[test]
fn archetype_rejects_an_empty_internal_name() {
    let err = archetype_with(
        "heavy-duty-mass",
        "",
        "heavy-duty-mass",
        vec![Goal::BuildMuscle],
    )
    .expect_err("an empty internal_name must be rejected");
    assert_eq!(err.field(), "internal_name");
}

// ===========================================================================
// SAC1 / AC1: ArchetypeError.field() routes every variant to its field name,
// exactly as the photo/nutrition/profile error idiom.
// ===========================================================================

#[test]
fn archetype_error_field_attribution_is_exhaustive() {
    // Each validation branch is reachable and names its own field; the prior
    // per-validator tests pin the individual mappings, this asserts they are all
    // distinct and stable.
    let fields: HashSet<&'static str> = [
        valid_frame(0.9).unwrap_err().field(),
        valid_program(0).unwrap_err().field(),
        archetype_with("id", "Yates-96", "d", vec![])
            .unwrap_err()
            .field(),
        archetype_with("id", "Yates-96", "", vec![Goal::BuildMuscle])
            .unwrap_err()
            .field(),
        archetype_with("id", "", "d", vec![Goal::BuildMuscle])
            .unwrap_err()
            .field(),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        fields,
        HashSet::from([
            "shoulder_to_waist",
            "weekly_frequency_per_muscle",
            "goals_served",
            "display_name",
            "internal_name",
        ]),
        "every validation branch must name a distinct, stable field"
    );
}

// ===========================================================================
// SAC2 / AC2 / AC7: the embedded library — EXACTLY six internally consistent,
// provenance-honest records with unique kebab-slug ids.
// ===========================================================================

#[test]
fn library_returns_exactly_six_records() {
    assert_eq!(
        library().len(),
        6,
        "the seed library must carry exactly the six approved archetypes"
    );
}

#[test]
fn library_records_each_revalidate_for_internal_consistency() {
    // Reconstructing each seed record through `Archetype::new` from its own parts
    // must succeed — proving every shipped record honours every invariant (SAC2:
    // "an invalid record can never ship"). The frame ratio is the field most
    // likely to drift out of range, so it is re-checked explicitly.
    for a in library() {
        assert!(
            (1.0..=2.5).contains(&a.frame_profile.shoulder_to_waist),
            "{}: shoulder_to_waist {} must be in [1.0, 2.5]",
            a.id,
            a.frame_profile.shoulder_to_waist
        );
        assert!(
            (1..=7).contains(&a.program_template.weekly_frequency_per_muscle),
            "{}: weekly_frequency_per_muscle must be in [1, 7]",
            a.id
        );
        assert!(
            !a.goals_served.is_empty(),
            "{}: goals_served non-empty",
            a.id
        );
        assert!(
            !a.display_name.is_empty(),
            "{}: display_name non-empty",
            a.id
        );
        assert!(
            !a.internal_name.is_empty(),
            "{}: internal_name non-empty",
            a.id
        );
        assert!(!a.id.is_empty(), "every record must carry a non-empty id");

        // A full reconstruction through the validating constructor must succeed.
        let rebuilt = Archetype::new(
            a.id,
            a.internal_name,
            a.display_name.clone(),
            a.summary.clone(),
            FrameProfile::new(
                a.frame_profile.shoulder_to_waist,
                a.frame_profile.height_band,
                a.frame_profile.clavicle_width,
                a.frame_profile.limb_length,
                a.frame_profile.build,
                a.frame_profile.structure_tags.clone(),
            )
            .unwrap_or_else(|e| panic!("{}: frame must revalidate ({})", a.id, e.field())),
            ProgramTemplate::new(
                a.program_template.philosophy,
                a.program_template.split.clone(),
                a.program_template.weekly_frequency_per_muscle,
                a.program_template.volume,
                a.program_template.intensity.clone(),
                a.program_template.rest.clone(),
                a.program_template.progression.clone(),
            )
            .unwrap_or_else(|e| panic!("{}: program must revalidate ({})", a.id, e.field())),
            DietTemplate::new(
                a.diet_template.approach.clone(),
                a.diet_template.calorie_strategy.clone(),
                a.diet_template.macro_emphasis,
                a.diet_template.meal_structure.clone(),
            ),
            Provenance::new(a.provenance.confidence, a.provenance.sources.clone()),
            a.goals_served.clone(),
        );
        assert!(
            rebuilt.is_ok(),
            "{}: every shipped record must re-validate through Archetype::new",
            a.id
        );
    }
}

#[test]
fn library_ids_are_unique_kebab_slugs() {
    let mut seen = HashSet::new();
    for a in library() {
        assert!(
            seen.insert(a.id),
            "archetype id {:?} is duplicated; ids must be unique",
            a.id
        );
        assert!(
            a.id.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "id {:?} must be a kebab slug (lowercase, digits, hyphens)",
            a.id
        );
        assert!(
            !a.id.starts_with('-') && !a.id.ends_with('-') && !a.id.contains("--"),
            "id {:?} must not have leading/trailing/double hyphens",
            a.id
        );
    }
    assert_eq!(seen.len(), 6, "all six ids must be distinct");
}

#[test]
fn library_internal_names_are_the_six_research_labels() {
    let mut names: Vec<&str> = library().iter().map(|a| a.internal_name).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec![
            "Arnold-70s",
            "Columbu",
            "Cutler-00s",
            "Heath-10s",
            "Mentzer",
            "Yates-96",
        ],
        "the six internal research labels must be exactly the owner-approved set"
    );
}

#[test]
fn library_provenance_is_honest_documented_versus_reconstructed() {
    // AC7: only the well-documented routines/diets carry `Documented`; the
    // reconstructed ones are flagged accordingly — no fabricated precision.
    let confidence_of = |internal_name: &str| -> Confidence {
        library()
            .iter()
            .find(|a| a.internal_name == internal_name)
            .unwrap_or_else(|| panic!("seed must contain {internal_name}"))
            .provenance
            .confidence
    };

    assert_eq!(
        confidence_of("Yates-96"),
        Confidence::Documented,
        "Yates' Blood & Guts is well-documented"
    );
    assert_eq!(
        confidence_of("Mentzer"),
        Confidence::Documented,
        "Mentzer's HIT books are well-documented"
    );
    for reconstructed in ["Arnold-70s", "Columbu", "Cutler-00s", "Heath-10s"] {
        assert_eq!(
            confidence_of(reconstructed),
            Confidence::Reconstructed,
            "{reconstructed} is reconstructed, not documented"
        );
    }
}

#[test]
fn library_every_record_serves_at_least_one_goal() {
    for a in library() {
        assert!(
            !a.goals_served.is_empty(),
            "{}: every record must serve at least one goal (non-empty goals_served)",
            a.id
        );
    }
}

// ===========================================================================
// SAC5 / AC5: find() — the single read path into the prior; known ids resolve,
// unknown ids return None (never a panic, never a wildcard match).
// ===========================================================================

#[test]
fn find_resolves_every_library_id() {
    for a in library() {
        let found = find(a.id).unwrap_or_else(|| panic!("find({:?}) must resolve", a.id));
        assert_eq!(found.id, a.id);
        assert_eq!(found.internal_name, a.internal_name);
    }
}

#[test]
fn find_returns_none_for_an_unknown_id() {
    assert!(
        find("no-such-archetype").is_none(),
        "an unknown id must resolve to None"
    );
    assert!(find("").is_none(), "the empty id must resolve to None");
}

// ===========================================================================
// SAC6 / AC6: the FrameProfile is matchable — it exposes the numeric ratio plus
// the banded/enum fields and a controlled `StructureTag` vocabulary, and the
// controlled enums encode on the wire as the documented lowercase vocabulary.
// ===========================================================================

#[test]
fn frame_profile_exposes_numeric_ratio_and_banded_enum_fields() {
    let frame = valid_frame(1.75).unwrap();
    // The numeric ratio R-0013 computes distance against.
    assert_eq!(frame.shoulder_to_waist, 1.75);
    // The banded/enum descriptors.
    let _: HeightBand = frame.height_band;
    let _: WidthBand = frame.clavicle_width;
    let _: LengthBand = frame.limb_length;
    let _: Somatotype = frame.build;
    // The controlled-vocabulary tag list (not free strings).
    let _: &Vec<StructureTag> = &frame.structure_tags;
    assert!(frame.structure_tags.contains(&StructureTag::WideClavicles));
}

#[test]
fn confidence_serializes_as_the_lowercase_controlled_vocabulary() {
    // The provenance level crosses the wire (SPEC-0012 §2.4) and must encode as
    // the documented lowercase tokens, mirroring `Angle`/`Sex`.
    assert_eq!(
        serde_json::to_string(&Confidence::Documented).unwrap(),
        "\"documented\""
    );
    assert_eq!(
        serde_json::to_string(&Confidence::Reconstructed).unwrap(),
        "\"reconstructed\""
    );
    assert_eq!(
        serde_json::to_string(&Confidence::Folklore).unwrap(),
        "\"folklore\""
    );
}

#[test]
fn volume_band_serializes_as_the_lowercase_controlled_vocabulary() {
    assert_eq!(serde_json::to_string(&VolumeBand::Low).unwrap(), "\"low\"");
    assert_eq!(
        serde_json::to_string(&VolumeBand::Moderate).unwrap(),
        "\"moderate\""
    );
    assert_eq!(
        serde_json::to_string(&VolumeBand::High).unwrap(),
        "\"high\""
    );
}

#[test]
fn width_length_and_somatotype_bands_serialize_as_lowercase() {
    assert_eq!(
        serde_json::to_string(&WidthBand::Narrow).unwrap(),
        "\"narrow\""
    );
    assert_eq!(
        serde_json::to_string(&WidthBand::Average).unwrap(),
        "\"average\""
    );
    assert_eq!(serde_json::to_string(&WidthBand::Wide).unwrap(), "\"wide\"");
    assert_eq!(
        serde_json::to_string(&LengthBand::Short).unwrap(),
        "\"short\""
    );
    assert_eq!(
        serde_json::to_string(&LengthBand::Long).unwrap(),
        "\"long\""
    );
    assert_eq!(
        serde_json::to_string(&Somatotype::Ecto).unwrap(),
        "\"ecto\""
    );
    assert_eq!(
        serde_json::to_string(&Somatotype::Meso).unwrap(),
        "\"meso\""
    );
    assert_eq!(
        serde_json::to_string(&Somatotype::Endo).unwrap(),
        "\"endo\""
    );
}

// ===========================================================================
// SAC4 / AC4 (type-level): the core `Archetype` aggregate is intentionally NOT
// `Serialize` — the wire shape is owned by the api `ArchetypeResponse` DTO so
// `internal_name`/`sources` cannot leak. This is pinned structurally here (a
// `Provenance` carries internal `sources`); the integration suite asserts the
// serialized body. The FrameProfile/enum wire encodings above are the only core
// types that serialize.
// ===========================================================================

#[test]
fn provenance_carries_internal_sources_for_the_dto_to_omit() {
    let p = valid_provenance(Confidence::Documented);
    assert_eq!(p.confidence, Confidence::Documented);
    assert!(
        !p.sources.is_empty(),
        "a documented record carries source notes (kept internal-only by the DTO)"
    );
}
