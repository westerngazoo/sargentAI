//! Workout-log domain: the `WorkoutSession` aggregate and its value types.
//!
//! Pure — no DB, no HTTP. Parse-don't-validate, exactly as `profile`/`user`:
//! the write models (`NewSet`/`NewExercise`/`NewWorkoutSession`) are the single
//! validation authority, built bottom-up through `::new`/`::try_new`
//! constructors that return [`WorkoutError`]; the read aggregates
//! (`WorkoutSet`/`WorkoutExercise`/`WorkoutSession`) are reconstructed from
//! persisted rows and serialize directly to the wire (SPEC-0004 §2.4).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::UserId;

/// Coarse muscle grouping. Closed set; the single authority for the controlled
/// vocabulary (AC9) — the only place the six canonical strings live, shared by
/// serde (JSON) and [`MuscleGroup::as_str`]/[`MuscleGroup::parse`] (SQL).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MuscleGroup {
    Chest,
    Back,
    Shoulders,
    Arms,
    Legs,
    Core,
}

impl MuscleGroup {
    /// The canonical SQL/JSON string for this variant.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            MuscleGroup::Chest => "chest",
            MuscleGroup::Back => "back",
            MuscleGroup::Shoulders => "shoulders",
            MuscleGroup::Arms => "arms",
            MuscleGroup::Legs => "legs",
            MuscleGroup::Core => "core",
        }
    }

    /// Parse the canonical SQL string (inverse of [`MuscleGroup::as_str`]).
    ///
    /// # Errors
    /// [`WorkoutError::MuscleGroupUnknown`] for anything outside the set.
    pub fn parse(raw: &str) -> Result<Self, WorkoutError> {
        match raw {
            "chest" => Ok(Self::Chest),
            "back" => Ok(Self::Back),
            "shoulders" => Ok(Self::Shoulders),
            "arms" => Ok(Self::Arms),
            "legs" => Ok(Self::Legs),
            "core" => Ok(Self::Core),
            _ => Err(WorkoutError::MuscleGroupUnknown),
        }
    }
}

/// Repetitions in a set, range `[1, 10000]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Reps(i32);

impl Reps {
    /// Smallest valid rep count.
    pub const MIN: i32 = 1;
    /// Largest valid rep count.
    pub const MAX: i32 = 10_000;

    /// Construct a validated rep count.
    ///
    /// # Errors
    /// [`WorkoutError::RepsOutOfRange`] when outside `[1, 10000]`.
    pub fn try_new(reps: i32) -> Result<Self, WorkoutError> {
        if (Self::MIN..=Self::MAX).contains(&reps) {
            Ok(Self(reps))
        } else {
            Err(WorkoutError::RepsOutOfRange)
        }
    }

    /// The underlying rep count.
    #[must_use]
    pub fn get(self) -> i32 {
        self.0
    }
}

/// Lifted load in kilograms, range `(0, 1000]`. Distinct from the profile
/// `WeightKg` (different semantics/range — a 2.5 kg dumbbell is valid load but
/// not a valid body weight).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct LoadKg(f64);

impl LoadKg {
    /// Largest valid load.
    pub const MAX: f64 = 1000.0;

    /// Construct a validated load.
    ///
    /// # Errors
    /// [`WorkoutError::WeightOutOfRange`] when not finite, `<= 0`, or `> 1000`.
    pub fn try_new(kg: f64) -> Result<Self, WorkoutError> {
        if kg.is_finite() && kg > 0.0 && kg <= Self::MAX {
            Ok(Self(kg))
        } else {
            Err(WorkoutError::WeightOutOfRange)
        }
    }

    /// The underlying load in kilograms.
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// Rate of perceived exertion: `[6.0, 10.0]` in `0.5` steps.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Rpe(f64);

impl Rpe {
    /// Smallest valid RPE.
    pub const MIN: f64 = 6.0;
    /// Largest valid RPE.
    pub const MAX: f64 = 10.0;

    /// Construct a validated RPE.
    ///
    /// # Errors
    /// [`WorkoutError::RpeInvalid`] when not finite, outside `[6, 10]`, or not a
    /// multiple of `0.5`.
    pub fn try_new(rpe: f64) -> Result<Self, WorkoutError> {
        // Every value on the 0.5 grid in [6, 10] is exactly representable in
        // f64 and stays exact under `* 2.0`, so this fract check is precise.
        let half_step = (rpe * 2.0).fract() == 0.0;
        if rpe.is_finite() && (Self::MIN..=Self::MAX).contains(&rpe) && half_step {
            Ok(Self(rpe))
        } else {
            Err(WorkoutError::RpeInvalid)
        }
    }

    /// The underlying RPE.
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// A non-blank exercise name, trimmed, `<= 100` characters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ExerciseName(String);

impl ExerciseName {
    /// Maximum length in characters (measured after trimming).
    pub const MAX_CHARS: usize = 100;

    /// Construct a validated, trimmed exercise name.
    ///
    /// # Errors
    /// [`WorkoutError::NameBlank`] if empty after trimming;
    /// [`WorkoutError::NameTooLong`] if over 100 characters.
    pub fn try_new(raw: &str) -> Result<Self, WorkoutError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(WorkoutError::NameBlank);
        }
        if trimmed.chars().count() > Self::MAX_CHARS {
            return Err(WorkoutError::NameTooLong);
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// The trimmed name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validation failure in the workout-log write model. `field()` names the
/// offending request field, driving `ApiError::Validation { field }`.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkoutError {
    #[error("performed_on is in the future")]
    PerformedOnInFuture,
    #[error("a session must have at least one exercise")]
    ExercisesEmpty,
    #[error("an exercise must have at least one set")]
    SetsEmpty,
    #[error("exercise name is blank")]
    NameBlank,
    #[error("exercise name is too long")]
    NameTooLong,
    #[error("reps is outside the allowed range")]
    RepsOutOfRange,
    #[error("weight_kg is outside the allowed range")]
    WeightOutOfRange,
    #[error("rpe is invalid")]
    RpeInvalid,
    #[error("unknown muscle group")]
    MuscleGroupUnknown,
}

impl WorkoutError {
    /// The request field this error concerns — drives `ApiError::Validation`.
    #[must_use]
    pub fn field(&self) -> &'static str {
        match self {
            WorkoutError::PerformedOnInFuture => "performed_on",
            WorkoutError::ExercisesEmpty => "exercises",
            WorkoutError::SetsEmpty => "sets",
            WorkoutError::NameBlank | WorkoutError::NameTooLong => "name",
            WorkoutError::RepsOutOfRange => "reps",
            WorkoutError::WeightOutOfRange => "weight_kg",
            WorkoutError::RpeInvalid => "rpe",
            WorkoutError::MuscleGroupUnknown => "muscle_group",
        }
    }
}

/// A validated set (no identity).
#[derive(Clone, Debug, PartialEq)]
pub struct NewSet {
    pub reps: Reps,
    pub weight_kg: Option<LoadKg>,
    pub rpe: Option<Rpe>,
}

impl NewSet {
    /// Build a validated set from raw scalars.
    ///
    /// # Errors
    /// The first [`WorkoutError`] among reps / weight / rpe validation.
    pub fn new(reps: i32, weight_kg: Option<f64>, rpe: Option<f64>) -> Result<Self, WorkoutError> {
        Ok(Self {
            reps: Reps::try_new(reps)?,
            weight_kg: weight_kg.map(LoadKg::try_new).transpose()?,
            rpe: rpe.map(Rpe::try_new).transpose()?,
        })
    }
}

/// A validated exercise with at least one set (no identity).
#[derive(Clone, Debug, PartialEq)]
pub struct NewExercise {
    pub name: ExerciseName,
    pub muscle_group: Option<MuscleGroup>,
    pub sets: Vec<NewSet>,
}

impl NewExercise {
    /// Build a validated exercise.
    ///
    /// # Errors
    /// [`WorkoutError::SetsEmpty`] if no sets, or the first name/set error.
    pub fn new(
        name: &str,
        muscle_group: Option<MuscleGroup>,
        sets: Vec<NewSet>,
    ) -> Result<Self, WorkoutError> {
        if sets.is_empty() {
            return Err(WorkoutError::SetsEmpty);
        }
        Ok(Self {
            name: ExerciseName::try_new(name)?,
            muscle_group,
            sets,
        })
    }
}

/// A validated session with at least one exercise (no identity/timestamps).
#[derive(Clone, Debug, PartialEq)]
pub struct NewWorkoutSession {
    pub performed_on: NaiveDate,
    pub exercises: Vec<NewExercise>,
}

impl NewWorkoutSession {
    /// Build a validated session. `today` is injected for a deterministic
    /// future-date check.
    ///
    /// # Errors
    /// [`WorkoutError::PerformedOnInFuture`], [`WorkoutError::ExercisesEmpty`],
    /// or the first nested exercise/set error.
    pub fn new(
        performed_on: NaiveDate,
        exercises: Vec<NewExercise>,
        today: NaiveDate,
    ) -> Result<Self, WorkoutError> {
        if performed_on > today {
            return Err(WorkoutError::PerformedOnInFuture);
        }
        if exercises.is_empty() {
            return Err(WorkoutError::ExercisesEmpty);
        }
        Ok(Self {
            performed_on,
            exercises,
        })
    }
}

/// A persisted set, reconstructed from a row. Serializes to the AC7 wire shape.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WorkoutSet {
    pub id: Uuid,
    pub position: i32,
    pub reps: Reps,
    pub weight_kg: Option<LoadKg>,
    pub rpe: Option<Rpe>,
}

/// A persisted exercise with its sets. Serializes to the AC7 wire shape.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WorkoutExercise {
    pub id: Uuid,
    pub position: i32,
    pub name: ExerciseName,
    pub muscle_group: Option<MuscleGroup>,
    pub sets: Vec<WorkoutSet>,
}

/// A persisted session aggregate. Serializes directly to the wire (SPEC-0004
/// §2.4 / OQ-C1): no derived field, field names match AC7 one-for-one.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WorkoutSession {
    pub id: Uuid,
    pub user_id: UserId,
    pub performed_on: NaiveDate,
    pub exercises: Vec<WorkoutExercise>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
