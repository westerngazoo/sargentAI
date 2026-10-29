//! SPEC-0059 §2.2 — the hip's lever arithmetic the squat check shows
//! ("La cuenta"), computed on the server from the same model as the moments.
//!
//! `g` is core's [`G`], the one constant every moment is multiplied by: a
//! second value here would break `bar_nm + body_nm == hip moment`.

use fitai_core::biomech::{above, Body, Joint, Posture, RangeFrame, G};
use serde::Serialize;

/// The hip moment split into the bar's share and the body's own share, so
/// that `bar_nm + body_nm` is the frame's hip moment (AC7).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) struct HipLever {
    /// The bar's line minus the hip, horizontally, metres: positive when the
    /// bar is ahead of the hip (toward the toes), as the hip's sign rule reads it.
    pub(crate) lever_m: f64,
    /// The bar's weight, `bar_kg × g`, newtons.
    pub(crate) bar_n: f64,
    /// `bar_n × lever_m`, the bar's share of the hip moment, N·m.
    pub(crate) bar_nm: f64,
    /// The body's own share: every segment above the hip, weight × lever, in
    /// the loaded posture under the stated placement conventions, N·m.
    pub(crate) body_nm: f64,
}

/// The hip lever of `frame` for `body`; `None` for a bodyweight-only frame.
/// Total: never fails, never panics.
pub(crate) fn hip_lever(body: &Body, frame: &RangeFrame) -> Option<HipLever> {
    let load = frame.posture.load?;
    let lever_m = load.at.x.0 - frame.posture.hip.x.0;
    let bar_n = load.kg.get() * G;
    Some(HipLever {
        lever_m,
        bar_n,
        bar_nm: bar_n * lever_m,
        body_nm: body_share_about_hip(body, &frame.posture),
    })
}

/// `Σ m·g·(x_com − x_hip)` over the segments above the hip, in `posture` as it
/// is, load included. This is the segment half of core's hip sum
/// (`chain.rs`, `raw_sum` under `Placement::STATED`), rebuilt from its public
/// pieces in the same order, because core exposes only the total.
///
/// Not `moment_about(Hip, body, posture without its load)` as SPEC-0059 §2.2
/// first wrote: core places the arms' centre of mass at the shoulder → bar
/// midpoint when there is a bar and hanging straight down when there is not,
/// so dropping the load also moves the arms and the shares stop summing to
/// the hip moment.
fn body_share_about_hip(body: &Body, posture: &Posture) -> f64 {
    let hip_x = posture.hip.x.0;
    above(Joint::Hip)
        .iter()
        .map(|&seg| body.mass_kg(seg) * G * (seg.com(body, posture).x.0 - hip_x))
        .sum()
}

/// What the floor pushes up with, `(bar_kg + mass_kg) × g`, newtons.
pub(crate) fn ground_n(bar_kg: Option<f64>, mass_kg: f64) -> f64 {
    (bar_kg.unwrap_or(0.0) + mass_kg) * G
}

#[cfg(test)]
mod tests {
    use fitai_core::biomech::{
        scale, squat_range, Dorsiflexion, FrameCount, Joint, MeasuredLengths, Metres,
    };
    use fitai_core::workout::LoadKg;

    use super::hip_lever;

    /// AC7 — per frame, `bar_nm + body_nm` is the hip joint moment to 1e-9:
    /// the sum the check shows is the model's arithmetic, not a residual.
    #[test]
    fn bar_share_plus_body_share_is_the_hip_moment() {
        let body = scale(Metres(1.78), 77.0, &MeasuredLengths::default());
        let bar = LoadKg::try_new(84.0).unwrap();
        let ankle = Dorsiflexion::from_degrees(29.0).unwrap();
        let range = squat_range(&body, Some(bar), ankle, FrameCount::new(41).unwrap()).unwrap();
        assert!(!range.frames.is_empty());
        for frame in &range.frames {
            let hip = hip_lever(&body, frame).expect("a loaded frame has a lever");
            let joint = &frame.mechanics.joints[0];
            assert_eq!(joint.joint, Joint::Hip);
            assert!(
                (hip.bar_nm + hip.body_nm - joint.moment.0).abs() < 1e-9,
                "progress {}: {} + {} != {}",
                frame.progress,
                hip.bar_nm,
                hip.body_nm,
                joint.moment.0
            );
            assert!((hip.bar_nm - hip.bar_n * hip.lever_m).abs() < 1e-12);
        }
    }

    /// A bodyweight-only frame has no bar, so no lever.
    #[test]
    fn bodyweight_only_has_no_lever() {
        let body = scale(Metres(1.78), 77.0, &MeasuredLengths::default());
        let ankle = Dorsiflexion::from_degrees(29.0).unwrap();
        let range = squat_range(&body, None, ankle, FrameCount::new(5).unwrap()).unwrap();
        assert!(range.frames.iter().all(|f| hip_lever(&body, f).is_none()));
    }
}
