//! SPEC-0045 §2.4.2 — the squat's lumped segments and where their masses act,
//! under the stated placement conventions.
//!
//! The moments themselves are the kernel's (SPEC-0066 slice 66-a2): this
//! module says *what* weighs and *where*, `kernel` and garust say what each
//! joint holds. Before 66-a2 this was `chain.rs`, which also summed the
//! moments by hand; the parity fixture pins that nothing moved.

use super::{along, Body, Metres, Point, Posture, L5S1_UP_THE_TRUNK, WINTER};

/// A lumped mass on the load side of a joint (SPEC-0045 §2.4.2). `Shank` and
/// `Thigh` are both legs and `Arms` both arms — the sagittal model
/// superimposes the pairs. The external load is not a segment; it travels on
/// [`Posture`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChainSegment {
    /// Knee → ankle, both legs.
    Shank,
    /// Hip → knee, both legs.
    Thigh,
    /// L4-L5 → greater trochanter; on the hip's load side in full, a stated
    /// convention with its error term.
    Pelvis,
    /// C7-T1 → L4-L5.
    ThoraxAbdomen,
    /// Placed by convention on the trunk line beyond the shoulder.
    HeadNeck,
    /// Both arms: on the bar when there is one, hanging when there is not.
    Arms,
}

impl ChainSegment {
    /// Every lumped segment above the ankle, in chain order — what the
    /// balance solve sums over and the squat tree hangs (SPEC-0045 §2.4.2).
    ///
    /// **What the solves rely on.** `Σ m·x` is affine in the sine of each
    /// angle the solves vary, because every placement here reads only `x`s
    /// that are themselves affine in those sines, and the trunk's per-unit
    /// coefficient does not depend on the legs. `squat::Balance::of`,
    /// `range::upright_lean` and the smoothness behind `range::SCAN_STEPS` all
    /// assume it. A placement that reads a `y`, or a knob that makes the
    /// trunk's coefficient depend on the legs (an arm angle, say), breaks it
    /// quietly; check them before adding one.
    pub const ALL: [Self; 6] = [
        Self::Shank,
        Self::Thigh,
        Self::Pelvis,
        Self::ThoraxAbdomen,
        Self::HeadNeck,
        Self::Arms,
    ];

    /// Where this segment's centre of mass sits in `p`, by the §2.3 COM
    /// fractions and the §2.4.2 placement conventions: the head & neck on the
    /// trunk line beyond the shoulder; the arms at the shoulder → load
    /// midpoint when there is a load, and hanging straight down from the
    /// shoulder, at the whole arm's centre of mass, when there is not.
    #[must_use]
    pub fn com(self, body: &Body, p: &Posture) -> Point {
        self.com_with(body, p, Placement::STATED)
    }

    /// [`ChainSegment::com`] with the placement conventions nudged. Every
    /// caller but [`Placement::STATED`] is the uncertainty walk.
    pub(super) fn com_with(self, body: &Body, p: &Posture, placement: Placement) -> Point {
        let l5s1 = l5s1_point(p, placement.l5s1);
        match self {
            Self::Shank => along(p.knee, p.ankle, WINTER.shank.com_from_proximal),
            Self::Thigh => along(p.hip, p.knee, WINTER.thigh.com_from_proximal),
            Self::Pelvis => along(l5s1, p.hip, WINTER.pelvis.com_from_proximal),
            Self::ThoraxAbdomen => along(p.shoulder, l5s1, WINTER.thorax_abdomen.com_from_proximal),
            Self::HeadNeck => {
                let beyond = WINTER.head_neck.com_from_proximal * body.head_neck();
                offset(p.shoulder, unit(p.hip, p.shoulder), beyond + placement.head)
            }
            Self::Arms => {
                if let Some(load) = p.load {
                    let midpoint = along(p.shoulder, load.at, 0.5);
                    offset(midpoint, unit(p.shoulder, load.at), placement.arms)
                } else {
                    offset(
                        p.shoulder,
                        (0.0, -1.0),
                        whole_arm_hang(body) + placement.arms,
                    )
                }
            }
        }
    }
}

/// How far below the shoulder a straight, hanging arm's centre of mass sits:
/// the mass-weighted mean of the upper arm, forearm and hand, each at its own
/// Table 4.1 centre along the arm (SPEC-0045 §2.4.2).
///
/// The draft placed the whole arm at the *upper arm's* centre — 0.436 of the
/// upper-arm length, 14 cm below the shoulder at 1.75 m — when the arm's is
/// 30 cm. No moment in slice A moved when this was corrected, because a
/// vertically hanging arm has the shoulder's `x` at any depth; the depth only
/// starts to matter the moment the arms do anything but hang.
fn whole_arm_hang(body: &Body) -> f64 {
    let (upper, fore, hand) = (body.upper_arm(), body.forearm(), body.hand());
    let rows = [
        (
            WINTER.upper_arm.mass_fraction,
            WINTER.upper_arm.com_from_proximal * upper,
        ),
        (
            WINTER.forearm.mass_fraction,
            upper + WINTER.forearm.com_from_proximal * fore,
        ),
        (
            WINTER.hand.mass_fraction,
            upper + fore + WINTER.hand.com_from_proximal * hand,
        ),
    ];
    let mass: f64 = rows.iter().map(|&(m, _)| m).sum();
    rows.iter().map(|&(m, depth)| m * depth).sum::<f64>() / mass
}

/// The §2.4.2 and §2.5 placement conventions, as metres a caller can nudge
/// along the line each convention places its point on.
///
/// They exist as a parameter rather than as literals so the model and the
/// bound on what the conventions cost are computed by **the same code**
/// (SPEC-0045 §2.8). A second copy of the placement rule, used only for the
/// error bar, would be free to drift away from the one that produces the
/// number.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Placement {
    /// Along the trunk line, added to the head & neck COM.
    pub head: f64,
    /// Along the arms' own placement line — shoulder → bar when there is one,
    /// straight down when there is not.
    pub arms: f64,
    /// Along the trunk line, added to the L5/S1 point. It moves the lumbar
    /// row's joint centre **and** the pelvis COM, which sits on every load
    /// side, so it is not a lumbar-only spread.
    pub l5s1: f64,
}

impl Placement {
    /// The conventions as §2.4.2 and §2.5 state them: no nudge.
    pub(super) const STATED: Self = Self {
        head: 0.0,
        arms: 0.0,
        l5s1: 0.0,
    };
}

/// The L5/S1 point: the §2.5 convention, nudged by `offset_m` along the trunk
/// line. One home for the placement (SPEC-0045 §2.5).
pub(super) fn l5s1_point(p: &Posture, offset_m: f64) -> Point {
    offset(p.l5s1(L5S1_UP_THE_TRUNK), unit(p.hip, p.shoulder), offset_m)
}

/// The unit vector from `from` to `to`; `(0, 0)` for a degenerate segment, so
/// a zero-length trunk nudges nothing rather than producing `NaN`.
fn unit(from: Point, to: Point) -> (f64, f64) {
    let (dx, dy) = (to.x.0 - from.x.0, to.y.0 - from.y.0);
    let length = dx.hypot(dy);
    if length == 0.0 {
        (0.0, 0.0)
    } else {
        (dx / length, dy / length)
    }
}

/// `point` moved `distance` metres along the unit vector `direction`.
fn offset(point: Point, direction: (f64, f64), distance: f64) -> Point {
    Point {
        x: Metres(point.x.0 + direction.0 * distance),
        y: Metres(point.y.0 + direction.1 * distance),
    }
}
