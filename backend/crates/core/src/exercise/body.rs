//! SPEC-0066 §2.3.1 — the body's measures a request may carry (the lengths
//! and the ankle's dorsiflexion): what each one is (served, so the app builds
//! its *Cuerpo* controls from the server, R-0067 AC3), what a request gave, and
//! what a range ran on.

use serde::Serialize;

use super::input::Invalid;
use crate::biomech::{Dorsiflexion, MeasuredLengths, Metres};

/// The standard body's weight-bearing ankle dorsiflexion, degrees.
///
/// Konor MM, Morton S, Eckerson JM, Grindstaff TL. *Reliability of three
/// measures of ankle dorsiflexion range of motion.* Int J Sports Phys Ther.
/// 2012;7(3):279–287 (PMC3362988, PMID 22666642): weight-bearing lunge,
/// digital inclinometer at the tibial tuberosity on the anterior tibial crest,
/// 20 healthy adults (age 24 ± 3), composite mean 38.8° ± 5.2°.
///
/// **The reading is an inference, stated (SPEC-0066 §2.3.1):** the paper
/// describes the angle as "relative to the ground (horizontal)", but compares
/// this mean with normative values of 30–50° reported as the tibia's angle from
/// vertical. This model takes it from vertical, which is [`Dorsiflexion`]'s
/// angle. Used only when a request gives no dorsiflexion, and stated then as
/// `Assumption::StandardDorsiflexion`.
pub const STANDARD_DORSIFLEXION_DEG: f64 = 38.8;

/// One body measure: its key, unit, bounds, the control's step, and the
/// standard body's value when it does not depend on height.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct BodyMeasure {
    /// The request's `body` key.
    pub key: &'static str,
    /// The unit the app shows.
    pub unit: &'static str,
    /// The least accepted value.
    pub min: f64,
    /// The greatest accepted value.
    pub max: f64,
    /// The control's step, a hint.
    pub step: f64,
    /// The standard body's value, when one is cited and height-independent;
    /// `None` for lengths, which the table scales from height.
    pub standard: Option<f64>,
}

/// Every body measure a request may carry, whatever the exercise. The one
/// home for their bounds: validation and the served description read it.
pub const BODY_MEASURES: [BodyMeasure; 6] = [
    length("thigh"),
    length("shank"),
    length("trunk"),
    length("upper_arm"),
    length("forearm"),
    BodyMeasure {
        key: "dorsiflexion_deg",
        unit: "deg",
        min: 0.0,
        max: 90.0,
        step: 0.5,
        standard: Some(STANDARD_DORSIFLEXION_DEG),
    },
];

/// A length: `0.1..=1.5` m, SPEC-0059's bound.
const fn length(key: &'static str) -> BodyMeasure {
    BodyMeasure {
        key,
        unit: "m",
        min: 0.1,
        max: 1.5,
        step: 0.005,
        standard: None,
    }
}

/// The served descriptions of `keys`, in order. A model names its measures by
/// key; `every_library_description_is_well_formed` checks each one is known.
pub(super) fn body_measures(keys: &[&str]) -> Vec<BodyMeasure> {
    keys.iter()
        .filter_map(|key| BODY_MEASURES.iter().find(|m| m.key == *key).copied())
        .collect()
}

/// The body measures a request gave, validated. Anything absent comes from
/// the table or the standard body.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BodyMeasures {
    given: Vec<(&'static str, f64)>,
}

impl BodyMeasures {
    /// Records `value` for `measure`, within its bounds.
    ///
    /// # Errors
    ///
    /// [`Invalid`] naming the measure when `value` is outside its bounds or
    /// not finite.
    pub(super) fn set(&mut self, measure: &BodyMeasure, value: f64) -> Result<(), Invalid> {
        if !(measure.min..=measure.max).contains(&value) {
            return Err(Invalid { field: measure.key });
        }
        self.given.push((measure.key, value));
        Ok(())
    }

    /// The value the request gave for `key`, if any.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<f64> {
        self.given.iter().find(|(k, _)| *k == key).map(|&(_, v)| v)
    }

    /// The given lengths, for `scale`. The one place a key becomes a length.
    #[must_use]
    pub fn lengths(&self) -> MeasuredLengths {
        let metres = |key| self.get(key).map(Metres);
        MeasuredLengths {
            thigh: metres("thigh"),
            shank: metres("shank"),
            trunk: metres("trunk"),
            upper_arm: metres("upper_arm"),
            forearm: metres("forearm"),
        }
    }

    /// The given dorsiflexion, if any.
    #[must_use]
    pub fn dorsiflexion(&self) -> Option<Dorsiflexion> {
        self.get("dorsiflexion_deg")
            .and_then(Dorsiflexion::from_degrees)
    }
}

/// Where a body measure a range ran on came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// The request gave it.
    Measured,
    /// The anthropometric table, scaled by height.
    Table,
    /// The standard body's cited value.
    Standard,
}

/// A body measure as a range ran on it: the value and where it came from
/// (R-0066 AC2: the result restates what it used).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct UsedMeasure {
    /// The `body` key.
    pub key: &'static str,
    /// The value used, in the measure's unit.
    pub value: f64,
    /// Measured, table or standard.
    pub source: Source,
}
