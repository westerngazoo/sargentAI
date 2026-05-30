//! User profile domain: the `Profile` aggregate and its validated value types.
//! Pure — no DB, no HTTP. Same parse-don't-validate style as `user`.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::UserId;

/// Biological sex. Optional on a profile; drives sex-specific ML priors.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sex {
    Male,
    Female,
}

impl Sex {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Sex::Male => "male",
            Sex::Female => "female",
        }
    }

    /// Parse the canonical SQL string (inverse of [`Sex::as_str`]).
    ///
    /// # Errors
    /// [`ProfileError::SexUnknown`] for any value outside the controlled set.
    pub fn parse(raw: &str) -> Result<Self, ProfileError> {
        match raw {
            "male" => Ok(Sex::Male),
            "female" => Ok(Sex::Female),
            _ => Err(ProfileError::SexUnknown),
        }
    }
}

/// A training goal. The controlled set is closed; parsing rejects anything else.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Goal {
    LoseFat,
    BuildMuscle,
    Recomp,
    Maintain,
    GainStrength,
}

impl Goal {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Goal::LoseFat => "lose_fat",
            Goal::BuildMuscle => "build_muscle",
            Goal::Recomp => "recomp",
            Goal::Maintain => "maintain",
            Goal::GainStrength => "gain_strength",
        }
    }

    /// Parse the canonical SQL string (inverse of [`Goal::as_str`]).
    ///
    /// # Errors
    /// [`ProfileError::GoalUnknown`] for any value outside the controlled set.
    pub fn parse(raw: &str) -> Result<Self, ProfileError> {
        match raw {
            "lose_fat" => Ok(Goal::LoseFat),
            "build_muscle" => Ok(Goal::BuildMuscle),
            "recomp" => Ok(Goal::Recomp),
            "maintain" => Ok(Goal::Maintain),
            "gain_strength" => Ok(Goal::GainStrength),
            _ => Err(ProfileError::GoalUnknown),
        }
    }
}

/// Height in whole centimetres, range [50, 300].
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct HeightCm(i32);

impl HeightCm {
    pub const MIN: i32 = 50;
    pub const MAX: i32 = 300;

    /// # Errors
    /// [`ProfileError::HeightOutOfRange`] when outside `[50, 300]`.
    pub fn try_new(cm: i32) -> Result<Self, ProfileError> {
        if (Self::MIN..=Self::MAX).contains(&cm) {
            Ok(Self(cm))
        } else {
            Err(ProfileError::HeightOutOfRange)
        }
    }

    #[must_use]
    pub fn get(self) -> i32 {
        self.0
    }
}

/// Weight in kilograms (0.1 resolution), range [20.0, 500.0].
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct WeightKg(f64);

impl WeightKg {
    pub const MIN: f64 = 20.0;
    pub const MAX: f64 = 500.0;

    /// # Errors
    /// [`ProfileError::WeightOutOfRange`] when outside `[20, 500]` or not finite.
    pub fn try_new(kg: f64) -> Result<Self, ProfileError> {
        if kg.is_finite() && (Self::MIN..=Self::MAX).contains(&kg) {
            Ok(Self(kg))
        } else {
            Err(ProfileError::WeightOutOfRange)
        }
    }

    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// Body-fat percentage (0.1 resolution), range [1.0, 75.0].
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct BodyFatPercentage(f64);

impl BodyFatPercentage {
    pub const MIN: f64 = 1.0;
    pub const MAX: f64 = 75.0;

    /// # Errors
    /// [`ProfileError::BodyFatOutOfRange`] when outside `[1, 75]` or not finite.
    pub fn try_new(pct: f64) -> Result<Self, ProfileError> {
        if pct.is_finite() && (Self::MIN..=Self::MAX).contains(&pct) {
            Ok(Self(pct))
        } else {
            Err(ProfileError::BodyFatOutOfRange)
        }
    }

    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// A non-empty, duplicate-free list of goals (input order preserved).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Goals(Vec<Goal>);

impl Goals {
    /// # Errors
    /// [`ProfileError::GoalsEmpty`] if empty; [`ProfileError::GoalsDuplicate`]
    /// if any goal repeats.
    pub fn new(goals: Vec<Goal>) -> Result<Self, ProfileError> {
        if goals.is_empty() {
            return Err(ProfileError::GoalsEmpty);
        }
        let mut seen = std::collections::HashSet::new();
        for g in &goals {
            if !seen.insert(*g) {
                return Err(ProfileError::GoalsDuplicate);
            }
        }
        Ok(Self(goals))
    }

    #[must_use]
    pub fn as_slice(&self) -> &[Goal] {
        &self.0
    }
}

/// Inclusive age bounds (years). Min 13 is a conservative minor-data floor
/// pending the M8 legal review.
pub const MIN_AGE: i32 = 13;
pub const MAX_AGE: i32 = 120;

/// Whole years from `dob` to `today`.
#[must_use]
pub fn age_on(dob: NaiveDate, today: NaiveDate) -> i32 {
    let mut age = today.year() - dob.year();
    if (today.month(), today.day()) < (dob.month(), dob.day()) {
        age -= 1;
    }
    age
}

/// Validated, writable profile fields. No identity (the token's) and no
/// timestamps (the DB's) — only what the client supplies, proven valid.
#[derive(Clone, Debug, PartialEq)]
pub struct NewProfile {
    pub date_of_birth: NaiveDate,
    pub height_cm: HeightCm,
    pub weight_kg: WeightKg,
    pub sex: Option<Sex>,
    pub body_fat_percentage: Option<BodyFatPercentage>,
    pub goals: Goals,
}

impl NewProfile {
    /// Validate raw inputs. `today` is injected for deterministic age checks.
    ///
    /// # Errors
    /// The first [`ProfileError`] encountered (field-named via
    /// [`ProfileError::field`]).
    pub fn new(
        date_of_birth: NaiveDate,
        height_cm: i32,
        weight_kg: f64,
        goals: Vec<Goal>,
        sex: Option<Sex>,
        body_fat_percentage: Option<f64>,
        today: NaiveDate,
    ) -> Result<Self, ProfileError> {
        if date_of_birth > today {
            return Err(ProfileError::DateOfBirthInFuture);
        }
        let age = age_on(date_of_birth, today);
        if !(MIN_AGE..=MAX_AGE).contains(&age) {
            return Err(ProfileError::AgeOutOfRange);
        }
        Ok(Self {
            date_of_birth,
            height_cm: HeightCm::try_new(height_cm)?,
            weight_kg: WeightKg::try_new(weight_kg)?,
            sex,
            body_fat_percentage: body_fat_percentage
                .map(BodyFatPercentage::try_new)
                .transpose()?,
            goals: Goals::new(goals)?,
        })
    }
}

/// The full read aggregate, reconstructed from a persisted row.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Profile {
    pub user_id: UserId,
    pub date_of_birth: NaiveDate,
    pub height_cm: HeightCm,
    pub weight_kg: WeightKg,
    pub sex: Option<Sex>,
    pub body_fat_percentage: Option<BodyFatPercentage>,
    pub goals: Goals,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Profile {
    #[must_use]
    pub fn age_on(&self, today: NaiveDate) -> i32 {
        age_on(self.date_of_birth, today)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProfileError {
    #[error("date_of_birth is in the future")]
    DateOfBirthInFuture,
    #[error("age is outside the allowed range")]
    AgeOutOfRange,
    #[error("height_cm is outside the allowed range")]
    HeightOutOfRange,
    #[error("weight_kg is outside the allowed range")]
    WeightOutOfRange,
    #[error("body_fat_percentage is outside the allowed range")]
    BodyFatOutOfRange,
    #[error("goals must not be empty")]
    GoalsEmpty,
    #[error("goals must not contain duplicates")]
    GoalsDuplicate,
    #[error("unknown goal")]
    GoalUnknown,
    #[error("unknown sex")]
    SexUnknown,
}

impl ProfileError {
    /// The request field this error concerns — drives `ApiError::Validation`.
    #[must_use]
    pub fn field(&self) -> &'static str {
        match self {
            ProfileError::DateOfBirthInFuture | ProfileError::AgeOutOfRange => "date_of_birth",
            ProfileError::HeightOutOfRange => "height_cm",
            ProfileError::WeightOutOfRange => "weight_kg",
            ProfileError::BodyFatOutOfRange => "body_fat_percentage",
            ProfileError::GoalsEmpty | ProfileError::GoalsDuplicate | ProfileError::GoalUnknown => {
                "goals"
            }
            ProfileError::SexUnknown => "sex",
        }
    }
}
