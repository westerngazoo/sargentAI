//! SPEC-0059 §2.2 — the hip's lever arithmetic the squat check shows
//! ("La cuenta"), computed on the server from the same model as the moments.

use fitai_core::biomech::{Body, RangeFrame};
use serde::Serialize;

/// The hip moment split into the bar's share and the body's own share.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) struct HipLever {
    pub(crate) lever_m: f64,
    pub(crate) bar_n: f64,
    pub(crate) bar_nm: f64,
    pub(crate) body_nm: f64,
}

/// The hip lever of `frame`; `None` for a bodyweight-only frame.
pub(crate) fn hip_lever(_body: &Body, _frame: &RangeFrame) -> Option<HipLever> {
    todo!("SPEC-0059 slice 59-a")
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
