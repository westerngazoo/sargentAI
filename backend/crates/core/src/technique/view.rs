//! View detection: four cues, two thresholds, a deliberately wide refusal
//! band, median classification and a stability check (SPEC-0044 §2.5).
//!
//! **This is the one place that deliberately uses both sides of a bilateral
//! pair** — the foreshortening *is* the signal. Everywhere else the near-side
//! rule of `geometry::resolve` applies.

use crate::pose::{Landmark, PoseKeypoints};

use super::geometry::{midpoint, DEGENERATE_LENGTH};
use super::segment::Stance;
use super::{
    median, CameraView, LiftSeries, Refusal, ViewClass, EAR_RATIO_FRONT_MIN, EAR_RATIO_SIDE_MAX,
    HIP_VIEW_FRONT_MIN, HIP_VIEW_SIDE_MAX, MAX_UNSTABLE_FRACTION, NOSE_OFFSET_FRONT_MAX,
    NOSE_OFFSET_SIDE_MIN, VIEW_FRONT_MIN, VIEW_SIDE_MAX,
};

/// The four per-frame view cues.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Cues {
    /// The primary cue, `shoulder_span / torso_length`.
    ///
    /// The shoulder span is a frontal-plane segment, so it images as
    /// `S·cos θ`; the torso lies along the body's long axis and a yaw is a
    /// rotation *about* that axis, so its image length is unchanged. The ratio
    /// is therefore a clean cosine — maximal front-on, collapsing to zero
    /// side-on.
    ///
    /// The torso length is **Euclidean, and that is the point**: a *vertical*
    /// torso measure would collapse when the lifter pitches forward, which is
    /// the deadlift setup's whole posture.
    pub(crate) shoulder_ratio: f64,
    /// The corroborating cosine cue, `hip_span / torso_length`. One ratio is
    /// not enough — a mis-detected shoulder swings it.
    pub(crate) hip_ratio: f64,
    /// `min(ear score) / max(ear score)`. **Orthogonal to the span cues**: it
    /// comes from occlusion, not foreshortening, so it fails differently.
    pub(crate) ear_score_ratio: f64,
    /// `|nose.x − shoulder_mid.x| / torso_length`. Also orthogonal — head
    /// geometry, not spans.
    pub(crate) nose_offset: f64,
}

/// The four cues for one frame, or `None` when the frame has no usable torso to
/// normalize by.
pub(crate) fn cues(pose: &PoseKeypoints) -> Option<Cues> {
    let left_shoulder = pose.get(Landmark::LeftShoulder);
    let right_shoulder = pose.get(Landmark::RightShoulder);
    let left_hip = pose.get(Landmark::LeftHip);
    let right_hip = pose.get(Landmark::RightHip);

    let shoulder_mid = midpoint(left_shoulder, right_shoulder);
    let hip_mid = midpoint(left_hip, right_hip);
    let torso = shoulder_mid.distance_to(hip_mid);
    if torso <= DEGENERATE_LENGTH {
        return None;
    }

    let nose = pose.get(Landmark::Nose);
    Some(Cues {
        shoulder_ratio: left_shoulder.distance_to(right_shoulder) / torso,
        hip_ratio: left_hip.distance_to(right_hip) / torso,
        ear_score_ratio: ear_score_ratio(pose),
        nose_offset: f64::from((nose.x - shoulder_mid.x).abs()) / torso,
    })
}

/// Side-on, the far ear is behind the head and the model scores it near zero;
/// front-on, both ears score similarly.
fn ear_score_ratio(pose: &PoseKeypoints) -> f64 {
    let left = f64::from(pose.get(Landmark::LeftEar).score);
    let right = f64::from(pose.get(Landmark::RightEar).score);
    let high = left.max(right);
    if high <= DEGENERATE_LENGTH {
        return 0.0;
    }
    left.min(right) / high
}

/// Classify one frame's cues.
///
/// **Two thresholds, and everything between them refuses.** A single cut would
/// classify a 45° camera as *something*. The band spans roughly yaw 28°–69°,
/// and that width is intentional: `S/T` varies between people, so the primary
/// ratio is `(this person's S/T) · cos θ` with the person's constant unknown. A
/// narrow band would trade a real refusal for a person-dependent guess — which
/// is exactly what supplying a `Calibration` would fix.
///
/// A verdict needs the primary cue, **agreement** from the second cosine cue,
/// and **support** from at least one of the two orthogonal cues.
pub(crate) fn classify(cues: &Cues) -> ViewClass {
    let side = cues.shoulder_ratio <= VIEW_SIDE_MAX
        && cues.hip_ratio <= HIP_VIEW_SIDE_MAX
        && (cues.ear_score_ratio <= EAR_RATIO_SIDE_MAX || cues.nose_offset >= NOSE_OFFSET_SIDE_MIN);
    if side {
        return ViewClass::Side;
    }
    let front = cues.shoulder_ratio >= VIEW_FRONT_MIN
        && cues.hip_ratio >= HIP_VIEW_FRONT_MIN
        && (cues.ear_score_ratio >= EAR_RATIO_FRONT_MIN
            || cues.nose_offset <= NOSE_OFFSET_FRONT_MAX);
    if front {
        return ViewClass::Front;
    }
    ViewClass::Indeterminate
}

/// Step 6 — classify on the quiet window's median, then require the whole clip
/// to agree (SPEC-0044 §2.5.4).
///
/// Per-frame cues are noisy and, mid-rep, genuinely change: a squatting
/// lifter's torso pitches. So the verdict is taken once, on the median of each
/// cue over the one stretch where posture is defined — and then every frame is
/// classified independently and required to match.
///
/// **Unstable classification is itself a refusal.** A clip that reads Side for
/// its first half and Front for its second is a panned camera or a rotating
/// lifter; either way there is no single view the geometry can assume, and
/// assuming one would be a confident wrong verdict.
pub(crate) fn classify_and_check_stability(
    series: &LiftSeries,
    stance: &Stance,
) -> Result<(), Refusal> {
    let window: Vec<Cues> = series.frames[stance.window.clone()]
        .iter()
        .filter_map(|f| cues(&f.pose))
        .collect();
    let reference = median_cues(&window);
    let looks_like = reference.map_or(ViewClass::Indeterminate, |c| classify(&c));

    let expected_class = match series.view {
        CameraView::Side => ViewClass::Side,
        CameraView::Front => ViewClass::Front,
    };
    if looks_like != expected_class {
        let c = reference.unwrap_or(Cues {
            shoulder_ratio: 0.0,
            hip_ratio: 0.0,
            ear_score_ratio: 0.0,
            nose_offset: 0.0,
        });
        return Err(Refusal::WrongView {
            expected: series.view,
            looks_like,
            shoulder_ratio: c.shoulder_ratio,
            hip_ratio: c.hip_ratio,
            ear_score_ratio: c.ear_score_ratio,
        });
    }

    let (mut side, mut front, mut indeterminate) = (0_u32, 0_u32, 0_u32);
    for frame in &series.frames {
        match cues(&frame.pose).map_or(ViewClass::Indeterminate, |c| classify(&c)) {
            ViewClass::Side => side += 1,
            ViewClass::Front => front += 1,
            ViewClass::Indeterminate => indeterminate += 1,
        }
    }
    let total = side + front + indeterminate;
    let agreeing = match looks_like {
        ViewClass::Side => side,
        ViewClass::Front => front,
        ViewClass::Indeterminate => indeterminate,
    };
    if total > 0 && f64::from(total - agreeing) / f64::from(total) > MAX_UNSTABLE_FRACTION {
        return Err(Refusal::UnstableView {
            side,
            front,
            indeterminate,
        });
    }
    Ok(())
}

/// The per-cue median over the window. Each cue is taken independently: a
/// single mis-detected shoulder must not drag the ear cue with it.
fn median_cues(window: &[Cues]) -> Option<Cues> {
    if window.is_empty() {
        return None;
    }
    let pick = |f: fn(&Cues) -> f64| median(&window.iter().map(f).collect::<Vec<_>>());
    Some(Cues {
        shoulder_ratio: pick(|c| c.shoulder_ratio),
        hip_ratio: pick(|c| c.hip_ratio),
        ear_score_ratio: pick(|c| c.ear_score_ratio),
        nose_offset: pick(|c| c.nose_offset),
    })
}
