//! The curated **archetype library** — the matching *prior* (R-0012, SPEC-0012).
//!
//! An [`Archetype`] models a bodybuilder/athlete archetype: a [`FrameProfile`]
//! (the body structure it suits), a [`ProgramTemplate`] (how it trains), a
//! [`DietTemplate`] (how it eats), and [`Provenance`] (how well-documented the
//! source is). The library is the **prior** R-0013 matches a user's
//! photo-derived frame against and R-0014 instantiates a starting plan from.
//!
//! Pure — no DB, no HTTP, no I/O. Parse-don't-validate, like `profile`/
//! `workout`/`nutrition`/`photo`: a record is built only through the validating
//! [`Archetype::new`] (and the value-type constructors), so a malformed record
//! cannot exist.
//!
//! ## Prior-only guardrail (AC5)
//!
//! This data is the matching **prior**. It must **never** be read as training
//! data by the M5 response model — those models consume *user logs*, not these
//! curated PED-era genetic outliers (whose *response* to training is not a model
//! for a real user's). There is no code path from this module into any training
//! input; the boundary is the guardrail.
//!
//! ## Wire privacy (AC4)
//!
//! The [`Archetype`] aggregate and its [`Provenance`] are intentionally **not**
//! `Serialize`: the user-facing wire shape is owned by the api
//! `ArchetypeResponse` DTO, so the internal research label (`internal_name`) and
//! the curation `sources` can never leak. Only the matchable value types
//! (frame/program/diet and their enums) serialize.

mod seed;

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Goal;

/// How tall an archetype's frame stands, as a coarse band (the pose pipeline
/// emits a height estimate, not a precise stature).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HeightBand {
    Short,
    Average,
    Tall,
}

/// Clavicle / shoulder-girdle width band — a key V-taper determinant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WidthBand {
    Narrow,
    Average,
    Wide,
}

/// Relative limb length band — a leverage descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LengthBand {
    Short,
    Average,
    Long,
}

/// Somatotype — the coarse build the frame reads as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Somatotype {
    Ecto,
    Meso,
    Endo,
}

/// A controlled structural-feature tag (the matchable vocabulary, AC6 — not free
/// strings). Extend the enum, not the data, to add a feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StructureTag {
    WideClavicles,
    NarrowHips,
    BlockyWaist,
    TightWaist,
    LongLimbs,
    ShortLimbs,
    DenseMuscle,
    FullMuscleBellies,
    SmallJoints,
}

/// The training philosophy an archetype embodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingPhilosophy {
    Hit,
    HighVolumeSplit,
    Powerbuilding,
    ModernHypertrophy,
}

/// Weekly working-set volume band per muscle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VolumeBand {
    Low,
    Moderate,
    High,
}

/// Which macronutrient an archetype's diet leans on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacroEmphasis {
    HighProtein,
    Balanced,
    HighCarb,
    LowCarb,
}

/// How well-documented an archetype's curated source material is (AC7). Honest
/// about documented-vs-folklore — no fabricated precision presented as fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Documented,
    Reconstructed,
    Folklore,
}

/// The matchable body-structure profile (AC6). Exposes the numeric V-taper proxy
/// R-0013 computes a distance against, plus banded/enum descriptors and a
/// controlled [`StructureTag`] vocabulary.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FrameProfile {
    /// Shoulder-to-waist ratio (the V-taper proxy), validated `1.0..=2.5`.
    pub shoulder_to_waist: f64,
    pub height_band: HeightBand,
    pub clavicle_width: WidthBand,
    pub limb_length: LengthBand,
    pub build: Somatotype,
    pub structure_tags: Vec<StructureTag>,
}

impl FrameProfile {
    /// The accepted shoulder-to-waist range (inclusive).
    pub const SHOULDER_TO_WAIST: std::ops::RangeInclusive<f64> = 1.0..=2.5;

    /// Build a validated frame profile.
    ///
    /// # Errors
    /// [`ArchetypeError::ShoulderToWaistOutOfRange`] if `shoulder_to_waist` is
    /// not finite or falls outside `1.0..=2.5`.
    pub fn new(
        shoulder_to_waist: f64,
        height_band: HeightBand,
        clavicle_width: WidthBand,
        limb_length: LengthBand,
        build: Somatotype,
        structure_tags: Vec<StructureTag>,
    ) -> Result<Self, ArchetypeError> {
        if !shoulder_to_waist.is_finite() || !Self::SHOULDER_TO_WAIST.contains(&shoulder_to_waist) {
            return Err(ArchetypeError::ShoulderToWaistOutOfRange);
        }
        Ok(Self {
            shoulder_to_waist,
            height_band,
            clavicle_width,
            limb_length,
            build,
            structure_tags,
        })
    }
}

/// How an archetype trains.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProgramTemplate {
    pub philosophy: TrainingPhilosophy,
    pub split: String,
    /// Times each muscle is trained per week, validated `1..=7`.
    pub weekly_frequency_per_muscle: u8,
    pub volume: VolumeBand,
    pub intensity: String,
    pub rest: String,
    pub progression: String,
}

impl ProgramTemplate {
    /// The accepted weekly-frequency-per-muscle range (inclusive).
    pub const WEEKLY_FREQUENCY: std::ops::RangeInclusive<u8> = 1..=7;

    /// Build a validated program template.
    ///
    /// # Errors
    /// [`ArchetypeError::WeeklyFrequencyOutOfRange`] if
    /// `weekly_frequency_per_muscle` falls outside `1..=7`.
    pub fn new(
        philosophy: TrainingPhilosophy,
        split: String,
        weekly_frequency_per_muscle: u8,
        volume: VolumeBand,
        intensity: String,
        rest: String,
        progression: String,
    ) -> Result<Self, ArchetypeError> {
        if !Self::WEEKLY_FREQUENCY.contains(&weekly_frequency_per_muscle) {
            return Err(ArchetypeError::WeeklyFrequencyOutOfRange);
        }
        Ok(Self {
            philosophy,
            split,
            weekly_frequency_per_muscle,
            volume,
            intensity,
            rest,
            progression,
        })
    }
}

/// How an archetype eats.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DietTemplate {
    pub approach: String,
    pub calorie_strategy: String,
    pub macro_emphasis: MacroEmphasis,
    pub meal_structure: String,
}

impl DietTemplate {
    /// Build a diet template. Total by construction — every field is free-form
    /// guidance, so there is nothing to reject.
    #[must_use]
    pub fn new(
        approach: String,
        calorie_strategy: String,
        macro_emphasis: MacroEmphasis,
        meal_structure: String,
    ) -> Self {
        Self {
            approach,
            calorie_strategy,
            macro_emphasis,
            meal_structure,
        }
    }
}

/// How well-documented an archetype's curated material is. `sources` is
/// **internal-only** — curation notes that never cross the wire (AC4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    pub confidence: Confidence,
    pub sources: Vec<&'static str>,
}

impl Provenance {
    /// Record a confidence level and its internal source notes.
    #[must_use]
    pub fn new(confidence: Confidence, sources: Vec<&'static str>) -> Self {
        Self {
            confidence,
            sources,
        }
    }
}

/// A curated archetype — the prior a user's frame is matched to. Built only
/// through [`Archetype::new`]; intentionally **not** `Serialize` (the api DTO
/// owns the wire shape and omits `internal_name`/`sources`, AC4).
#[derive(Clone, Debug, PartialEq)]
pub struct Archetype {
    /// Stable kebab slug — the API key (`"heavy-duty-mass"`).
    pub id: &'static str,
    /// The research label (`"Yates-96"`); **never** serialized to a user (AC4).
    pub internal_name: &'static str,
    /// Abstracted, user-facing name.
    pub display_name: String,
    /// Abstracted, user-facing one-line description.
    pub summary: String,
    pub frame_profile: FrameProfile,
    pub program_template: ProgramTemplate,
    pub diet_template: DietTemplate,
    pub provenance: Provenance,
    /// The goals this archetype serves; non-empty.
    pub goals_served: Vec<Goal>,
}

impl Archetype {
    /// Assemble a validated archetype from already-validated parts.
    ///
    /// # Errors
    /// [`ArchetypeError::InternalNameEmpty`] / [`ArchetypeError::DisplayNameEmpty`]
    /// for an empty name; [`ArchetypeError::GoalsServedEmpty`] for an empty
    /// goals list. (The frame/program ranges are enforced by their own
    /// constructors.)
    #[allow(clippy::too_many_arguments)] // an archetype is the aggregate of its
                                         // eight curated parts plus its slug; grouping them further would only hide
                                         // the record's shape from the reviewer who approves each field.
    pub fn new(
        id: &'static str,
        internal_name: &'static str,
        display_name: String,
        summary: String,
        frame_profile: FrameProfile,
        program_template: ProgramTemplate,
        diet_template: DietTemplate,
        provenance: Provenance,
        goals_served: Vec<Goal>,
    ) -> Result<Self, ArchetypeError> {
        if internal_name.is_empty() {
            return Err(ArchetypeError::InternalNameEmpty);
        }
        if display_name.is_empty() {
            return Err(ArchetypeError::DisplayNameEmpty);
        }
        if goals_served.is_empty() {
            return Err(ArchetypeError::GoalsServedEmpty);
        }
        Ok(Self {
            id,
            internal_name,
            display_name,
            summary,
            frame_profile,
            program_template,
            diet_template,
            provenance,
            goals_served,
        })
    }
}

/// A malformed-archetype error, field-named via [`ArchetypeError::field`] (the
/// `profile`/`photo`/`nutrition` error idiom).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ArchetypeError {
    #[error("shoulder_to_waist must be a finite ratio in [1.0, 2.5]")]
    ShoulderToWaistOutOfRange,
    #[error("weekly_frequency_per_muscle must be in [1, 7]")]
    WeeklyFrequencyOutOfRange,
    #[error("goals_served must not be empty")]
    GoalsServedEmpty,
    #[error("display_name must not be empty")]
    DisplayNameEmpty,
    #[error("internal_name must not be empty")]
    InternalNameEmpty,
}

impl ArchetypeError {
    /// The record field this error concerns.
    #[must_use]
    pub fn field(&self) -> &'static str {
        match self {
            ArchetypeError::ShoulderToWaistOutOfRange => "shoulder_to_waist",
            ArchetypeError::WeeklyFrequencyOutOfRange => "weekly_frequency_per_muscle",
            ArchetypeError::GoalsServedEmpty => "goals_served",
            ArchetypeError::DisplayNameEmpty => "display_name",
            ArchetypeError::InternalNameEmpty => "internal_name",
        }
    }
}

/// The curated library — the six approved archetypes, validated once at first
/// access and cached for the process lifetime.
#[must_use]
pub fn library() -> &'static [Archetype] {
    static LIB: OnceLock<Vec<Archetype>> = OnceLock::new();
    LIB.get_or_init(seed::all).as_slice()
}

/// Resolve an archetype by its slug id. The single read path into the prior;
/// an unknown id resolves to `None` (never a panic, never a wildcard match).
#[must_use]
pub fn find(id: &str) -> Option<&'static Archetype> {
    library().iter().find(|a| a.id == id)
}
