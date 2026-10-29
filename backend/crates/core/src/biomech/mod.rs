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
//! applied in exactly one module, `chain` — by [`moment_about`] and its body
//! share [`body_moment_about`].
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

mod analysis;
mod anthro;
mod chain;
mod model;
mod range;
mod squat;

use serde::{Deserialize, Serialize};

use crate::workout::LoadKg;

pub use analysis::{
    analyse_baseline, Assumption, JointMoment, LoadBasis, Mechanics, Position, ResistingGroup,
    Uncertainty, COM_PLACEMENT_SPREAD_M, L5S1_PLACEMENT_SPREAD_M,
};
pub use anthro::{
    scale, Body, Length, MeasuredLengths, Segment, Table, Verification, WINTER, WINTER_VERIFICATION,
};
pub use chain::{above, body_moment_about, moment_about, ChainSegment};
pub use model::model_for;
pub use range::{squat_range, FrameCount, RangeEnd, RangeFrame, SquatRange};
pub use squat::{baseline, baseline_mechanics, Baseline, ANKLE_BEHIND_MID_FOOT, HIGH_BAR};

/// Standard gravity, m·s⁻². One constant for the whole model (SPEC-0045 §2.2);
/// `chain` ([`moment_about`], [`body_moment_about`]) is the only place it is
/// multiplied.
pub const G: f64 = 9.81;

/// Where L5/S1 sits on the trunk line, as a fraction of hip → shoulder.
///
/// **Derived from the table, not chosen** (SPEC-0045 §2.5). Winter gives no
/// L5/S1 placement, but it does give the lumped HAT row — same mass, same
/// line, its own centre of mass — so the split must reproduce it. Solving
/// that identity for the one placement the table does not supply gives this
/// value, and `hat_row_is_reproduced_by_the_split` is the claim that keeps it
/// honest.
///
/// The draft's `0.10` was a convention nobody had checked, and it was wrong:
/// it put L5/S1 5 cm above the greater trochanter and left the split's centre
/// of mass at 0.539 of the trunk line against the table's own 0.626 — a 16%
/// error in every hip moment the model produced. At this value the split and
/// the lump agree to 0.2%, and L5/S1 lands 13.5 cm above the trochanter for a
/// 1.75 m lifter, which is where it belongs.
///
/// The L4-L5 segment boundary is taken as the same point, the mismatch
/// absorbed into the [`Uncertainty::l5s1_placement`] spread.
pub const L5S1_UP_THE_TRUNK: f64 = 0.267_425_301;

/// Metres. A newtype so a lever arm cannot be mistaken for a ratio, a pixel or
/// a count (SPEC-0045 §3, architect finding 22).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Metres(pub f64);

/// Newton-metres, **signed** — the moment resisting extension (SPEC-0045
/// §2.4.1). Positive: the extensors are loaded.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NewtonMetres(pub f64);

/// Weight-bearing ankle dorsiflexion. With the foot flat this **is** the
/// shank's angle from vertical — the same angle from the same neutral — which
/// is why the baseline's shank angle is a property of the lifter rather than a
/// convention to pick (SPEC-0045 §2.6.1.1).
///
/// It is a **required** input to [`baseline`]: `core::biomech` commits no
/// population default, because an uncited one would be an invented constant and
/// no sum claim could catch it. A caller with no measurement must decide for
/// itself what to tell the user.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Dorsiflexion {
    degrees: f64,
}

impl Dorsiflexion {
    /// `None` outside `0°..=90°`, the geometric domain of a shank angle from
    /// vertical — not a claim about any ankle. Slice B gives the out-of-domain
    /// case a typed home when dorsiflexion becomes a knob
    /// (`Rejection::OutOfRange`); until then there is one reason and the caller
    /// owns the message.
    #[must_use]
    pub fn from_degrees(degrees: f64) -> Option<Self> {
        (degrees.is_finite() && (0.0..=90.0).contains(&degrees)).then_some(Self { degrees })
    }

    /// The angle in degrees, as it was measured and as coaching reports it.
    #[must_use]
    pub fn degrees(self) -> f64 {
        self.degrees
    }

    /// `sin δ` — the shank's forward reach per metre of shank.
    pub(super) fn sin(self) -> f64 {
        self.degrees.to_radians().sin()
    }
}

/// A point in the sagittal plane, metres. `+x` toward the toes, `+y` up,
/// origin on the mid-foot line at floor level (SPEC-0045 §2.1).
///
/// `Serialize` so the points reach a reel or a client without anyone
/// recomputing where the knee is (SPEC-0045 §2.6.6).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Point {
    /// Horizontal, toward the toes.
    pub x: Metres,
    /// Vertical, up from the floor.
    pub y: Metres,
}

/// The external load, with the point it acts at. A bar with no position is
/// unrepresentable (SPEC-0045 §2.1, architect finding 9), and a bar with an
/// impossible mass is too: [`LoadKg`] is the crate's validated load, finite
/// and within `(0, 1000]`. Without it a negative bar solved to a plausible
/// posture, with the physics rejections standing in by accident for what is a
/// type error.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Load {
    /// Mass on the bar.
    pub kg: LoadKg,
    /// Where it acts.
    pub at: Point,
}

/// A posture the model can load: the four joint centres it needs, in metres,
/// and where the load acts. Produced by the v1 [`baseline`] solve or, later,
/// by a measurement; the model does not care which (SPEC-0045 §2.1).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
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
    pub fn at(&self, j: Joint) -> Point {
        match j {
            Joint::Hip => self.hip,
            Joint::Knee => self.knee,
            Joint::Ankle => self.ankle,
            Joint::Lumbar => self.l5s1(L5S1_UP_THE_TRUNK),
        }
    }

    /// L5/S1 placed `fraction` of the way up the trunk line. The parameter
    /// exists so the uncertainty walk can nudge the convention without a
    /// second copy of the placement rule.
    fn l5s1(&self, fraction: f64) -> Point {
        along(self.hip, self.shoulder, fraction)
    }
}

/// The point `fraction` of the way from `a` to `b`. Extrapolates beyond `b`
/// for `fraction > 1` — which is how the head & neck COM is placed on the
/// trunk line beyond the shoulder (SPEC-0045 §2.4.2).
pub(super) fn along(a: Point, b: Point, fraction: f64) -> Point {
    Point {
        x: Metres(a.x.0 + fraction * (b.x.0 - a.x.0)),
        y: Metres(a.y.0 + fraction * (b.y.0 - a.y.0)),
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

impl Joint {
    /// The group that resists this joint's moment — and nothing finer
    /// (R-0045 AC5, SPEC-0045 §2.7).
    #[must_use]
    pub fn resisting(self) -> ResistingGroup {
        match self {
            Self::Hip => ResistingGroup::HipExtensors,
            Self::Knee => ResistingGroup::KneeExtensors,
            Self::Ankle => ResistingGroup::PlantarFlexors,
            Self::Lumbar => ResistingGroup::SpinalErectors,
        }
    }

    /// `+1` or `−1`: what the raw coordinate sum must be multiplied by to
    /// become the moment **resisting extension** (SPEC-0045 §2.4.1).
    ///
    /// The knee is the one that flips. On a zig-zag chain every load-side
    /// centre of mass sits *behind* the knee while the knee extensors are
    /// loaded, so its raw sum is negative exactly when the hip's is positive.
    /// This is the only place the orientation is applied — architect finding 3
    /// was a model that applied it in several, and disagreed with itself.
    pub(super) fn extension_sign(self) -> f64 {
        match self {
            Self::Hip | Self::Ankle | Self::Lumbar => 1.0,
            Self::Knee => -1.0,
        }
    }
}

/// The one unknown the balance equation is solved for (SPEC-0045 §2.6.3):
/// the trunk angle unless a torso angle is specified, in which case the shank
/// angle. The rule decides which rejection fires.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unknown {
    /// Trunk angle from vertical, θ — the baseline's unknown.
    TrunkAngle,
    /// Shank angle from vertical — the unknown when a torso angle is fixed,
    /// and the range's upright lean (SPEC-0045 §2.6.6).
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
    /// A solved joint angle is outside the range observed in lifting.
    ///
    /// **Unreachable in slice A**: SPEC-0045 §2.6.4 commits no limit constant
    /// until one is cited from measured squat kinematics, and a threshold
    /// nobody has sourced is not a threshold. The variant is present so the
    /// closed set does not change shape when one is.
    AnatomicallyImplausible {
        /// The joint that left its range.
        joint: Joint,
        /// The solved angle, degrees.
        angle_deg: f64,
    },
    /// The depth needs more range than the lifter has — at the baseline, more
    /// ankle dorsiflexion than they possess.
    ///
    /// This is a **finding about the lifter, not a failure of the model**:
    /// "your ankle is what stops you" is the most useful sentence available
    /// for someone whose ankles cannot reach parallel, and it beats a number
    /// for a posture they cannot hold. `needed_deg` is their own algebra
    /// (SPEC-0045 §2.6.3's `u_min`), not a comparison against a norm.
    DepthNotReachable {
        /// The joint that runs out first. At the baseline, always the ankle:
        /// it is the only joint whose range is an input.
        limiting: Joint,
        /// The least dorsiflexion at which this lift reaches this depth.
        needed_deg: f64,
        /// What the lifter has.
        available_deg: f64,
    },
}
