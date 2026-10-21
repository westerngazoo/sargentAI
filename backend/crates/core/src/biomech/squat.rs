//! SPEC-0045 §2.6 — the v1 baseline squat posture, solved.
//!
//! The baseline is **solved, not measured** (§2.6.1): the lifter's own lengths
//! and load under stated conventions — thigh parallel at the bottom (the depth
//! standard R-0044 AC8 also uses), the ankle at
//! `(−0.26 × foot length, 0.039 H)`, the shank at the lifter's own
//! dorsiflexion (§2.6.1.1), the bar at 0.95 of the trunk line (high-bar),
//! hands on the bar — closed with one balance equation in one unknown, the
//! trunk angle θ (§2.6.3): the combined centre of mass of everything above the
//! ankle, segments and load, over the mid-foot line. Bar-over-mid-foot is
//! recovered as the limit where the bar's mass dominates.
//!
//! ## Why it is closed-form in two variables, not one
//!
//! Every load-side `x` is `hip_x + k·trunk·sin θ` for a fixed `k`, and
//! `hip_x = ankle_x + shank·sin δ − femur` is itself affine in `sin δ`. So the
//! first moment about the mid-foot line is **jointly affine, with no cross
//! term**:
//!
//! ```text
//! Σ m·x = P + Q·sin δ + B·sin θ ,     Q > 0,  B > 0
//! ```
//!
//! and balance gives `sin θ = −(P + Q·sin δ)/B` directly — never a search,
//! never an interpolation between measured templates, which would put postures
//! the lifter never held into the model.
//!
//! Because it is affine, `P`, `Q` and `B` are **measured off the model rather
//! than re-derived**: three probe postures through the same
//! [`ChainSegment::com`] the moments use. Writing the `k`-fractions out a
//! second time here would have given §2.4.2 two homes free to drift apart —
//! the thing folding `lumbar.rs` away was meant to prevent.
//!
//! Requiring `sin θ ∈ [0, 1]` — the trunk between vertical and horizontal —
//! turns into a window in the lifter's **own ankle**: `u_min` is the least
//! dorsiflexion that reaches this depth under this load, `u_max` the point
//! past which they would have to lean backwards. That window is what decides
//! between the two rejections, and neither end contains a constant anyone has
//! to cite (§2.6.4).

use crate::workout::LoadKg;

use super::{
    above, along, analyse_baseline, Body, ChainSegment, Dorsiflexion, Joint, Load, Mechanics,
    Metres, Point, Posture, Rejection, Unknown,
};

/// How far behind the mid-foot line the ankle sits, as a fraction of foot
/// length — mid-foot at half the foot, so the ankle lands 0.24 of a foot
/// length ahead of the heel. A **stated convention** (SPEC-0045 §2.6.1); it
/// replaces `muneco.py`'s `−0.04 m` literal, which had no source.
pub const ANKLE_BEHIND_MID_FOOT: f64 = 0.26;

/// Where the bar attaches on the trunk line, as a fraction of hip → shoulder.
/// High-bar; a **stated convention** (SPEC-0045 §2.6.1). Low-bar is a knob in
/// slice B, not a different lift.
pub const HIGH_BAR: f64 = 0.95;

/// A posture the v1 parametric solve produced, and the evidence that it did.
///
/// The wrapper is the whole point: [`analyse_baseline`] asserts
/// `SolvedBaseline` and `SinglePosture` on its result, and those are claims
/// about where a posture *came from*, which a bare [`Posture`] cannot carry
/// (SPEC-0045 §2.1). Only [`baseline`] can build one, so the claim cannot be
/// made about a posture that was measured or hand-built. When the R-0044
/// source lands it gets its own wrapper and its own door.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Baseline(Posture);

impl Baseline {
    /// The posture itself.
    #[must_use]
    pub fn posture(&self) -> &Posture {
        &self.0
    }
}

/// Solve the baseline posture for `body` under `bar_kg`, at `ankle`
/// dorsiflexion. `None` is bodyweight-only: the model never assumes a bar
/// (R-0045 AC3).
///
/// # Errors
///
/// [`Rejection::DepthNotReachable`] when the lifter's ankle is short of the
/// `u_min` this lift needs — *your ankle is what stops you*, with the
/// dorsiflexion it would take. [`Rejection::DoesNotBalance`] when no trunk
/// angle between vertical and horizontal balances the system centre of mass
/// over the mid-foot line at **any** dorsiflexion in `0°..=90°`, which is a
/// statement about their proportions and load rather than their ankles.
pub fn baseline(
    body: &Body,
    bar_kg: Option<LoadKg>,
    ankle: Dorsiflexion,
) -> Result<Baseline, Rejection> {
    let balance = Balance::of(body, bar_kg);
    let window = balance.window().ok_or(Rejection::DoesNotBalance {
        unknown: Unknown::TrunkAngle,
    })?;
    let u = ankle.sin();

    if u < window.least {
        // Below the window: more ankle would help. Whether *enough* ankle
        // exists in `0°..=90°` is what separates the two rejections — and
        // `90°` there is the geometric domain of a shank angle from vertical,
        // not a claim about anyone's ankle.
        return Err(if window.least <= 1.0 {
            Rejection::DepthNotReachable {
                limiting: Joint::Ankle,
                needed_deg: window.least.asin().to_degrees(),
                available_deg: ankle.degrees(),
            }
        } else {
            Rejection::DoesNotBalance {
                unknown: Unknown::TrunkAngle,
            }
        });
    }
    if u > window.most {
        // Above it: the knee is so far forward that the lifter would have to
        // lean *backwards* to stay over the mid-foot. More ankle cannot fix
        // what too much ankle caused.
        return Err(Rejection::DoesNotBalance {
            unknown: Unknown::TrunkAngle,
        });
    }

    Ok(Baseline(pose(
        body,
        bar_kg,
        u,
        balance.sin_theta(u).clamp(0.0, 1.0),
    )))
}

/// Solve this lifter's baseline and analyse it — the parametric source and the
/// model in one call (SPEC-0045 §2.8).
///
/// # Errors
///
/// Whatever [`baseline`] rejects. Once a posture exists it has moments: the
/// analysis itself is total.
pub fn baseline_mechanics(
    body: &Body,
    bar_kg: Option<LoadKg>,
    ankle: Dorsiflexion,
) -> Result<Mechanics, Rejection> {
    let solved = baseline(body, bar_kg, ankle)?;
    Ok(analyse_baseline(body, &solved))
}

/// The §2.6.1 ankle: behind the mid-foot line by a fraction of the foot, at
/// the table's ankle height.
fn ankle_point(body: &Body) -> Point {
    Point {
        x: Metres(-ANKLE_BEHIND_MID_FOOT * body.foot()),
        y: Metres(body.ankle_height()),
    }
}

/// The §2.6.1 geometry at a given shank and trunk angle, both as sines: the
/// ankle placed, the knee out along the shank, the thigh parallel with the hip
/// a femur behind the knee, the trunk leaning, and the bar — if there is one —
/// at [`HIGH_BAR`] along the trunk line.
///
/// The one home for the baseline's shape. The solve uses it to probe, and then
/// again to build what it returns, so a probe can never describe a posture the
/// solve would not produce.
fn pose(body: &Body, bar_kg: Option<LoadKg>, sin_shank: f64, sin_trunk: f64) -> Posture {
    let cos = |sin: f64| sin.mul_add(-sin, 1.0).max(0.0).sqrt();
    let ankle = ankle_point(body);
    let knee = Point {
        x: Metres(ankle.x.0 + body.shank() * sin_shank),
        y: Metres(ankle.y.0 + body.shank() * cos(sin_shank)),
    };
    // Parallel: the femur is horizontal, so the hip is at knee height, behind
    // it by a femur.
    let hip = Point {
        x: Metres(knee.x.0 - body.thigh()),
        y: knee.y,
    };
    let shoulder = Point {
        x: Metres(hip.x.0 + body.trunk() * sin_trunk),
        y: Metres(hip.y.0 + body.trunk() * cos(sin_trunk)),
    };
    Posture {
        ankle,
        knee,
        hip,
        shoulder,
        load: bar_kg.map(|kg| Load {
            kg,
            at: along(hip, shoulder, HIGH_BAR),
        }),
    }
}

/// `Σ m·x` about the mid-foot line over everything above the ankle, plus the
/// load: the quantity balance sets to zero (SPEC-0045 §2.6.3). Computed
/// through the same placement conventions as every moment.
fn first_moment(body: &Body, p: &Posture) -> f64 {
    let segments: f64 = above(Joint::Ankle)
        .iter()
        .map(|&seg: &ChainSegment| body.mass_kg(seg) * seg.com(body, p).x.0)
        .sum();
    segments + p.load.map_or(0.0, |l| l.kg.get() * l.at.x.0)
}

/// `Σ m·x = p + q·sin δ + b·sin θ` about the mid-foot line, for this lifter
/// under this load. Every coefficient is their own lengths and the table's
/// mass fractions; none of them is a constant anyone has to cite.
struct Balance {
    /// The part that depends on neither angle.
    p: f64,
    /// Per unit `sin δ` — strictly positive, so more ankle always moves the
    /// system forward.
    q: f64,
    /// Per unit `sin θ` — strictly positive, so more lean always does too.
    b: f64,
}

/// The dorsiflexion window this lift leaves, in `sin δ` (SPEC-0045 §2.6.3).
struct Window {
    /// `u_min`: below it the trunk would have to pass horizontal.
    least: f64,
    /// `u_max`: above it the trunk would have to lean backwards.
    most: f64,
}

impl Balance {
    /// Measured off the model in three probes. `Σ m·x` is affine in
    /// `(sin δ, sin θ)` with no cross term, so its value at the origin and at
    /// each unit vector determines it everywhere — exactly, not numerically.
    fn of(body: &Body, bar_kg: Option<LoadKg>) -> Self {
        let at =
            |sin_shank, sin_trunk| first_moment(body, &pose(body, bar_kg, sin_shank, sin_trunk));
        let p = at(0.0, 0.0);
        Self {
            p,
            q: at(1.0, 0.0) - p,
            b: at(0.0, 1.0) - p,
        }
    }

    /// The admissible `sin δ`, or `None` when the trunk angle is not
    /// determined at all — a body with no mass on the trunk line and no bar,
    /// for which the balance equation is `0 = 0` and picking a lean would be
    /// inventing a posture.
    fn window(&self) -> Option<Window> {
        (self.b > 0.0 && self.q > 0.0).then(|| Window {
            least: (-self.b - self.p) / self.q,
            most: -self.p / self.q,
        })
    }

    /// The trunk lean that balances this lifter at `u = sin δ`.
    fn sin_theta(&self, u: f64) -> f64 {
        -self.q.mul_add(u, self.p) / self.b
    }
}
