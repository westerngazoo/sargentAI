//! SPEC-0066 §2.3 — the exercise model library: every modelled exercise as
//! one [`ExerciseModel`], the one list of them, and the generic validation of
//! a request against an exercise's [`Description`].
//!
//! Each model describes itself (its controls, equipment and joints, so the app
//! builds R-0067's screen without a list compiled into it) and computes its
//! range of postures on the garust kernel. Slice 66-b1 serves the back squat;
//! the curl, pushdown, pulldown and guided machines join in 66-c to 66-e.

mod back_squat;
mod description;
mod input;
mod plan;
mod range;
#[cfg(test)]
mod tests;

pub use description::{Description, Equipment, Group, Ordered, Parameter, ParameterKind};
pub use input::{validate, Invalid, Value, Values};
pub use plan::model_for;
pub use range::{End, ExerciseRange, Frame, Reason};

use crate::biomech::{Body, FrameCount, KernelError};

/// One exercise: what can be changed, and its range of postures.
pub trait ExerciseModel: Sync {
    /// The controls, the equipment, the joints reported and the stable id.
    fn describe(&self) -> Description;

    /// The range of postures for `body` under `values`, sampled at `frames`.
    /// A posture the convention cannot reach is an [`End`], not an error.
    ///
    /// # Errors
    ///
    /// [`LibraryError::Invalid`] when `values` were not validated against this
    /// model's description (a programming error); [`LibraryError::Kernel`]
    /// when the kernel's cross-check fails (a modelling bug, never served).
    fn range(
        &self,
        body: &Body,
        values: &Values,
        frames: FrameCount,
    ) -> Result<ExerciseRange, LibraryError>;
}

/// Why a model gave no range at all.
#[derive(Clone, Copy, Debug, PartialEq, thiserror::Error)]
pub enum LibraryError {
    /// A value the model needs is missing or not what its description says.
    #[error("invalid field `{}`", .0.field)]
    Invalid(Invalid),
    /// Two derivations of a moment disagreed.
    #[error(transparent)]
    Kernel(#[from] KernelError),
}

impl From<Invalid> for LibraryError {
    fn from(invalid: Invalid) -> Self {
        Self::Invalid(invalid)
    }
}

/// Every modelled exercise. The only list: the plan and the endpoints read it.
static LIBRARY: [&dyn ExerciseModel; 1] = [&back_squat::BackSquat];

/// Every modelled exercise, in a stable order.
#[must_use]
pub fn library() -> &'static [&'static dyn ExerciseModel] {
    &LIBRARY
}

/// The model whose id is `id`, if the library has it.
#[must_use]
pub fn find(id: &str) -> Option<&'static dyn ExerciseModel> {
    LIBRARY.iter().copied().find(|m| m.describe().id == id)
}
