//! R-0045 / SPEC-0045 — lift biomechanics: the moment at each joint of a
//! posture, and (slice B) how it moves when the lifter changes something.
//!
//! The model consumes a [`Posture`] — joint positions in **metres**, sagittal
//! plane, plus where the external load acts — and never knows what produced
//! it (SPEC-0045 §2.1, `Model × Source`): in v1 the parametric [`baseline`]
//! solve; later, the R-0044 keypoint series through a scale bridge. Pure — no
//! I/O, no model, no clock, no randomness (R-0045 AC15).
//!
//! Slice A (SPEC-0045 §2.10): the anthropometric table, the static chain with
//! its per-joint sign rule, the lumbar row, the baseline squat solve with the
//! system-COM balance constraint, and the two rejections reachable without
//! knobs.
//!
//! ## Coordinate space (SPEC-0045 §2.1) — load-bearing
//!
//! `+x` toward the toes, `+y` up, origin on the mid-foot line at floor level.
//! Every lever arm is a difference of `x` in this space.
//!
//! ## Sign convention, stated once (SPEC-0045 §2.4.1)
//!
//! A moment is reported as the moment **resisting extension**: positive means
//! the extensors are loaded, negative means the load is assisting extension —
//! a real state, never flattened by `.abs()`. The raw coordinate sum flips
//! sign at the knee on a zig-zag chain, so the per-joint orientation is
//! applied in exactly one place, [`moment_about`].
//!
//! ## What stays out of the type system on purpose
//!
//! No `Muscle` type exists and none may be added (R-0045 AC5): a joint moment
//! is attributed to the group that resists it, and the split across the
//! muscles crossing a joint is indeterminate without a musculoskeletal model.
//! Nothing here says *risk*, *danger*, *safe* or *injury* (AC14): the output
//! describes load distribution, not risk.

#[cfg(test)]
mod tests;

mod anthro;
mod chain;
mod squat;

use serde::{Deserialize, Serialize};

pub use anthro::{
    scale, Body, Length, MeasuredLengths, Segment, Table, Verification, WINTER, WINTER_VERIFICATION,
};
pub use chain::{above, moment_about, ChainSegment};
pub use squat::baseline;

/// Metres. A newtype so a lever arm cannot be mistaken for a ratio, a pixel or
/// a count (SPEC-0045 §3, architect finding 22).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Metres(pub f64);

/// Newton-metres, **signed** — the moment resisting extension (SPEC-0045
/// §2.4.1). Positive: the extensors are loaded.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct NewtonMetres(pub f64);

/// A point in the sagittal plane, metres. `+x` toward the toes, `+y` up,
/// origin on the mid-foot line at floor level (SPEC-0045 §2.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// Horizontal, toward the toes.
    pub x: Metres,
    /// Vertical, up from the floor.
    pub y: Metres,
}

/// The external load, with the point it acts at. A bar with no position is
/// unrepresentable (SPEC-0045 §2.1, architect finding 9).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Load {
    /// Mass on the bar, kilograms.
    pub kg: f64,
    /// Where it acts.
    pub at: Point,
}

/// A posture the model can load: the four joint centres it needs, in metres,
/// and where the load acts. Produced by the v1 [`baseline`] solve or, later,
/// by a measurement; the model does not care which (SPEC-0045 §2.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Posture {
    /// Ankle joint centre.
    pub ankle: Point,
    /// Knee joint centre.
    pub knee: Point,
    /// Hip joint centre (greater trochanter).
    pub hip: Point,
    /// Shoulder (glenohumeral) — the top of the trunk line.
    pub shoulder: Point,
    /// `None` is bodyweight-only; `Some` carries both the mass and where it is.
    pub load: Option<Load>,
}

impl Posture {
    /// Where joint `j` is. Ankle, knee and hip are the posture's own points;
    /// L5/S1 is placed on the trunk line above the hip by the SPEC-0045 §2.5
    /// convention.
    #[must_use]
    pub fn at(&self, _j: Joint) -> Point {
        todo!("R-0045 slice A")
    }
}

/// The four joints of the sagittal chain, in the result's fixed order
/// (SPEC-0045 §2.8, §5.3). `Lumbar` is the single L5/S1 joint of §2.5.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Joint {
    /// Hip — resisted by the hip extensors.
    Hip,
    /// Knee — resisted by the knee extensors.
    Knee,
    /// Ankle — resisted by the plantar flexors.
    Ankle,
    /// L5/S1 — the net extensor moment attributed to the erector group.
    Lumbar,
}

/// The one unknown the balance equation is solved for (SPEC-0045 §2.6.3):
/// the trunk angle unless a torso angle is specified, in which case the shank
/// angle. The rule decides which rejection fires.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unknown {
    /// Trunk angle from vertical, θ — the baseline's unknown.
    TrunkAngle,
    /// Shank angle from vertical — the unknown when a torso angle is fixed.
    ShankAngle,
}

/// Why a requested posture was rejected (R-0045 AC9, AC13) — a closed set,
/// every variant reachable through the public solve (SPEC-0045 §2.6.4). The
/// model must not silently return numbers for a posture a human cannot hold.
///
/// Slice A carries the two variants reachable without knobs (§2.10); the
/// knobs' `OutOfRange` arrives with them in slice B and fires first, so the
/// ranges bound what the solve ever sees.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "reason")]
pub enum Rejection {
    /// The balance equation has no solution in the unknown's domain: the
    /// lifter would have to lean past horizontal (θ), or the knee would have
    /// to travel past the foot's reach (shank angle).
    DoesNotBalance {
        /// Which unknown ran out of domain.
        unknown: Unknown,
    },
    /// A solved joint angle is outside physiological range — stated
    /// conventions pending a goniometric source (§2.6.4).
    AnatomicallyImplausible {
        /// The joint that left its range.
        joint: Joint,
        /// The solved angle, degrees.
        angle_deg: f64,
    },
}
