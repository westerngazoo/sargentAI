//! Pose keypoints and the **pure** frame-feature derivation (R-0013, SPEC-0013
//! §2.2).
//!
//! A [`PoseKeypoints`] is the fixed COCO-17 landmark set a pose model emits;
//! [`derive_frame_features`] turns it into a [`FrameFeatures`] — the numeric
//! V-taper proxy plus the geometrically-derivable banded descriptors. Pure: no
//! model, no I/O. The inference that *produces* keypoints lives behind the
//! `api::pose` seam; this module only interprets them.
//!
//! ## Honesty (SPEC-0013 §2.2)
//!
//! 2-D keypoints are a **skeletal-frame proxy** — bony-landmark positions. They
//! cannot recover somatotype, muscle mass, or body-fat, so [`FrameFeatures`]
//! carries geometry only and the banded fields are `Option` (absent, never
//! fabricated, when their keypoints are not confident). There is no `build` /
//! `structure_tags` here to invent.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::archetype::{LengthBand, WidthBand};

/// Keypoints below this confidence score are treated as absent.
///
/// Public since SPEC-0044 §2.10.1: `core::technique` must apply the *same*
/// floor, not a second copy that can drift away from this one.
pub const CONFIDENCE_FLOOR: f32 = 0.2;

/// The library's matchable shoulder-to-waist envelope (mirrors
/// `FrameProfile::SHOULDER_TO_WAIST`); the derived ratio is clamped into it.
const RATIO_MIN: f64 = 1.0;
const RATIO_MAX: f64 = 2.5;

/// A single pose landmark in the model's **normalized, isotropic canvas
/// coordinates** (`0.0..=1.0`) with the model's per-point confidence `score`.
///
/// # The isotropy invariant (SPEC-0044 §2.2.2)
///
/// `x`/`y` are normalized to the *letterboxed square canvas* the model is fed —
/// a single uniform scale plus a translation from source pixels — **not**
/// independently to the source image's width and height. A uniform scale
/// preserves angles and preserves length *ratios*, which is the only reason
/// `distance_to`, [`derive_frame_features`] and every angle in
/// `core::technique` are correct without an aspect correction.
///
/// Any `PoseEstimator` implementation — and any hand-authored test fixture —
/// must emit coordinates in such a space.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Keypoint {
    pub x: f32,
    pub y: f32,
    pub score: f32,
}

impl Keypoint {
    /// Whether the model scored this point at or above [`CONFIDENCE_FLOOR`].
    #[must_use]
    pub fn is_confident(self) -> bool {
        self.score >= CONFIDENCE_FLOOR
    }

    /// Euclidean distance to another keypoint, in normalized canvas units.
    #[must_use]
    pub fn distance_to(self, other: Keypoint) -> f64 {
        let dx = f64::from(self.x) - f64::from(other.x);
        let dy = f64::from(self.y) - f64::from(other.y);
        dx.hypot(dy)
    }

    /// The midpoint of two keypoints (the score is averaged).
    fn midpoint(self, other: Keypoint) -> Keypoint {
        Keypoint {
            x: f32::midpoint(self.x, other.x),
            y: f32::midpoint(self.y, other.y),
            score: f32::midpoint(self.score, other.score),
        }
    }
}

/// The COCO-17 landmarks, in the model's fixed output order.
///
/// The `snake_case` serde tokens are part of the stable on-disk series format
/// (SPEC-0044 §2.10.2) — renaming a variant breaks stored analyses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Landmark {
    Nose,
    LeftEye,
    RightEye,
    LeftEar,
    RightEar,
    LeftShoulder,
    RightShoulder,
    LeftElbow,
    RightElbow,
    LeftWrist,
    RightWrist,
    LeftHip,
    RightHip,
    LeftKnee,
    RightKnee,
    LeftAnkle,
    RightAnkle,
}

impl Landmark {
    /// The landmark's fixed COCO-17 index.
    #[must_use]
    pub fn index(self) -> usize {
        self as usize
    }
}

/// A full COCO-17 pose: the 17 landmarks addressed by name.
///
/// Serializes **transparently** as its bare `[Keypoint; 17]` array — the stored
/// series is 400 poses deep, and a wrapper object per pose would put a field
/// name on disk 400 times to say nothing (SPEC-0044 §2.10.2).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PoseKeypoints {
    points: [Keypoint; 17],
}

impl PoseKeypoints {
    /// Build a pose from the model's 17 keypoints (COCO-17 order).
    #[must_use]
    pub fn new(points: [Keypoint; 17]) -> Self {
        Self { points }
    }

    /// The keypoint for a named landmark.
    #[must_use]
    pub fn get(&self, landmark: Landmark) -> Keypoint {
        self.points[landmark.index()]
    }

    /// All 17 keypoints in COCO-17 order — the accessor iteration and
    /// round-tripping need, since `points` is private (SPEC-0044 §2.10.1).
    #[must_use]
    pub fn points(&self) -> &[Keypoint; 17] {
        &self.points
    }

    /// The mean confidence across all 17 landmarks (the aggregate the derivation
    /// reports).
    fn mean_score(&self) -> f64 {
        const COCO17: u8 = 17;
        let sum: f64 = self.points.iter().map(|k| f64::from(k.score)).sum();
        sum / f64::from(COCO17)
    }
}

/// The matchable query profile derived from a pose — the same vocabulary the
/// archetype library is authored in (SPEC-0013 §2.2). Geometry only; the
/// banded fields are absent (`None`) when their keypoints are not confident.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameFeatures {
    /// The V-taper proxy (shoulder span ÷ hip span), clamped into `1.0..=2.5`.
    pub shoulder_to_waist: f64,
    pub clavicle_width: Option<WidthBand>,
    pub limb_length: Option<LengthBand>,
    /// The aggregate keypoint confidence the derivation rested on (`0.0..=1.0`).
    pub confidence: f64,
}

/// A pose that cannot yield a frame profile — too few confident load-bearing
/// keypoints, or a degenerate (zero-width) hip span.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum FrameError {
    #[error("too few confident keypoints to form the shoulder-to-waist ratio")]
    TooFewConfidentKeypoints,
    #[error("degenerate hip span (zero-width waist proxy)")]
    DegenerateHipSpan,
}

/// Derive the matchable [`FrameFeatures`] from a pose (R-0013 AC1).
///
/// The numeric ratio needs both shoulders and both hips above the confidence
/// floor; the banded fields are produced only when their own keypoints clear it
/// (else `None`). Pure — no model, no I/O.
///
/// # Errors
/// [`FrameError::TooFewConfidentKeypoints`] if a shoulder/hip point is below the
/// floor; [`FrameError::DegenerateHipSpan`] if the hips collapse to a
/// zero-width span (the division guard).
pub fn derive_frame_features(pose: &PoseKeypoints) -> Result<FrameFeatures, FrameError> {
    let left_shoulder = pose.get(Landmark::LeftShoulder);
    let right_shoulder = pose.get(Landmark::RightShoulder);
    let left_hip = pose.get(Landmark::LeftHip);
    let right_hip = pose.get(Landmark::RightHip);

    if ![left_shoulder, right_shoulder, left_hip, right_hip]
        .iter()
        .all(|k| k.is_confident())
    {
        return Err(FrameError::TooFewConfidentKeypoints);
    }

    let shoulder_span = left_shoulder.distance_to(right_shoulder);
    let hip_span = left_hip.distance_to(right_hip);
    if hip_span <= f64::EPSILON {
        return Err(FrameError::DegenerateHipSpan);
    }

    let shoulder_to_waist = (shoulder_span / hip_span).clamp(RATIO_MIN, RATIO_MAX);

    Ok(FrameFeatures {
        shoulder_to_waist,
        clavicle_width: clavicle_width(pose, shoulder_span),
        limb_length: limb_length(
            pose,
            &[left_shoulder, right_shoulder],
            &[left_hip, right_hip],
        ),
        confidence: pose.mean_score(),
    })
}

/// Band the shoulder span against the head width (ear-to-ear) — `None` when the
/// head normalizer is not confident.
fn clavicle_width(pose: &PoseKeypoints, shoulder_span: f64) -> Option<WidthBand> {
    let left_ear = pose.get(Landmark::LeftEar);
    let right_ear = pose.get(Landmark::RightEar);
    if !left_ear.is_confident() || !right_ear.is_confident() {
        return None;
    }
    let head_width = left_ear.distance_to(right_ear);
    if head_width <= f64::EPSILON {
        return None;
    }
    let ratio = shoulder_span / head_width;
    Some(if ratio < 2.3 {
        WidthBand::Narrow
    } else if ratio < 2.9 {
        WidthBand::Average
    } else {
        WidthBand::Wide
    })
}

/// Band the longer of the confident limb chains (a leg, else an arm) against the
/// torso height — `None` when no limb chain is confident.
fn limb_length(
    pose: &PoseKeypoints,
    shoulders: &[Keypoint; 2],
    hips: &[Keypoint; 2],
) -> Option<LengthBand> {
    let torso = shoulders[0]
        .midpoint(shoulders[1])
        .distance_to(hips[0].midpoint(hips[1]));
    if torso <= f64::EPSILON {
        return None;
    }

    let leg = confident_chain(
        pose,
        Landmark::LeftHip,
        Landmark::LeftKnee,
        Landmark::LeftAnkle,
    )
    .or_else(|| {
        confident_chain(
            pose,
            Landmark::RightHip,
            Landmark::RightKnee,
            Landmark::RightAnkle,
        )
    });
    let arm = confident_chain(
        pose,
        Landmark::LeftShoulder,
        Landmark::LeftElbow,
        Landmark::LeftWrist,
    )
    .or_else(|| {
        confident_chain(
            pose,
            Landmark::RightShoulder,
            Landmark::RightElbow,
            Landmark::RightWrist,
        )
    });

    let limb = leg.or(arm)?;
    let ratio = limb / torso;
    Some(if ratio < 1.1 {
        LengthBand::Short
    } else if ratio < 1.5 {
        LengthBand::Average
    } else {
        LengthBand::Long
    })
}

/// The length of a three-joint chain (e.g. hip→knee→ankle) if all three joints
/// clear the confidence floor, else `None`.
fn confident_chain(
    pose: &PoseKeypoints,
    proximal: Landmark,
    middle: Landmark,
    distal: Landmark,
) -> Option<f64> {
    let (a, b, c) = (pose.get(proximal), pose.get(middle), pose.get(distal));
    if a.is_confident() && b.is_confident() && c.is_confident() {
        Some(a.distance_to(b) + b.distance_to(c))
    } else {
        None
    }
}
