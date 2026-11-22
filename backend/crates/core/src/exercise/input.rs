//! SPEC-0066 §2.3 — one validation for every exercise: a request's values and
//! lengths checked against the exercise's [`Description`].

use serde::Serialize;

use super::description::{Description, ParameterKind};
use crate::biomech::{MeasuredLengths, Metres};

/// Bounds on a measured length, metres (SPEC-0059's bound).
const LENGTH_M: std::ops::RangeInclusive<f64> = 0.1..=1.5;

/// The lengths a request may carry, whatever the exercise.
const LENGTH_KEYS: [&str; 5] = ["thigh", "shank", "trunk", "upper_arm", "forearm"];

/// A request value: a number, or one option of a choice.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A number.
    Number(f64),
    /// An option, by name.
    Choice(String),
}

/// The field a request got wrong, as a fixed token (the API's `400` names it).
/// An unknown key is reported as `"values"` or `"lengths"`: a request's own
/// text never comes back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Invalid {
    /// The offending key, or `"values"` / `"lengths"` for an unknown one.
    pub field: &'static str,
}

/// A request validated against a description: every parameter present
/// (defaults filled in) and in bounds, and the measured lengths.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Values {
    numbers: Vec<(&'static str, f64)>,
    choices: Vec<(&'static str, &'static str)>,
    #[serde(skip)]
    lengths: MeasuredLengths,
}

impl Values {
    /// The number for `key`, if the description has it.
    #[must_use]
    pub fn number(&self, key: &str) -> Option<f64> {
        self.numbers
            .iter()
            .find(|(k, _)| *k == key)
            .map(|&(_, v)| v)
    }

    /// The option for `key`, if the description has it.
    #[must_use]
    pub fn choice(&self, key: &str) -> Option<&'static str> {
        self.choices
            .iter()
            .find(|(k, _)| *k == key)
            .map(|&(_, v)| v)
    }

    /// The measured lengths, for `scale`.
    #[must_use]
    pub fn lengths(&self) -> MeasuredLengths {
        self.lengths
    }
}

/// Checks `values` and `lengths` against `d` (SPEC-0066 §2.3).
///
/// A missing key takes its default. `step` is not enforced: it is a hint for
/// the control.
///
/// # Errors
///
/// [`Invalid`] naming the field when a key is unknown (`"values"`,
/// `"lengths"`), a required key is missing, a number is outside its bounds
/// (or not finite), a choice is not one of its options, a value is of the
/// wrong kind, a pair is out of order (its `high` key), or a length is one
/// the exercise does not use or outside `0.1..=1.5` m.
pub fn validate(
    d: &Description,
    values: &[(String, Value)],
    lengths: &[(String, f64)],
) -> Result<Values, Invalid> {
    if values
        .iter()
        .any(|(key, _)| !d.parameters.iter().any(|p| p.key == key.as_str()))
    {
        return Err(Invalid { field: "values" });
    }
    let mut out = Values::default();
    for p in &d.parameters {
        let given = values.iter().find(|(key, _)| key.as_str() == p.key);
        let invalid = Invalid { field: p.key };
        match (&p.kind, given.map(|(_, v)| v)) {
            (ParameterKind::Number { min, max, .. }, Some(Value::Number(n))) => {
                if !(min..=max).contains(&n) {
                    return Err(invalid);
                }
                out.numbers.push((p.key, *n));
            }
            (ParameterKind::Number { default, .. }, None) => {
                out.numbers.push((p.key, default.ok_or(invalid)?));
            }
            (ParameterKind::Choice { options, .. }, Some(Value::Choice(c))) => {
                let option = options.iter().find(|o| **o == c.as_str()).ok_or(invalid)?;
                out.choices.push((p.key, option));
            }
            (ParameterKind::Choice { default, .. }, None) => {
                out.choices.push((p.key, default.ok_or(invalid)?));
            }
            (ParameterKind::Number { .. }, Some(Value::Choice(_)))
            | (ParameterKind::Choice { .. }, Some(Value::Number(_))) => return Err(invalid),
        }
    }
    for pair in d.pairs {
        if let (Some(low), Some(high)) = (out.number(pair.low), out.number(pair.high)) {
            if low >= high {
                return Err(Invalid { field: pair.high });
            }
        }
    }
    for (key, metres) in lengths {
        let Some(&known) = LENGTH_KEYS.iter().find(|k| **k == key.as_str()) else {
            return Err(Invalid { field: "lengths" });
        };
        if !d.lengths.contains(&known) || !LENGTH_M.contains(metres) {
            return Err(Invalid { field: known });
        }
        let m = Some(Metres(*metres));
        match known {
            "thigh" => out.lengths.thigh = m,
            "shank" => out.lengths.shank = m,
            "trunk" => out.lengths.trunk = m,
            "upper_arm" => out.lengths.upper_arm = m,
            _ => out.lengths.forearm = m,
        }
    }
    Ok(out)
}
