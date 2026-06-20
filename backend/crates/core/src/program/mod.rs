//! Program + diet instantiation from an archetype prior (R-0014, SPEC-0014).
// Step-3 (TDD red): helper functions are intentionally unused until step-5.
#![allow(dead_code)]
//!
//! Pure — no DB, no HTTP, no I/O. [`instantiate`] maps an [`Archetype`] +
//! [`Profile`] pair to a [`ProgramProposal`] (concrete week-1 program and diet
//! estimates). The `today` date is injected by the caller so the function is
//! deterministic and trivially unit-tested.
//!
//! ## Prior-only guardrail (R-0014 AC11)
//!
//! This module reads the archetype library and the user profile — it writes
//! nothing back to either. There is no path from the generated output into the
//! M5 response model's training data; the statistical learning loop (R-0015–
//! R-0017) consumes *user logs*, not these derived week-1 proposals.

#[cfg(test)]
mod tests;

use chrono::NaiveDate;
use serde::Serialize;

use crate::archetype::{Archetype, MacroEmphasis, VolumeBand};
use crate::profile::Profile;

// ---------------------------------------------------------------------------
// Output types (serializable — these cross the wire in the api crate)
// ---------------------------------------------------------------------------

/// The concrete training programme derived from an archetype's
/// [`ProgramTemplate`](crate::archetype::ProgramTemplate) and the user's
/// profile. All fields serialise; `internal_name` and `sources` are not
/// present (R-0014 AC11 / R-0012 AC4).
///
/// Step-5 note: `Deserialize` requires `VolumeBand: Deserialize`, so step-5
/// must add `#[derive(Deserialize)]` to `core::archetype::VolumeBand` and
/// `MacroEmphasis` before deriving it here.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GeneratedProgram {
    /// The split description copied from [`ProgramTemplate::split`].
    pub split: String,
    /// Number of training days per week, derived from the split keyword
    /// (SPEC-0014 §2.2.1).
    pub days_per_week: u8,
    /// Times each muscle is trained per week, copied from the template.
    pub weekly_frequency_per_muscle: u8,
    /// Volume band, copied from the template.
    pub volume: VolumeBand,
    /// Intensity guidance, copied from the template.
    pub intensity_guidance: String,
    /// Rest guidance, copied from the template.
    pub rest_guidance: String,
    /// Progression guidance, copied from the template.
    pub progression_guidance: String,
    /// Estimated session duration in minutes, derived from [`VolumeBand`]
    /// (SPEC-0014 §2.2.1).
    pub estimated_session_duration_min: u16,
    /// Representative exercises for the split — mnemonic highlights for the
    /// Flutter card (SPEC-0014 §2.2.1).
    pub highlight_exercises: Vec<String>,
}

/// The concrete diet plan derived from an archetype's
/// [`DietTemplate`](crate::archetype::DietTemplate) and the user's profile.
/// All numeric macro values are derived from Mifflin-St Jeor TDEE with a
/// goal-based multiplier (SPEC-0014 §2.2.2).
///
/// Step-5 note: same `Deserialize` dependency as `GeneratedProgram` —
/// `MacroEmphasis` must derive `Deserialize` first.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GeneratedDiet {
    /// Diet approach description, copied from the template.
    pub approach: String,
    /// Calorie strategy description, copied from the template.
    pub calorie_strategy: String,
    /// Macro emphasis, copied from the template.
    pub macro_emphasis: MacroEmphasis,
    /// Meal structure description, copied from the template.
    pub meal_structure: String,
    /// Estimated daily kilocalorie target, recomputed from rounded macros so
    /// the displayed kcal is consistent with the macro gram values.
    pub estimated_kcal: u32,
    /// Daily protein target in grams.
    pub protein_g: u32,
    /// Daily carbohydrate target in grams.
    pub carbs_g: u32,
    /// Daily fat target in grams.
    pub fat_g: u32,
}

/// A single archetype proposal — the per-card wire shape returned by
/// `GET /photo-sessions/:id/program-proposals` (SPEC-0014 §2.2).
///
/// Carries the user-facing archetype identifiers (slug, display name, summary)
/// plus the R-0013 distance/score and the two derived plans.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProgramProposal {
    /// The archetype's stable kebab slug. Never `internal_name` (R-0012 AC4).
    pub archetype_id: String,
    /// User-facing display name.
    pub display_name: String,
    /// User-facing one-line summary.
    pub summary: String,
    /// R-0013 match score ∈ [0, 1] (1 − distance).
    pub score: f64,
    /// R-0013 frame distance ∈ [0, 1].
    pub distance: f64,
    /// The generated training programme.
    pub program: GeneratedProgram,
    /// The generated diet plan.
    pub diet: GeneratedDiet,
}

// ---------------------------------------------------------------------------
// Instantiation entry point
// ---------------------------------------------------------------------------

/// Instantiate a [`ProgramProposal`] from a curated [`Archetype`] and the
/// user's [`Profile`].
///
/// Pure — no I/O. `score` and `distance` are supplied by the R-0013 ranking
/// call-site. `today` is injected for deterministic age calculation (never
/// `Utc::now()` inside this function).
///
/// This is the single derivation function for both the proposals endpoint and
/// the choose endpoint (SPEC-0014 §2.4.1). Calling it twice with the same
/// inputs always produces the same output.
#[must_use]
pub fn instantiate(
    _archetype: &Archetype,
    _profile: &Profile,
    _score: f64,
    _distance: f64,
    _today: NaiveDate,
) -> ProgramProposal {
    todo!("R-0014 step-5: implement program instantiation from archetype + profile")
}

// ---------------------------------------------------------------------------
// Internal derivation helpers (stubs — `todo!()` bodies)
// ---------------------------------------------------------------------------

/// Derive `days_per_week` from the split string via keyword matching
/// (SPEC-0014 §2.2.1 table).
fn days_per_week_from_split(_split: &str) -> u8 {
    todo!("R-0014 step-5: derive days_per_week from split keyword")
}

/// Derive `estimated_session_duration_min` from a [`VolumeBand`]
/// (SPEC-0014 §2.2.1 table).
fn duration_from_volume(_volume: VolumeBand) -> u16 {
    todo!("R-0014 step-5: derive session duration from VolumeBand")
}

/// Return the static highlight exercise list for a split category
/// (SPEC-0014 §2.2.1 table).
fn highlight_exercises_for_split(_split: &str) -> Vec<String> {
    todo!("R-0014 step-5: return highlight exercises for split category")
}

/// Compute the Mifflin-St Jeor TDEE and apply the goal multiplier to obtain
/// the daily kilocalorie target (SPEC-0014 §2.2.2).
fn kcal_target(_profile: &Profile, _today: NaiveDate) -> f64 {
    todo!("R-0014 step-5: compute kcal target via Mifflin-St Jeor + goal multiplier")
}

/// Derive protein grams from body weight and [`MacroEmphasis`]
/// (SPEC-0014 §2.2.2 table).
fn protein_g(_weight_kg: f64, _emphasis: MacroEmphasis) -> u32 {
    todo!("R-0014 step-5: derive protein_g from weight and macro emphasis")
}

/// Derive fat grams from total kcal and [`MacroEmphasis`]
/// (SPEC-0014 §2.2.2 table).
fn fat_g(_kcal: f64, _emphasis: MacroEmphasis) -> u32 {
    todo!("R-0014 step-5: derive fat_g from kcal and macro emphasis")
}

/// Derive carbohydrate grams as the kcal remainder after protein and fat,
/// floored at 0 (SPEC-0014 §2.2.2). Never negative.
fn carbs_g(_kcal: f64, _protein_g: u32, _fat_g: u32) -> u32 {
    todo!("R-0014 step-5: derive carbs_g as non-negative kcal remainder")
}

/// Recompute `estimated_kcal` from the rounded macro gram values so the
/// displayed kcal is consistent with protein/carbs/fat (SPEC-0014 §2.2.2).
fn estimated_kcal_from_macros(_protein_g: u32, _carbs_g: u32, _fat_g: u32) -> u32 {
    todo!("R-0014 step-5: recompute kcal from rounded macro grams")
}
