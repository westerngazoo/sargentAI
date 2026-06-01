//! Nutrition-log domain: the `NutritionLog` aggregate, its `Macros` value group,
//! and the calorie derivation. Pure — no DB, no HTTP. Parse-don't-validate, as
//! `profile`/`user`/`workout`.

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use crate::UserId;

/// A macronutrient mass in grams, range `[0, 2000]`. `0` is valid (e.g. a
/// zero-fat day); negatives, `> 2000`, and non-finite values are rejected.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Grams(f64);

impl Grams {
    pub const MAX: f64 = 2000.0;

    /// Validate a macro mass, tagging failures with `field` so the caller can
    /// report which macro was out of range (three fields share this newtype).
    ///
    /// # Errors
    /// [`NutritionError::MacroOutOfRange`] when not finite, `< 0`, or `> 2000`.
    pub fn try_new(value: f64, field: &'static str) -> Result<Self, NutritionError> {
        if value.is_finite() && (0.0..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(NutritionError::MacroOutOfRange { field })
        }
    }

    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// The three macronutrients. The calorie derivation lives here as the single
/// authority (AC9): `4·protein + 4·carbs + 9·fat`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Macros {
    pub protein: Grams,
    pub carbs: Grams,
    pub fat: Grams,
}

impl Macros {
    const KCAL_PER_G_PROTEIN: f64 = 4.0;
    const KCAL_PER_G_CARB: f64 = 4.0;
    const KCAL_PER_G_FAT: f64 = 9.0;

    /// Validate the three macros, attributing a range failure to its field.
    ///
    /// # Errors
    /// The first [`NutritionError::MacroOutOfRange`] among protein/carbs/fat.
    pub fn new(protein_g: f64, carbs_g: f64, fat_g: f64) -> Result<Self, NutritionError> {
        Ok(Self {
            protein: Grams::try_new(protein_g, "protein_g")?,
            carbs: Grams::try_new(carbs_g, "carbs_g")?,
            fat: Grams::try_new(fat_g, "fat_g")?,
        })
    }

    /// Total energy in kilocalories (the AC9 single authority).
    #[must_use]
    pub fn calories(&self) -> f64 {
        Self::KCAL_PER_G_PROTEIN * self.protein.get()
            + Self::KCAL_PER_G_CARB * self.carbs.get()
            + Self::KCAL_PER_G_FAT * self.fat.get()
    }
}

/// A validated nutrition log (no identity/timestamps).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NewNutritionLog {
    pub performed_on: NaiveDate,
    pub macros: Macros,
}

impl NewNutritionLog {
    /// `today` is injected for a deterministic future-date check, which runs
    /// before macro validation so a future date reports `"performed_on"` even
    /// when a macro is also invalid.
    ///
    /// # Errors
    /// [`NutritionError::PerformedOnInFuture`] or the first macro range error.
    pub fn new(
        performed_on: NaiveDate,
        protein_g: f64,
        carbs_g: f64,
        fat_g: f64,
        today: NaiveDate,
    ) -> Result<Self, NutritionError> {
        if performed_on > today {
            return Err(NutritionError::PerformedOnInFuture);
        }
        Ok(Self {
            performed_on,
            macros: Macros::new(protein_g, carbs_g, fat_g)?,
        })
    }
}

/// A stored nutrition log, reconstructed from a row. Intentionally not
/// `Serialize`: the wire shape carries a derived `calories` the aggregate does
/// not store, so the HTTP `NutritionResponse` DTO owns serialization (the
/// R-0003 `ProfileResponse`/`age` precedent, SPEC-0005 §2.4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NutritionLog {
    pub id: Uuid,
    pub user_id: UserId,
    pub performed_on: NaiveDate,
    pub macros: Macros,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NutritionLog {
    /// Derived total energy (delegates to the single `Macros` authority).
    #[must_use]
    pub fn calories(&self) -> f64 {
        self.macros.calories()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum NutritionError {
    #[error("performed_on is in the future")]
    PerformedOnInFuture,
    #[error("macro `{field}` is outside the allowed range")]
    MacroOutOfRange { field: &'static str },
}

impl NutritionError {
    /// The request field this error concerns — drives `ApiError::Validation`.
    #[must_use]
    pub fn field(&self) -> &'static str {
        match self {
            NutritionError::PerformedOnInFuture => "performed_on",
            NutritionError::MacroOutOfRange { field } => field,
        }
    }
}
