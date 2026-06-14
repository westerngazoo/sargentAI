//! R-0013 **real-ONNX** integration test (AC4 / SAC4) — the one path that runs
//! the actual bundled MoveNet model on a committed fixture image, end-to-end:
//! bytes → keypoints → `FrameFeatures` → ranking (SPEC-0013 §2.6/§2.7).
//!
//! Authored by the qa agent during R-0013 step 3 (test planning). This is the
//! deliberate exception to the "fake estimator everywhere" rule: AC4 demands at
//! least one test exercise the genuine model, and SPEC-0013 §2.7 requires it to
//! assert a **plausible `shoulder_to_waist` range**, not merely status `200` — so
//! a silently-wrong preprocessing (the NHWC/int32/letterbox gotcha, SPEC-0013
//! §2.6) that yields a distorted-but-non-erroring pose is caught.
//!
//! Step 5 delivered the artifacts this test needs — the bundled
//! `backend/crates/api/models/movenet-lightning.onnx` (embedded via
//! `include_bytes!`), the `ort`/`image` dependencies, the `OnnxPoseEstimator`,
//! and the committed `fixtures/physique-front.jpg` — so the test now runs in CI
//! (the qa step-3 author left it `#[ignore]` until then; that attribute is gone).
//! SPEC-0013 §2.6: the default `download-binaries` path works on `ubuntu-latest`
//! with no system setup.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::doc_markdown)]

use fitai_api::pose::{OnnxPoseEstimator, PoseEstimator};
use fitai_core::pose::derive_frame_features;
use fitai_core::ImageContentType;

/// A committed public-domain JPEG of a single, clearly-posed standing figure
/// (front angle) — see `fixtures/SOURCES.md`.
const FIXTURE_JPEG: &[u8] = include_bytes!("fixtures/physique-front.jpg");

/// AC4: the REAL model, end-to-end on the fixture, asserting a plausible
/// `shoulder_to_waist` — not just that inference returned.
#[tokio::test]
async fn real_onnx_estimator_derives_a_plausible_ratio_from_a_fixture() {
    // The real estimator loads the bundled model once (Arc<Session>); no DB, no
    // router — this is the inference + derivation seam only.
    let estimator = OnnxPoseEstimator::load().expect("the bundled MoveNet model must load");

    let keypoints = estimator
        .estimate(FIXTURE_JPEG, ImageContentType::ImageJpeg)
        .await
        .expect("the real model must extract a pose from a clearly-posed figure");

    let features = derive_frame_features(&keypoints)
        .expect("the fixture's real keypoints must derive frame features");

    // The load-bearing AC4 assertion: a PLAUSIBLE ratio, not just non-erroring.
    // A real human physique's shoulder-to-hip span ratio sits within the
    // library's matchable envelope; a preprocessing bug (NCHW vs NHWC, float vs
    // int32, a missing letterbox) distorts the pose and pushes the ratio out of
    // this band even though inference "succeeds".
    assert!(
        (1.0..=2.5).contains(&features.shoulder_to_waist),
        "the real model's derived shoulder_to_waist {} must be a plausible human ratio in [1.0, 2.5] \
         — an out-of-band value signals a silently-wrong preprocessing (NHWC/int32/letterbox)",
        features.shoulder_to_waist
    );
    assert!(
        features.confidence > 0.0,
        "a real detection must carry a positive aggregate confidence"
    );
}
