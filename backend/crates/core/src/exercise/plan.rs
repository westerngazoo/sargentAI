//! SPEC-0059 §2.5, SPEC-0066 §2.3 — which exercise names the plan marks as
//! modelled, read from the exercise library.

use super::library;
use crate::periodize::lift_key;
use crate::technique::Lift;

/// The lift the plan names for `exercise`, or `None` when no library entry the
/// plan exposes covers it (SPEC-0059 §2.5: the server decides, not a client
/// string list).
///
/// It looks the name up, after [`lift_key`] normalization (trim, lowercase),
/// among the aliases of the library entries whose `in_plan_as` is set: exact,
/// never fuzzy. Only the back squat is exposed today, as [`Lift::Squat`], so
/// installed app builds keep their *Física* button; an entry joins the plan
/// when the app can open it (SPEC-0066 §2.3, SPEC-0067).
///
/// Narrower than [`Lift::from_exercise_name`] on purpose: that one says which
/// lift a video shows; this one says which mechanics are modelled.
#[must_use]
pub fn model_for(exercise: &str) -> Option<Lift> {
    let key = lift_key(exercise);
    library().iter().find_map(|model| {
        let d = model.describe();
        d.in_plan_as
            .filter(|_| d.aliases.iter().any(|alias| *alias == key))
    })
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
    fn the_spanish_back_squat_names_have_the_squat_model() {
        for name in [
            "Sentadilla",
            "sentadilla",
            " SENTADILLA ",
            "Sentadilla trasera",
            "sentadilla trasera",
            "SENTADILLA TRASERA",
        ] {
            assert_eq!(model_for(name), Some(Lift::Squat), "{name:?}");
        }
    }

    #[test]
    fn exercises_without_a_model_have_none() {
        // Other squats are other mechanics (the model is the back squat);
        // the deadlift and bench wait for R-0045's garust work.
        for name in [
            "Squat",
            "Sentadilla frontal",
            "Sentadilla  trasera",
            "Sentadillas",
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
