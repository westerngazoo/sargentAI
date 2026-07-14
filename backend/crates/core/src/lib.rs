//! fitAI domain types. Pure: no DB, no HTTP, no I/O.
//!
//! Persistence and presentation live in the `fitai-api` crate.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod aggregate;
pub mod archetype;
pub mod matching;
pub mod nutrition;
pub mod photo;
pub mod pose;
pub mod profile;
pub mod program;
pub mod user;
pub mod workout;

pub use aggregate::{
    summarize, Adherence, BodyPoint, BodyTrend, LiftSummary, MuscleVolume, TrainingSummary,
    TrendPoint,
};
pub use archetype::{
    Archetype, ArchetypeError, Confidence, DietTemplate, FrameProfile, HeightBand, LengthBand,
    MacroEmphasis, ProgramTemplate, Provenance, Somatotype, StructureTag, TrainingPhilosophy,
    VolumeBand, WidthBand,
};
pub use matching::{rank, RankedMatch};
pub use nutrition::{Grams, Macros, NewNutritionLog, NutritionError, NutritionLog};
pub use photo::{
    Angle, ImageContentType, NewPhoto, PhotoError, PhotoSession, SessionPhoto, MAX_BYTES,
};
pub use pose::{
    derive_frame_features, FrameError, FrameFeatures, Keypoint, Landmark, PoseKeypoints,
};
pub use profile::{
    BodyFatPercentage, Goal, Goals, HeightCm, NewProfile, Profile, ProfileError, Sex, WeightKg,
};
pub use program::{instantiate, GeneratedDiet, GeneratedProgram, ProgramProposal};
pub use user::{Email, EmailParseError, User, UserId};
pub use workout::{
    ExerciseName, LoadKg, MuscleGroup, NewExercise, NewSet, NewWorkoutSession, Reps, Rpe,
    WorkoutError, WorkoutExercise, WorkoutSession, WorkoutSet,
};
