//! SPEC-0066 §2.3 — what an exercise says about itself: its id and names, its
//! equipment, the joints it reports, the lengths it uses, and the parameters
//! R-0067's controls are built from.

use serde::Serialize;

use crate::biomech::Joint;
use crate::technique::Lift;

/// An exercise's self-description (R-0066 AC1, AC9). Built by value: cheap,
/// and it keeps the trait free of statics.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Description {
    /// Stable id, e.g. `"back_squat"`. Never renamed once served.
    pub id: &'static str,
    /// The name in Spanish, the app's first language.
    pub name_es: &'static str,
    /// The name in English.
    pub name_en: &'static str,
    /// The names a plan or a log may use, already lowercase and trimmed
    /// (the plan matches them exactly after `lift_key`).
    pub aliases: &'static [&'static str],
    /// The kind of resistance (R-0066 AC2).
    pub equipment: Equipment,
    /// The joints the range reports a moment for.
    pub joints: &'static [Joint],
    /// The body lengths the model uses, by the request's `lengths` keys; the
    /// rest of a body comes from the table.
    pub lengths: &'static [&'static str],
    /// The controls: motion, machine and load.
    pub parameters: Vec<Parameter>,
    /// Pairs of number parameters that must stay ordered.
    pub pairs: &'static [Ordered],
    /// The value the plan's `physics` field carries for this exercise, or
    /// `None` while the app cannot open it (SPEC-0066 §2.3, SPEC-0067).
    #[serde(skip)]
    pub in_plan_as: Option<Lift>,
}

/// The four kinds of resistance (R-0066 AC2). `LeverCam` has no entry yet
/// (R-0066 OQ-3: its first entry waits for a measurable machine).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Equipment {
    /// A mass under gravity: barbell, dumbbell, kettlebell, the body itself.
    FreeWeight,
    /// A cable pulling toward a pulley.
    Cable,
    /// A carriage on a rail.
    Guided,
    /// A lever or cam with a resistance profile.
    LeverCam,
}

/// Which group of R-0067's controls a parameter belongs to. The body's
/// lengths are not parameters: they travel at the request's top level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    /// How the exercise moves.
    Motion,
    /// The equipment's settings.
    Machine,
    /// The load.
    Load,
}

/// One control.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Parameter {
    /// The request's key, e.g. `"pulley_height_m"`.
    pub key: &'static str,
    /// Its group of controls.
    pub group: Group,
    /// A number or a choice, with its bounds and default.
    pub kind: ParameterKind,
}

/// A parameter's kind. A `default` of `None` makes it required: the squat's
/// dorsiflexion is one, because SPEC-0045 commits no uncited population
/// default.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ParameterKind {
    /// A number in `min..=max`. `step` is a hint for the control, not a rule.
    Number {
        /// The unit the app shows, e.g. `"deg"`, `"m"`, `"kg"`.
        unit: &'static str,
        /// The least accepted value.
        min: f64,
        /// The greatest accepted value.
        max: f64,
        /// The control's step.
        step: f64,
        /// The value when the request leaves it out; `None` means required.
        default: Option<f64>,
    },
    /// One of a fixed set of options.
    Choice {
        /// The accepted options.
        options: &'static [&'static str],
        /// The option when the request leaves it out; `None` means required.
        default: Option<&'static str>,
    },
}

/// `low < high`, else the request is invalid and `high` is named: the two
/// ends of a motion (SPEC-0066 §2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Ordered {
    /// The key that must be smaller.
    pub low: &'static str,
    /// The key that must be larger, and the one an error names.
    pub high: &'static str,
}
