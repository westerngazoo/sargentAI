//! SPEC-0045 §2.6 — the v1 baseline squat posture, solved.
//!
//! The baseline is **solved, not measured** (§2.6.1): the lifter's own
//! lengths and load under stated conventions — thigh parallel at the bottom
//! (the depth standard R-0044 AC8 also uses), the ankle at
//! `(−0.26 × foot length, 0.039 H)`, the bar at 0.95 of the trunk line
//! (high-bar), hands on the bar — closed with one balance equation in one
//! unknown, the trunk angle θ (§2.6.3): the combined centre of mass of
//! everything above the ankle, segments and load, over the mid-foot line.
//! Bar-over-mid-foot is recovered as the limit where the bar's mass
//! dominates. Kinematic and closed-form; never interpolation.

use super::{Body, Posture, Rejection};

/// Solve the baseline posture for `body` under `bar_kg`. `None` is
/// bodyweight-only: the model never assumes a bar (R-0045 AC3).
///
/// # Errors
///
/// [`Rejection::DoesNotBalance`] when no trunk angle short of horizontal
/// brings the system centre of mass over the mid-foot line;
/// [`Rejection::AnatomicallyImplausible`] when the solved posture puts a
/// joint past its stated range (SPEC-0045 §2.6.4).
pub fn baseline(_body: &Body, _bar_kg: Option<f64>) -> Result<Posture, Rejection> {
    todo!("R-0045 slice A")
}
