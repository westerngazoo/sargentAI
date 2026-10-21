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

use std::f64::consts::FRAC_PI_2;

use serde::Serialize;

use crate::workout::LoadKg;

use super::analysis::analyse_range;
use super::squat::{balanced, balancing_trunk, first_moment_at, Legs};
use super::{
    analyse_baseline, baseline, Body, Dorsiflexion, Joint, Mechanics, Posture, Rejection, Unknown,
};

/// How finely the range is scanned for its end before bisecting. Fixed, so
/// `s*` does not depend on how many frames the caller asks for. A hole
/// narrower than one step between two balanced scan points would be missed;
/// the narrowest §2.6.6 found is about 0.25 of the range, three orders wider.
const SCAN_STEPS: u32 = 4096;

/// How closely `s*` is bisected.
const S_TOLERANCE: f64 = 1e-9;

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
    pub fn new(frames: u16) -> Option<Self> {
        (2..=241).contains(&frames).then_some(Self(frames))
    }

    /// The number of frames requested.
    #[must_use]
    pub fn get(self) -> u16 {
        self.0
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
#[serde(rename_all = "snake_case", tag = "kind")]
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
    body: &Body,
    bar_kg: Option<LoadKg>,
    ankle: Dorsiflexion,
    frames: FrameCount,
) -> Result<SquatRange, Rejection> {
    let lean = upright_lean(body, bar_kg).ok_or(Rejection::DoesNotBalance {
        unknown: Unknown::ShankAngle,
    })?;
    let dorsi = ankle.degrees().to_radians();
    if dorsi < lean {
        return Err(Rejection::DepthNotReachable {
            limiting: Joint::Ankle,
            needed_deg: lean.to_degrees(),
            available_deg: ankle.degrees(),
        });
    }
    let convention = Convention {
        body,
        bar_kg,
        lean,
        dorsi,
    };

    let bottom = baseline(body, bar_kg, ankle);
    let end = match convention.first_failure(bottom.is_ok()) {
        None => RangeEnd::Bottom,
        Some(reached) => RangeEnd::Stopped {
            reached,
            bottom: bottom.as_ref().err().copied(),
        },
    };
    let reach = match end {
        RangeEnd::Bottom => 1.0,
        RangeEnd::Stopped { reached, .. } => reached,
    };

    let last = frames.get() - 1;
    let mut out = Vec::with_capacity(usize::from(frames.get()));
    for i in 0..frames.get() {
        let s = f64::from(i) / f64::from(last);
        if s > reach {
            break;
        }
        let knee_flexion_deg = s * (ankle.degrees() + 90.0);
        let frame = if i == last {
            // Reached only when the end is the bottom: the frame is slice A's
            // baseline itself, never an interpolation that lands near it.
            bottom.as_ref().ok().map(|solved| {
                frame_of(
                    s,
                    ankle.degrees(),
                    knee_flexion_deg,
                    *solved.posture(),
                    analyse_baseline(body, solved),
                )
            })
        } else {
            let at = if i == 0 {
                RangeAt::Upright
            } else {
                RangeAt::Partial
            };
            balanced(body, bar_kg, &legs_at(s, lean, dorsi)).map(|posture| {
                let solved = RangePosture { posture, at };
                frame_of(
                    s,
                    shank_at(s, lean, dorsi).to_degrees(),
                    knee_flexion_deg,
                    posture,
                    analyse_range(body, &solved),
                )
            })
        };
        // Every sampled `s ≤ s*` balances by construction of `s*`; a `None`
        // here would be a frame on the boundary itself, and the range ends.
        let Some(frame) = frame else { break };
        out.push(frame);
    }
    Ok(SquatRange { frames: out, end })
}

/// Where a range posture sits, which decides what it admits to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RangeAt {
    /// `s = 0`, the solved straight-body upright.
    Upright,
    /// Strictly between the upright and the bottom.
    Partial,
}

/// A posture the range solved, and which kind. Only this module builds one,
/// so `analyse_range` cannot be handed a posture nobody solved: the range's
/// counterpart of [`Baseline`](super::Baseline) (SPEC-0045 §3).
pub(super) struct RangePosture {
    posture: Posture,
    at: RangeAt,
}

impl RangePosture {
    /// The posture itself.
    pub(super) fn posture(&self) -> &Posture {
        &self.posture
    }

    /// Where in the range it sits.
    pub(super) fn at(&self) -> RangeAt {
        self.at
    }
}

/// The shank angle `α(s)`, radians.
fn shank_at(s: f64, lean: f64, dorsi: f64) -> f64 {
    (1.0 - s) * lean + s * dorsi
}

/// The convention's legs at progress `s` (SPEC-0045 §2.6.6): both angles
/// interpolated in radians, never in sine, in the `(1 − s)·a + s·b` form.
fn legs_at(s: f64, lean: f64, dorsi: f64) -> Legs {
    Legs::at_angles(shank_at(s, lean, dorsi), (1.0 - s) * -lean + s * FRAC_PI_2)
}

/// The upright lean `λ`, radians: the straight body, knee and hip extended
/// and the trunk in line, leaning from the ankle until the system centre of
/// mass is over the mid-foot (the ankle strategy of quiet standing).
///
/// `Σ m·x` is affine in `sin λ` along the straight body, so two probes of the
/// model determine it, the §3 rule for `P`, `Q` and `B`: measured off the
/// model, never re-derived from the segment fractions. `None` when the lean
/// does not move the system (no mass and no bar) or no forward lean balances.
fn upright_lean(body: &Body, bar_kg: Option<LoadKg>) -> Option<f64> {
    let straight = |sin_lean: f64| {
        let lean = sin_lean.asin();
        first_moment_at(body, bar_kg, &Legs::at_angles(lean, -lean), sin_lean)
    };
    let vertical = straight(0.0);
    let per_unit = straight(1.0) - vertical;
    if per_unit <= 0.0 {
        return None;
    }
    let sin_lean = -vertical / per_unit;
    (0.0..1.0).contains(&sin_lean).then(|| sin_lean.asin())
}

/// One lifter's convention: everything the end search needs to ask "does the
/// range balance at `s`?".
struct Convention<'a> {
    body: &'a Body,
    bar_kg: Option<LoadKg>,
    lean: f64,
    dorsi: f64,
}

impl Convention<'_> {
    /// Whether the trunk that balances the legs at `s < 1` lies between
    /// vertical and horizontal.
    fn balances_at(&self, s: f64) -> bool {
        balancing_trunk(self.body, self.bar_kg, &legs_at(s, self.lean, self.dorsi))
            .is_some_and(|sin_trunk| (0.0..=1.0).contains(&sin_trunk))
    }

    /// `s*`, the end of the largest balanced prefix, or `None` when every
    /// `s < 1` balances and so does the bottom. A fixed scan finds the first
    /// failing step; bisection then narrows the boundary to [`S_TOLERANCE`]
    /// and returns its balanced side, so every `s ≤ s*` balances.
    fn first_failure(&self, bottom_balances: bool) -> Option<f64> {
        let mut balanced_s = 0.0;
        for k in 1..=SCAN_STEPS {
            let s = f64::from(k) / f64::from(SCAN_STEPS);
            let ok = if k == SCAN_STEPS {
                bottom_balances
            } else {
                self.balances_at(s)
            };
            if !ok {
                return Some(self.bisect(balanced_s, s));
            }
            balanced_s = s;
        }
        None
    }

    fn bisect(&self, mut balanced_s: f64, mut failing_s: f64) -> f64 {
        while failing_s - balanced_s > S_TOLERANCE {
            let mid = f64::midpoint(balanced_s, failing_s);
            if self.balances_at(mid) {
                balanced_s = mid;
            } else {
                failing_s = mid;
            }
        }
        balanced_s
    }
}

/// A frame, with the angles a reader needs read off its own points.
fn frame_of(
    progress: f64,
    shank_deg: f64,
    knee_flexion_deg: f64,
    posture: Posture,
    mechanics: Mechanics,
) -> RangeFrame {
    let trunk = (
        posture.shoulder.x.0 - posture.hip.x.0,
        posture.shoulder.y.0 - posture.hip.y.0,
    );
    let femur = (
        posture.knee.x.0 - posture.hip.x.0,
        posture.knee.y.0 - posture.hip.y.0,
    );
    // The hip angle between the trunk (up) and the femur (down): 180° when
    // they are in line, so flexion is its supplement.
    let between = (trunk.0 * femur.1 - trunk.1 * femur.0)
        .abs()
        .atan2(trunk.0 * femur.0 + trunk.1 * femur.1);
    RangeFrame {
        progress,
        shank_deg,
        knee_flexion_deg,
        hip_flexion_deg: 180.0 - between.to_degrees(),
        trunk_lean_deg: trunk.0.atan2(trunk.1).to_degrees(),
        posture,
        mechanics,
    }
}
