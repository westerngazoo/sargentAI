//! Unit tests for the `fitai_core::pose` domain — pose keypoints and the **pure**
//! frame-feature derivation (SPEC-0013 §2.2): `Keypoint`, the COCO-17
//! `PoseKeypoints` (`Landmark` enum + indexed access), `FrameFeatures`, and
//! `derive_frame_features(&PoseKeypoints) -> Result<FrameFeatures, FrameError>`.
//!
//! Authored by the qa agent during R-0013 step 3 (test planning), BEFORE the
//! `core::pose` module exists. Pre-implementation red state = compile failure
//! (the module / `Landmark` / `Keypoint` / `PoseKeypoints` / `FrameFeatures` /
//! `derive_frame_features` / `FrameError` are all absent). Implementation step 5
//! makes these green. No model is in the loop here — keypoints are hand-authored,
//! exactly as AC1/AC8 require ("unit-tested from fixed keypoints with no model").
//!
//! SAC → AC → test traceability (the full table lives in the qa sign-off report):
//! - SAC1 → AC1: `derive_frame_features` turns fixed keypoints into a validated
//!   `FrameFeatures` — the numeric `shoulder_to_waist` from a known
//!   wide-shoulder/narrow-hip geometry; the banded `clavicle_width`/`limb_length`
//!   derived only when their keypoints clear the confidence floor (else `None`,
//!   never fabricated); and a typed `FrameError` for degenerate input
//!   (too-few-confident keypoints, a zero/near-zero hip span). The honesty
//!   constraint: a 2-D photo yields geometry — there is no `build`/`structure_tags`
//!   on `FrameFeatures` to fabricate.
//!
//! Builders mirror R-0012's `valid_*` golden-input style: `valid_pose()` is the
//! fully-confident wide-frame golden case each rejection test mutates off.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Banded thresholds and ratio checks round-trip f32/f64 coordinates through the
// pure derivation; exact-equality assertions on authored values are correct here,
// and approximate comparisons use an explicit epsilon.
#![allow(clippy::float_cmp)]
// Test doc comments quote field names and the COCO-17 landmark vocabulary as prose.
#![allow(clippy::doc_markdown)]

use fitai_core::archetype::{LengthBand, WidthBand};
use fitai_core::pose::{
    derive_frame_features, FrameError, FrameFeatures, Keypoint, Landmark, PoseKeypoints,
};

// ===========================================================================
// Builders — a hand-authored COCO-17 skeleton in normalized image coordinates
// (0.0..=1.0). The golden frame is a clear wide-shoulder / narrow-hip "V":
// shoulders span 0.30..0.70 (width 0.40) and hips span 0.43..0.57 (width 0.14),
// so the shoulder ÷ hip span ratio is a strong taper. Every point is fully
// confident; the rejection tests dim or collapse specific points.
// ===========================================================================

/// A confident keypoint at the given normalized coordinate.
fn kp(x: f32, y: f32) -> Keypoint {
    Keypoint { x, y, score: 0.9 }
}

/// A keypoint placed where the golden one is, but below the confidence floor —
/// the derivation must treat it as absent.
fn dim(x: f32, y: f32) -> Keypoint {
    Keypoint { x, y, score: 0.05 }
}

/// COCO-17 order (the model's fixed output order):
/// 0 nose, 1 left_eye, 2 right_eye, 3 left_ear, 4 right_ear,
/// 5 left_shoulder, 6 right_shoulder, 7 left_elbow, 8 right_elbow,
/// 9 left_wrist, 10 right_wrist, 11 left_hip, 12 right_hip,
/// 13 left_knee, 14 right_knee, 15 left_ankle, 16 right_ankle.
///
/// Builds the golden wide-frame pose; `overrides` lets a test replace specific
/// landmarks (e.g. dim the shoulders) without re-authoring the whole skeleton.
fn pose_with(overrides: &[(Landmark, Keypoint)]) -> PoseKeypoints {
    // A plausible standing figure, head-to-toe, in normalized coordinates.
    let mut points: [Keypoint; 17] = [
        kp(0.50, 0.08), // nose
        kp(0.47, 0.06), // left_eye
        kp(0.53, 0.06), // right_eye
        kp(0.44, 0.07), // left_ear
        kp(0.56, 0.07), // right_ear
        kp(0.30, 0.22), // left_shoulder  — wide
        kp(0.70, 0.22), // right_shoulder — wide
        kp(0.26, 0.40), // left_elbow
        kp(0.74, 0.40), // right_elbow
        kp(0.24, 0.55), // left_wrist
        kp(0.76, 0.55), // right_wrist
        kp(0.43, 0.55), // left_hip       — narrow
        kp(0.57, 0.55), // right_hip      — narrow
        kp(0.42, 0.75), // left_knee
        kp(0.58, 0.75), // right_knee
        kp(0.41, 0.95), // left_ankle
        kp(0.59, 0.95), // right_ankle
    ];
    for (landmark, point) in overrides {
        points[landmark_index(*landmark)] = *point;
    }
    PoseKeypoints::new(points)
}

/// The COCO-17 index of a landmark — the test's own copy of the ordering, so the
/// builder can poke a single named point. The production `Landmark`-indexed
/// `get()` is what the derivation uses; this is test scaffolding only.
fn landmark_index(landmark: Landmark) -> usize {
    match landmark {
        Landmark::Nose => 0,
        Landmark::LeftEye => 1,
        Landmark::RightEye => 2,
        Landmark::LeftEar => 3,
        Landmark::RightEar => 4,
        Landmark::LeftShoulder => 5,
        Landmark::RightShoulder => 6,
        Landmark::LeftElbow => 7,
        Landmark::RightElbow => 8,
        Landmark::LeftWrist => 9,
        Landmark::RightWrist => 10,
        Landmark::LeftHip => 11,
        Landmark::RightHip => 12,
        Landmark::LeftKnee => 13,
        Landmark::RightKnee => 14,
        Landmark::LeftAnkle => 15,
        Landmark::RightAnkle => 16,
    }
}

/// The fully-confident golden pose — every rejection/absence test mutates off it.
fn valid_pose() -> PoseKeypoints {
    pose_with(&[])
}

// ===========================================================================
// PoseKeypoints — COCO-17 named landmark access (SPEC-0013 §2.2).
// ===========================================================================

#[test]
fn pose_keypoints_resolves_named_landmarks() {
    let pose = valid_pose();
    // `get(Landmark)` reads the named joint, not a magic index.
    let left_shoulder = pose.get(Landmark::LeftShoulder);
    let right_shoulder = pose.get(Landmark::RightShoulder);
    assert_eq!(left_shoulder.x, 0.30);
    assert_eq!(right_shoulder.x, 0.70);
    assert_eq!(pose.get(Landmark::Nose).y, 0.08);
    assert_eq!(pose.get(Landmark::RightAnkle).x, 0.59);
}

// ===========================================================================
// SAC1 / AC1: the numeric shoulder_to_waist from a known wide/narrow geometry.
// shoulder span = |0.70 - 0.30| = 0.40; hip span = |0.57 - 0.43| = 0.14;
// ratio = 0.40 / 0.14 ≈ 2.857, clamped into the library's matchable envelope.
// The exact production normalization is the impl's, but the ordering property —
// a wide-shoulder/narrow-hip frame yields a HIGH ratio — must hold and the value
// must land in the documented 1.0..=2.5 band.
// ===========================================================================

#[test]
fn derive_produces_a_high_ratio_for_a_wide_shoulder_narrow_hip_frame() {
    let features = derive_frame_features(&valid_pose())
        .expect("a confident wide-frame pose must derive features");

    // The V-taper proxy is in the library's matchable band (SPEC-0013 §2.2:
    // "in the library's 1.0..=2.5 band").
    assert!(
        (1.0..=2.5).contains(&features.shoulder_to_waist),
        "shoulder_to_waist {} must fall in the documented matchable band [1.0, 2.5]",
        features.shoulder_to_waist
    );
    // A strongly tapered frame must read as a high ratio — comfortably above the
    // midpoint of the band (1.75). This pins the *direction* of the geometry, not
    // the exact normalization constant.
    assert!(
        features.shoulder_to_waist >= 1.75,
        "a wide-shoulder/narrow-hip frame must yield a high V-taper ratio, got {}",
        features.shoulder_to_waist
    );
    // The aggregate confidence the derivation rested on is reported and sane.
    assert!(
        (0.0..=1.0).contains(&features.confidence),
        "confidence {} must be a 0.0..=1.0 aggregate",
        features.confidence
    );
}

#[test]
fn derive_produces_a_low_ratio_for_a_narrow_shoulder_wide_hip_frame() {
    // Invert the golden geometry: narrow shoulders (span 0.14) over wide hips
    // (span 0.40) — a non-tapered, hip-dominant frame must read LOW.
    let pose = pose_with(&[
        (Landmark::LeftShoulder, kp(0.43, 0.22)),
        (Landmark::RightShoulder, kp(0.57, 0.22)),
        (Landmark::LeftHip, kp(0.30, 0.55)),
        (Landmark::RightHip, kp(0.70, 0.55)),
    ]);
    let features =
        derive_frame_features(&pose).expect("a confident (if untapered) pose must derive");

    assert!(
        (1.0..=2.5).contains(&features.shoulder_to_waist),
        "even an untapered ratio must be clamped into the matchable band, got {}",
        features.shoulder_to_waist
    );
    assert!(
        features.shoulder_to_waist < 1.75,
        "a narrow-shoulder/wide-hip frame must yield a low ratio, got {}",
        features.shoulder_to_waist
    );
}

#[test]
fn derive_orders_taper_monotonically() {
    // The whole point of the ratio is that a wider taper ⇒ a higher number. A
    // strongly-tapered frame must out-rank a mildly-tapered one, whatever the
    // exact normalization.
    let strong = derive_frame_features(&valid_pose())
        .unwrap()
        .shoulder_to_waist;

    // Mild taper: shoulders span 0.24, hips span 0.14 — tapered, but less so.
    let mild = derive_frame_features(&pose_with(&[
        (Landmark::LeftShoulder, kp(0.38, 0.22)),
        (Landmark::RightShoulder, kp(0.62, 0.22)),
    ]))
    .unwrap()
    .shoulder_to_waist;

    assert!(
        strong > mild,
        "a stronger taper ({strong}) must yield a higher ratio than a milder one ({mild})"
    );
}

// ===========================================================================
// SAC1 / AC1: the banded categorical fields — present when their keypoints clear
// the floor, `None` when they don't (absent, never fabricated). The build /
// somatotype / structure_tags a 2-D photo cannot determine are simply not on
// `FrameFeatures` (SPEC-0013 §2.2 honesty constraint) — there is nothing to
// fabricate and nothing to assert-absent at the type level beyond their omission.
// ===========================================================================

#[test]
fn derive_populates_banded_fields_when_keypoints_are_confident() {
    let features = derive_frame_features(&valid_pose())
        .expect("a fully-confident pose must derive banded fields");

    // With confident shoulders + a torso normalizer, clavicle_width is derivable.
    let clavicle: Option<WidthBand> = features.clavicle_width;
    assert!(
        clavicle.is_some(),
        "a confident wide-frame pose must yield a clavicle_width band"
    );
    // The wide golden frame should band as Wide (the widest shoulder span).
    assert_eq!(
        clavicle,
        Some(WidthBand::Wide),
        "the wide golden frame's clavicle_width should band Wide"
    );

    // With confident limb + torso points, limb_length is derivable.
    let limb: Option<LengthBand> = features.limb_length;
    assert!(
        limb.is_some(),
        "a confident pose with limbs must yield a limb_length band"
    );
}

#[test]
fn derive_leaves_clavicle_band_absent_when_shoulders_are_low_confidence() {
    // Dim BOTH shoulders below the floor but keep them geometrically placed so a
    // naive derivation that ignores confidence would still produce a band. The
    // ratio still needs the shoulders, so the whole derivation must fail OR the
    // band must be absent — we assert the band is honestly absent by deriving
    // from a pose whose shoulders are present for the *ratio* but the *width
    // normalizer keypoints* (the torso/height reference) are dim. To keep the
    // shoulders confident for the ratio yet drop the band, dim the ears (the
    // head-width normalizer the clavicle band reads against).
    let pose = pose_with(&[
        (Landmark::LeftEar, dim(0.44, 0.07)),
        (Landmark::RightEar, dim(0.56, 0.07)),
        (Landmark::Nose, dim(0.50, 0.08)),
        (Landmark::LeftEye, dim(0.47, 0.06)),
        (Landmark::RightEye, dim(0.53, 0.06)),
    ]);
    let features = derive_frame_features(&pose)
        .expect("confident shoulders+hips still yield the numeric ratio");

    assert!(
        features.clavicle_width.is_none(),
        "with no confident head/torso normalizer, clavicle_width must be None (absent, not fabricated)"
    );
    // The numeric ratio survives — shoulders and hips are still confident.
    assert!((1.0..=2.5).contains(&features.shoulder_to_waist));
}

#[test]
fn derive_leaves_limb_band_absent_when_limbs_are_low_confidence() {
    // Dim every elbow/wrist/knee/ankle: the limb-length band has no confident
    // segment to measure, so it must be absent — but the shoulder/hip ratio still
    // derives.
    let pose = pose_with(&[
        (Landmark::LeftElbow, dim(0.26, 0.40)),
        (Landmark::RightElbow, dim(0.74, 0.40)),
        (Landmark::LeftWrist, dim(0.24, 0.55)),
        (Landmark::RightWrist, dim(0.76, 0.55)),
        (Landmark::LeftKnee, dim(0.42, 0.75)),
        (Landmark::RightKnee, dim(0.58, 0.75)),
        (Landmark::LeftAnkle, dim(0.41, 0.95)),
        (Landmark::RightAnkle, dim(0.59, 0.95)),
    ]);
    let features = derive_frame_features(&pose)
        .expect("confident shoulders+hips still yield the numeric ratio");

    assert!(
        features.limb_length.is_none(),
        "with no confident limb segments, limb_length must be None (absent, not fabricated)"
    );
    assert!((1.0..=2.5).contains(&features.shoulder_to_waist));
}

// ===========================================================================
// SAC1 / AC1: degenerate geometry → a typed FrameError, never a fabricated
// match. (a) too few confident keypoints for the ratio; (b) a zero/near-zero
// hip span (the division guard).
// ===========================================================================

#[test]
fn derive_rejects_too_few_confident_keypoints() {
    // Dim BOTH shoulders and BOTH hips: the load-bearing ratio points are below
    // the floor, so the numeric ratio cannot be formed — a typed error, not a
    // guessed number.
    let pose = pose_with(&[
        (Landmark::LeftShoulder, dim(0.30, 0.22)),
        (Landmark::RightShoulder, dim(0.70, 0.22)),
        (Landmark::LeftHip, dim(0.43, 0.55)),
        (Landmark::RightHip, dim(0.57, 0.55)),
    ]);
    let err = derive_frame_features(&pose)
        .expect_err("a pose with no confident shoulders/hips must be a FrameError");
    // A typed, meaningful error (not stringly-typed) — its Display is non-empty.
    assert!(
        !err.to_string().is_empty(),
        "FrameError must carry a meaningful message: {err:?}"
    );
}

#[test]
fn derive_rejects_a_degenerate_zero_hip_span() {
    // Collapse both hips onto the SAME x — the hip span (the waist proxy and the
    // ratio's denominator) is zero. The division guard must reject this rather
    // than divide by zero / emit an infinite ratio.
    let pose = pose_with(&[
        (Landmark::LeftHip, kp(0.50, 0.55)),
        (Landmark::RightHip, kp(0.50, 0.55)),
    ]);
    let err = derive_frame_features(&pose)
        .expect_err("a zero hip span (degenerate denominator) must be a FrameError");
    assert!(
        !err.to_string().is_empty(),
        "FrameError must carry a meaningful message: {err:?}"
    );
}

#[test]
fn derive_error_is_a_typed_value() {
    // Pin that FrameError is a real, matchable typed error (the no-stringly-typed
    // rule, CLAUDE.md §6) — usable in a `match`, not just a string.
    let pose = pose_with(&[
        (Landmark::LeftHip, kp(0.50, 0.55)),
        (Landmark::RightHip, kp(0.50, 0.55)),
    ]);
    let result: Result<FrameFeatures, FrameError> = derive_frame_features(&pose);
    assert!(
        result.is_err(),
        "a degenerate frame must yield Err(FrameError), got {result:?}"
    );
}
