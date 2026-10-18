//! SPEC-0045 §2.6.6 — the squat over its range (slice B1): from the solved
//! upright toward the §2.6 bottom.
//!
//! **One equation short, closed by one declared convention.** Between standing
//! and the bottom three angles move (shank, femur, trunk) and balance supplies
//! one equation. The legs move by convention — the shank angle `α` and the
//! femur angle `φ` interpolated **in angle, never in sine**:
//!
//! ```text
//! α(s) = (1 − s)·λ + s·δ          λ: the solved upright lean     δ: the lifter's dorsiflexion
//! φ(s) = −(1 − s)·λ + s·90°       −λ: the straight leg at upright   90°: the femur horizontal
//! knee flexion(s) = s·(δ + 90°)
//! ```
//!
//! and the trunk is **solved, never interpolated**: at every `s` it comes from
//! the §2.6.3 balance equation. The frame at `s = 1` is not interpolated
//! either; it is the output of [`baseline`](super::baseline).
//!
//! Everything read across the frames — where a moment peaks, whether the trunk
//! lean is monotone, how far a lifter reaches — is a statement about this
//! convention, not about the lifter. The lifter's own curve needs their own
//! kinematics, which is the measured source.

use serde::Serialize;

use crate::workout::LoadKg;

use super::{Body, Dorsiflexion, Mechanics, Posture, Rejection};

/// How many frames the caller wants sampled across the range: `2..=241`, a
/// resource bound rather than a physical one (SPEC-0045 §2.6.6).
///
/// Frame `i` sits at `s = i / (n − 1)`, computed directly and never
/// accumulated, so a frame at a given `s` is the same whatever `n` is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FrameCount(u16);

impl FrameCount {
    /// `None` below 2 (a range needs both ends) or above 241.
    #[must_use]
    pub fn new(_frames: u16) -> Option<Self> {
        todo!("R-0045 slice B1")
    }

    /// The number of frames requested.
    #[must_use]
    pub fn get(self) -> u16 {
        todo!("R-0045 slice B1")
    }
}

/// One solved, balanced posture of the range, and its mechanics.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RangeFrame {
    /// `s`, the progress along the §2.6.6 convention: 0 at the upright, 1 at
    /// the bottom. A coordinate of the convention, not a time.
    pub progress: f64,
    /// `α(s)`, degrees from vertical, knee ahead positive: the shank in use,
    /// not the lifter's dorsiflexion.
    pub shank_deg: f64,
    /// `s·(δ + 90°)`. Index a video comparison by this, not by `s`.
    pub knee_flexion_deg: f64,
    /// Degrees between the femur and the trunk; 0 at the upright.
    pub hip_flexion_deg: f64,
    /// The solved trunk angle, degrees from vertical, forward positive.
    pub trunk_lean_deg: f64,
    /// The points a reel or a video overlay draws.
    pub posture: Posture,
    /// The four joint moments, and what this frame admits to.
    pub mechanics: Mechanics,
}

/// The range: its frames, upright first, and how it ended.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SquatRange {
    /// The sampled frames with `s ≤ s*`, from the upright, in order.
    pub frames: Vec<RangeFrame>,
    /// Whether the range reached the bottom, and if not, why it stopped.
    pub end: RangeEnd,
}

/// How the range ended (SPEC-0045 §2.6.6). The frames are the convention's
/// **largest balanced prefix**: its end `s*` is the first `s` at which the
/// balancing trunk would leave vertical-to-horizontal.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "end")]
pub enum RangeEnd {
    /// Reached the bottom; the last frame is slice A's baseline.
    Bottom,
    /// Stopped at `reached` = `s*`.
    Stopped {
        /// `s*`, independent of the frame count.
        reached: f64,
        /// Slice A's outcome at `s = 1`: `Some(rejection)` when the bottom
        /// itself does not balance, `None` when it does but this convention
        /// passes horizontal on the way — the convention is to blame, not the
        /// ankle.
        bottom: Option<Rejection>,
    },
}

/// The squat from the solved upright toward the §2.6 bottom for `body` under
/// `bar_kg`, at `ankle` dorsiflexion, sampled at `frames` points.
///
/// # Errors
///
/// [`Rejection::DepthNotReachable`] when the lifter's dorsiflexion is below
/// the solved upright lean, which would drive the shank backwards along the
/// range; [`Rejection::DoesNotBalance`] with [`Unknown::ShankAngle`] when the
/// upright's lean is undetermined (no mass on the trunk line and no bar).
///
/// [`Unknown::ShankAngle`]: super::Unknown::ShankAngle
pub fn squat_range(
    _body: &Body,
    _bar_kg: Option<LoadKg>,
    _ankle: Dorsiflexion,
    _frames: FrameCount,
) -> Result<SquatRange, Rejection> {
    todo!("R-0045 slice B1")
}
