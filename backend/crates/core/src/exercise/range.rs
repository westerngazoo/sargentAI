//! SPEC-0066 §2.3 — what every exercise's range returns: how it ended, its
//! frames (named points and joint moments), the settings used and what it
//! all holds under.

use serde::Serialize;

use super::input::Values;
use crate::biomech::{Assumption, JointMoment, Point, Rejection};

/// One exercise's range of postures.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExerciseRange {
    /// Whether it reached its end, stopped part-way, or could not start.
    pub end: End,
    /// The frames the model stands behind, in order; empty when rejected.
    pub frames: Vec<Frame>,
    /// Every value used, defaults included (R-0066 AC2).
    pub settings: Values,
    /// What readings across the frames hold under.
    pub assumptions: Vec<Assumption>,
}

/// How a range ended (SPEC-0059's `end`, generalized).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum End {
    /// The whole motion.
    Complete,
    /// A prefix of frames, then why the motion could go no further.
    Stopped {
        /// The progress reached, `0..1`.
        reached: f64,
        /// Why it stopped.
        reason: Reason,
    },
    /// No frames: the motion cannot start.
    Rejected {
        /// Why.
        reason: Reason,
    },
}

/// Why a range stopped or could not start: a closed set for the library.
/// Further variants arrive with the exercises that can reach them.
///
/// Tagged `source`, not `reason`: the squat's [`Rejection`] is itself tagged
/// `reason`, and one object must not carry the key twice.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "source")]
pub enum Reason {
    /// The squat's own rejection, unchanged.
    Squat(Rejection),
    /// The squat's bottom balances, but its convention passes horizontal on
    /// the way down (SPEC-0045 §2.6.6, `Stopped { bottom: None }`).
    ConventionPassesHorizontal,
}

/// One posture and its moments.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Frame {
    /// Progress along the model's convention, `0..=1`.
    pub progress: f64,
    /// The named points the app draws (`ankle`, `knee`, `hip`, `shoulder`,
    /// `bar`, …).
    pub points: Vec<NamedPoint>,
    /// Each reported joint's moment, signed, with its resisting group and
    /// uncertainty.
    pub moments: Vec<JointMoment>,
    /// What this frame's numbers hold under (R-0066 §1).
    pub assumptions: Vec<Assumption>,
}

/// A point the app draws, by name.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct NamedPoint {
    /// `ankle`, `knee`, `hip`, `shoulder`, `bar`, …
    pub name: &'static str,
    /// Where, in the sagittal plane.
    pub at: Point,
}
