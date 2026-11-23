//! SPEC-0066 §2.3 — one validation for every exercise: a request's values and
//! body measures checked against the exercise's [`Description`].

use serde::ser::{Serialize, SerializeMap, Serializer};

use super::body::{BodyMeasures, BODY_MEASURES};
use super::description::{Description, ParameterKind};

/// A request value: a number, or one option of a choice.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A number.
    Number(f64),
    /// An option, by name.
    Choice(String),
}

/// The field a request got wrong, as a fixed token (the API's `400` names it).
/// An unknown or repeated key is reported as `"values"` or `"body"`: a
/// request's own text never comes back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Invalid {
    /// The offending key, or `"values"` / `"body"`.
    pub field: &'static str,
}

/// What [`validate`] accepts: the settings the model runs on, and the
/// body measures the caller scales the body with. Kept apart so the
/// settings restate exactly what the model used (R-0066 AC2).
#[derive(Clone, Debug, PartialEq)]
pub struct Validated {
    /// Every parameter, defaults filled in, in bounds.
    pub settings: Values,
    /// The body measures the request gave; the rest come from the table or
    /// the standard body.
    pub body: BodyMeasures,
}

/// An exercise's settings. Only [`validate`] makes one, so a model never runs
/// on values nobody checked. Serializes as one flat object, e.g.
/// `{"dorsiflexion_deg": 38.0, "cop": "mid"}`.
#[derive(Clone, Debug, PartialEq)]
pub struct Values {
    numbers: Vec<(&'static str, f64)>,
    choices: Vec<(&'static str, &'static str)>,
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
}

impl Serialize for Values {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.numbers.len() + self.choices.len()))?;
        for (key, n) in &self.numbers {
            map.serialize_entry(key, n)?;
        }
        for (key, c) in &self.choices {
            map.serialize_entry(key, c)?;
        }
        map.end()
    }
}

/// Checks `values` and `body` against `d` (SPEC-0066 §2.3, §2.3.1).
///
/// A missing key takes its default. `step` is not enforced: it is a hint for
/// the control.
///
/// # Errors
///
/// [`Invalid`] naming the field when a key is unknown or repeated
/// (`"values"`, `"body"`), a required key is missing, a number is outside
/// its bounds (or not finite), a choice is not one of its options, a value is
/// of the wrong kind, a pair is out of order (its `high` key, even when the
/// other end was a default), or a body measure is one the exercise does not
/// use, a length outside `0.1..=1.5` m, or a dorsiflexion outside `0..=90`°.
pub fn validate(
    d: &Description,
    values: &[(String, Value)],
    body: &[(String, f64)],
) -> Result<Validated, Invalid> {
    let unknown = |key: &String| !d.parameters.iter().any(|p| p.key == key.as_str());
    if values.iter().any(|(key, _)| unknown(key)) || has_duplicates(values.iter().map(|(k, _)| k)) {
        return Err(Invalid { field: "values" });
    }
    let mut settings = Values {
        numbers: Vec::new(),
        choices: Vec::new(),
    };
    for p in &d.parameters {
        let given = values
            .iter()
            .find(|(key, _)| key.as_str() == p.key)
            .map(|(_, v)| v);
        let invalid = Invalid { field: p.key };
        match (&p.kind, given) {
            (ParameterKind::Number { min, max, .. }, Some(Value::Number(n))) => {
                if !(min..=max).contains(&n) {
                    return Err(invalid);
                }
                settings.numbers.push((p.key, *n));
            }
            (ParameterKind::Number { default, .. }, None) => {
                settings.numbers.push((p.key, default.ok_or(invalid)?));
            }
            (ParameterKind::Choice { options, .. }, Some(Value::Choice(c))) => {
                let option = options.iter().find(|o| **o == c.as_str()).ok_or(invalid)?;
                settings.choices.push((p.key, option));
            }
            (ParameterKind::Choice { default, .. }, None) => {
                settings.choices.push((p.key, default.ok_or(invalid)?));
            }
            (ParameterKind::Number { .. }, Some(Value::Choice(_)))
            | (ParameterKind::Choice { .. }, Some(Value::Number(_))) => return Err(invalid),
        }
    }
    for pair in d.pairs {
        // Every pair names two number parameters: `every_library_description_
        // is_well_formed` holds each description to it.
        if let (Some(low), Some(high)) = (settings.number(pair.low), settings.number(pair.high)) {
            if low >= high {
                return Err(Invalid { field: pair.high });
            }
        }
    }
    if has_duplicates(body.iter().map(|(k, _)| k)) {
        return Err(Invalid { field: "body" });
    }
    let mut measures = BodyMeasures::default();
    for (key, value) in body {
        let Some(measure) = BODY_MEASURES.iter().find(|m| m.key == key.as_str()) else {
            return Err(Invalid { field: "body" });
        };
        if !d.body.iter().any(|used| used.key == measure.key) {
            return Err(Invalid { field: measure.key });
        }
        measures.set(measure, *value)?;
    }
    Ok(Validated {
        settings,
        body: measures,
    })
}

fn has_duplicates<'a>(keys: impl Iterator<Item = &'a String>) -> bool {
    let mut seen: Vec<&str> = Vec::new();
    for key in keys {
        if seen.contains(&key.as_str()) {
            return true;
        }
        seen.push(key);
    }
    false
}
