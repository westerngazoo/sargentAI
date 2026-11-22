//! SPEC-0066 §2.3.1 — the body's measures a request may carry (the lengths
//! and the ankle's dorsiflexion), and the standard body's cited dorsiflexion.

use crate::biomech::{Dorsiflexion, MeasuredLengths};

/// The standard body's weight-bearing ankle dorsiflexion, degrees.
///
/// Konor MM, Morton S, Eckerson JM, Grindstaff TL. *Reliability of three
/// measures of ankle dorsiflexion range of motion.* Int J Sports Phys Ther.
/// 2012;7(3):279–287 (PMC3362988): weight-bearing lunge, digital inclinometer
/// at the tibial tuberosity on the anterior tibial crest, 20 healthy adults
/// (age 24 ± 3), composite mean 38.8° ± 5.2°.
///
/// Read as the tibia's inclination from vertical, which is
/// [`Dorsiflexion`]'s angle; the paper's "relative to the ground" read from
/// horizontal would be 51°, outside the 30–50° normal range it cites
/// (SPEC-0066 §2.3.1). Used only when a request gives no dorsiflexion, and
/// stated then as `Assumption::StandardDorsiflexion`.
pub const STANDARD_DORSIFLEXION_DEG: f64 = 38.8;

/// Every body measure a request may carry, whatever the exercise.
pub const BODY_KEYS: [&str; 6] = [
    "thigh",
    "shank",
    "trunk",
    "upper_arm",
    "forearm",
    "dorsiflexion_deg",
];

/// The body measures a request gave: lengths for `scale`, and the ankle.
/// Anything absent comes from the table or the standard body.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyMeasures {
    /// The measured lengths; the rest come from the table.
    pub lengths: MeasuredLengths,
    /// The ankle's weight-bearing dorsiflexion, if measured.
    pub dorsiflexion: Option<Dorsiflexion>,
}

impl BodyMeasures {
    /// Whether the request gave the measure named `key` (one of
    /// [`BODY_KEYS`]); an unknown key was never given.
    #[must_use]
    pub fn has(&self, key: &str) -> bool {
        let l = &self.lengths;
        match key {
            "thigh" => l.thigh.is_some(),
            "shank" => l.shank.is_some(),
            "trunk" => l.trunk.is_some(),
            "upper_arm" => l.upper_arm.is_some(),
            "forearm" => l.forearm.is_some(),
            "dorsiflexion_deg" => self.dorsiflexion.is_some(),
            _ => false,
        }
    }
}
