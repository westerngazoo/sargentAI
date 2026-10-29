//! SPEC-0059 §2.5 — which exercise names have a physics model in this module.

use crate::periodize::lift_key;
use crate::technique::Lift;

/// The lift whose physics model covers `exercise`, or `None` when there is
/// none yet (SPEC-0059 §2.5: the server decides, not a client string list).
///
/// Only the back squat has a model today ([`super::squat_range`]), and the
/// library names it "Back Squat". Exact after [`lift_key`] normalization:
/// a front, hack or low-bar squat is different mechanics, and the deadlift and
/// bench wait for R-0045's garust work.
///
/// Narrower than [`Lift::from_exercise_name`] on purpose: that one says which
/// lift a video shows; this one says which mechanics are modelled.
#[must_use]
pub fn model_for(exercise: &str) -> Option<Lift> {
    (lift_key(exercise) == "back squat").then_some(Lift::Squat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_back_squat_has_the_squat_model() {
        for name in ["Back Squat", "back squat", "  BACK SQUAT "] {
            assert_eq!(model_for(name), Some(Lift::Squat), "{name:?}");
        }
    }

    #[test]
    fn exercises_without_a_model_have_none() {
        // Other squats are other mechanics (the model is the back squat);
        // the deadlift and bench wait for R-0045's garust work.
        for name in [
            "Front Squat",
            "Hack Squat",
            "Low Bar Squat",
            "Deadlift",
            "Romanian Deadlift",
            "Bench Press",
            "Close-Grip Bench Press",
            "Leg Press",
            "",
        ] {
            assert_eq!(model_for(name), None, "{name:?}");
        }
    }
}
