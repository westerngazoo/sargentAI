//! Program + diet instantiation from an archetype prior (R-0014, SPEC-0014).
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
use serde::{Deserialize, Serialize};

use crate::archetype::{Archetype, MacroEmphasis, VolumeBand};
use crate::profile::{Goal, Profile, Sex};

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// The concrete training programme derived from an archetype's
/// [`ProgramTemplate`](crate::archetype::ProgramTemplate) and the user's
/// profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeneratedProgram {
    pub split: String,
    pub days_per_week: u8,
    pub weekly_frequency_per_muscle: u8,
    pub volume: VolumeBand,
    pub intensity_guidance: String,
    pub rest_guidance: String,
    pub progression_guidance: String,
    pub estimated_session_duration_min: u16,
    pub highlight_exercises: Vec<String>,
}

/// The concrete diet plan derived from an archetype's
/// [`DietTemplate`](crate::archetype::DietTemplate) and the user's profile.
/// `estimated_kcal` is recomputed from the rounded macro grams so the
/// displayed value is always consistent with `protein_g + carbs_g + fat_g`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeneratedDiet {
    pub approach: String,
    pub calorie_strategy: String,
    pub macro_emphasis: MacroEmphasis,
    pub meal_structure: String,
    pub estimated_kcal: u32,
    pub protein_g: u32,
    pub carbs_g: u32,
    pub fat_g: u32,
}

/// A single archetype proposal — the per-card wire shape (SPEC-0014 §2.2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgramProposal {
    /// Archetype slug — never `internal_name` (R-0012 AC4).
    pub archetype_id: String,
    pub display_name: String,
    pub summary: String,
    pub score: f64,
    pub distance: f64,
    pub program: GeneratedProgram,
    pub diet: GeneratedDiet,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Instantiate a [`ProgramProposal`] from a curated [`Archetype`] and the
/// user's [`Profile`].
///
/// Pure — no I/O. `score` and `distance` come from the R-0013 ranking
/// call-site. `today` is injected for deterministic age calculation.
#[must_use]
pub fn instantiate(
    archetype: &Archetype,
    profile: &Profile,
    score: f64,
    distance: f64,
    today: NaiveDate,
) -> ProgramProposal {
    let t = &archetype.program_template;
    let d = &archetype.diet_template;

    let weight = profile.weight_kg.get();
    let kcal = kcal_target(profile, today);
    let p_g = protein_g(weight, d.macro_emphasis);
    let f_g = fat_g(kcal, d.macro_emphasis);
    let c_g = carbs_g(kcal, p_g, f_g);

    ProgramProposal {
        archetype_id: archetype.id.to_string(),
        display_name: archetype.display_name.clone(),
        summary: archetype.summary.clone(),
        score,
        distance,
        program: GeneratedProgram {
            split: t.split.clone(),
            days_per_week: days_per_week_from_split(&t.split),
            weekly_frequency_per_muscle: t.weekly_frequency_per_muscle,
            volume: t.volume,
            intensity_guidance: t.intensity.clone(),
            rest_guidance: t.rest.clone(),
            progression_guidance: t.progression.clone(),
            estimated_session_duration_min: duration_from_volume(t.volume),
            highlight_exercises: highlight_exercises_for_split(&t.split),
        },
        diet: GeneratedDiet {
            approach: d.approach.clone(),
            calorie_strategy: d.calorie_strategy.clone(),
            macro_emphasis: d.macro_emphasis,
            meal_structure: d.meal_structure.clone(),
            estimated_kcal: estimated_kcal_from_macros(p_g, c_g, f_g),
            protein_g: p_g,
            carbs_g: c_g,
            fat_g: f_g,
        },
    }
}

// ---------------------------------------------------------------------------
// Program derivation helpers (SPEC-0014 §2.2.1)
// ---------------------------------------------------------------------------

fn days_per_week_from_split(split: &str) -> u8 {
    let s = split.to_lowercase();
    if s.contains("ppl") || s.contains("push/pull") {
        return 6;
    }
    if s.contains("upper") && s.contains("lower") {
        return 4;
    }
    if s.contains("full body") || s.contains("whole body") {
        return 3;
    }
    4
}

fn duration_from_volume(volume: VolumeBand) -> u16 {
    match volume {
        VolumeBand::Low => 45,
        VolumeBand::Moderate => 60,
        VolumeBand::High => 75,
    }
}

fn highlight_exercises_for_split(split: &str) -> Vec<String> {
    let s = split.to_lowercase();
    let list: &[&str] = if s.contains("ppl") || s.contains("push/pull") {
        &[
            "Bench Press",
            "Overhead Press",
            "Squat",
            "Barbell Row",
            "Deadlift",
            "Pull-up",
        ]
    } else if s.contains("upper") && s.contains("lower") {
        &[
            "Barbell Squat",
            "Bench Press",
            "Barbell Row",
            "Overhead Press",
            "Romanian Deadlift",
            "Pull-up",
        ]
    } else {
        &["Barbell Squat", "Bench Press", "Deadlift", "Barbell Row"]
    };
    list.iter().map(|e| (*e).to_string()).collect()
}

// ---------------------------------------------------------------------------
// Diet derivation helpers (SPEC-0014 §2.2.2)
// ---------------------------------------------------------------------------

/// Mifflin-St Jeor TDEE with goal multiplier (SPEC-0014 §2.2.2).
fn kcal_target(profile: &Profile, today: NaiveDate) -> f64 {
    let w = profile.weight_kg.get();
    let h = f64::from(profile.height_cm.get());
    let a = f64::from(profile.age_on(today));
    let sex_offset = match profile.sex {
        Some(Sex::Male) => 5.0,
        Some(Sex::Female) => -161.0,
        None => 0.0,
    };
    let bmr = 10.0_f64.mul_add(w, 6.25_f64.mul_add(h, (-5.0_f64).mul_add(a, sex_offset)));
    let tdee = bmr * 1.55;

    match profile
        .goals
        .as_slice()
        .first()
        .copied()
        .unwrap_or(Goal::Maintain)
    {
        Goal::LoseFat => tdee * 0.80,
        Goal::BuildMuscle | Goal::GainStrength => tdee * 1.15,
        Goal::Recomp | Goal::Maintain => tdee,
    }
}

fn protein_g(weight_kg: f64, emphasis: MacroEmphasis) -> u32 {
    let multiplier = match emphasis {
        MacroEmphasis::HighProtein => 2.2,
        MacroEmphasis::Balanced => 1.8,
        MacroEmphasis::HighCarb => 1.6,
        MacroEmphasis::LowCarb => 2.0,
    };
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    {
        (weight_kg * multiplier).round() as u32
    }
}

fn fat_g(kcal: f64, emphasis: MacroEmphasis) -> u32 {
    let fat_fraction = match emphasis {
        MacroEmphasis::HighProtein => 0.25,
        MacroEmphasis::Balanced => 0.30,
        MacroEmphasis::HighCarb => 0.20,
        MacroEmphasis::LowCarb => 0.40,
    };
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    {
        (kcal * fat_fraction / 9.0).round() as u32
    }
}

fn carbs_g(kcal: f64, protein_g: u32, fat_g: u32) -> u32 {
    let remainder = kcal - f64::from(protein_g) * 4.0 - f64::from(fat_g) * 9.0;
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    {
        (remainder / 4.0).max(0.0).round() as u32
    }
}

fn estimated_kcal_from_macros(protein_g: u32, carbs_g: u32, fat_g: u32) -> u32 {
    protein_g * 4 + carbs_g * 4 + fat_g * 9
}
