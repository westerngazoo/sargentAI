//! fitAI domain types. Pure: no DB, no HTTP, no I/O.
//!
//! Persistence and presentation live in the `fitai-api` crate.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod adjust;
pub mod aggregate;
pub mod archetype;
pub mod authoring;
pub mod goals;
pub mod matching;
pub mod nutrition;
pub mod periodize;
pub mod photo;
pub mod pose;
pub mod profile;
pub mod program;
pub mod technique;
pub mod user;
pub mod workout;

pub use adjust::{suggest, Adjustment, Change, Severity};
pub use aggregate::{
    current_e1rm, summarize, Adherence, BodyPoint, BodyTrend, LiftSummary, MuscleVolume,
    TrainingSummary, TrendPoint, DEFAULT_WINDOW_WEEKS,
};
pub use archetype::{
    Archetype, ArchetypeError, Confidence, DietTemplate, FrameProfile, HeightBand, LengthBand,
    MacroEmphasis, ProgramTemplate, Provenance, Somatotype, StructureTag, TrainingPhilosophy,
    VolumeBand, WidthBand,
};
// `authoring::validate` is deliberately NOT re-exported here: a bare
// `fitai_core::validate` says nothing about *what* it validates. Callers reach
// it as `fitai_core::authoring::validate`, where the module name carries the
// meaning (SPEC-0041 §2.4).
//
// `goals` is likewise NOT re-exported at the root: `goals::Goal` would collide
// with the R-0003 `profile::Goal` (the coarse training *direction*), and
// `goals::validate` has the same anonymity problem as `authoring::validate`.
// Callers reach everything as `fitai_core::goals::…` (SPEC-0042 §2.1).
pub use authoring::{
    materialize, AuthorError, AuthoredExercise, AuthoredProgram, ClassPrescription, IntensityClass,
    MaterializedCycle, MaterializedDay, MaterializedEntry, Schedule, ScheduleEntry, WorkSetLine,
};
pub use matching::{rank, RankedMatch};
pub use nutrition::{Grams, Macros, NewNutritionLog, NutritionError, NutritionLog};
pub use periodize::{
    block, linear, undulating, Block, BlockParams, DayProfile, E1rmMap, LinearParams,
    PeriodizationScheme, PeriodizedProgram, PlanError, PlanParams, PrescribedExercise,
    PrescribedSet, TrainingSession, TrainingWeek, UndulatingParams,
};
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
// `technique` is NOT re-exported at the crate root: `fitai_core::analyze` says
// nothing about *what* it analyses, and `Side`/`Unit`/`Metric` are generic
// enough to collide. Callers reach it as `fitai_core::technique::…`, where the
// module name carries the meaning (the `authoring`/`goals` precedent).
pub use user::{Email, EmailParseError, User, UserId};
pub use workout::{
    ExerciseName, LoadKg, MuscleGroup, NewExercise, NewSet, NewWorkoutSession, Reps, Rpe,
    WorkoutError, WorkoutExercise, WorkoutSession, WorkoutSet,
};
