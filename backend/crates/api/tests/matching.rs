//! R-0013 photo→archetype match integration suite — `POST /photo-sessions/:id/match`
//! driven by the deterministic **fake** pose estimator (SPEC-0013 §2.5/§2.7).
//!
//! Authored by the qa agent during R-0013 step 3 (test planning), BEFORE the
//! `api::pose` seam, the `api::matching` endpoint, the `AppState.pose` field, the
//! `ApiError::Unprocessable` variant, and `db::match_candidates_for_session`
//! exist. Pre-implementation red state = this crate fails to COMPILE (the
//! `fitai_api::pose` module, the `/photo-sessions/:id/match` route, and the
//! `build_app_with_pose` harness helper are all absent). Implementation step 5
//! makes it green.
//!
//! The estimator is injected so the endpoint suite is deterministic and never
//! loads the ONNX model (AC3/AC8): each test builds the app with a
//! `FakePoseEstimator` configured to return either a hand-authored
//! `PoseKeypoints` or a `PoseError`. The session + photo are created through the
//! REAL R-0006 photo endpoints (multipart upload), so the match path exercises
//! the genuine ownership + storage seam, with only inference faked.
//!
//! Every test is `#[sqlx::test(migrations = "../../migrations")]` — sqlx
//! provisions a fresh per-test database (the R-0002..R-0006 harness); the
//! `register_and_token` + `create_session` + `upload` helpers seed the auth and
//! photo substrate. No matching state is persisted (R-0013 is read-only).
//!
//! SAC → AC → test traceability (full table in the qa sign-off report):
//! - SAC5 → AC5: a ranked `200` whose body is `{ "matches": [...] }`, each match
//!   the R-0012 `ArchetypeResponse` shape PLUS `distance` + `score`, nearest
//!   first; `internal_name`/`sources`/the famous labels never cross the wire.
//! - SAC6 → AC6: a session with no photos → `422 no_usable_photo`; an estimator
//!   that detects no person → `422 no_person_detected`; a foreign session → `404`
//!   (cross-user 404-not-403); a missing session → `404`; no token → `401`.
//! - SAC7 → AC7: the match is scoped to the token `sub` (the foreign-404 case);
//!   bytes are read through the seam and never returned (only derived features).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Test doc comments quote JSON keys, slugs, and reason tokens as prose.
#![allow(clippy::doc_markdown)]

mod common;

use axum::http::StatusCode;
use common::{body_json, build_app_with_pose, create_session, png_upload, register_and_token};
use fitai_api::pose::{FakePoseEstimator, PoseError};
use fitai_core::pose::{Keypoint, Landmark, PoseKeypoints};
use serde_json::json;
use sqlx::PgPool;

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

// ===========================================================================
// A hand-authored COCO-17 skeleton (a clear wide-shoulder / narrow-hip "V")
// the fake estimator returns, so the derived `shoulder_to_waist` is a sane,
// matchable ratio. Mirrors `core/tests/pose.rs::valid_pose`.
// ===========================================================================

fn kp(x: f32, y: f32) -> Keypoint {
    Keypoint { x, y, score: 0.9 }
}

/// COCO-17 order; see `core/tests/pose.rs` for the index legend.
fn wide_frame_pose() -> PoseKeypoints {
    PoseKeypoints::new([
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
    ])
}

#[allow(unused)]
fn touch_landmark_enum() {
    // Keep the `Landmark` import meaningful for readers wiring new fixtures.
    let _ = Landmark::LeftShoulder;
}

// ===========================================================================
// SAC5 / AC5: a ranked 200 — the wire shape + the privacy contract.
// ===========================================================================

/// AC5: POST .../match with a usable injected pose → 200 + a `matches` array,
/// nearest first, each carrying the user-facing archetype shape PLUS a numeric
/// `distance` and `score`.
#[sqlx::test(migrations = "../../migrations")]
async fn match_returns_200_ranked_matches_with_distance_and_score(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (_id, token) = register_and_token(&app, "match-ok@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;
    png_upload(&app, &token, &session_id, Some("front")).await;

    let resp = common::post_json_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/match"),
        Some(&bearer(&token)),
        json!({}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    let matches = body["matches"]
        .as_array()
        .expect("the response must carry a `matches` array");
    assert_eq!(
        matches.len(),
        6,
        "the ranking must cover all six library archetypes"
    );

    // Each match carries the user-facing archetype fields + distance + score.
    for m in matches {
        assert!(
            m.get("id").and_then(|v| v.as_str()).is_some(),
            "each match must carry the archetype id"
        );
        assert!(
            !m["display_name"].as_str().unwrap().is_empty(),
            "each match must carry a non-empty display_name"
        );
        let distance = m["distance"]
            .as_f64()
            .expect("each match must carry a numeric distance");
        let score = m["score"]
            .as_f64()
            .expect("each match must carry a numeric score");
        assert!(
            (0.0..=1.0).contains(&distance),
            "distance {distance} must be in [0.0, 1.0]"
        );
        assert!(
            (0.0..=1.0).contains(&score),
            "score {score} must be in [0.0, 1.0]"
        );
        assert!(
            (score - (1.0 - distance)).abs() < 1e-9,
            "score must equal 1 - distance (score={score}, distance={distance})"
        );
    }

    // Nearest first: distances ascend.
    let distances: Vec<f64> = matches
        .iter()
        .map(|m| m["distance"].as_f64().unwrap())
        .collect();
    for pair in distances.windows(2) {
        assert!(
            pair[0] <= pair[1],
            "matches must be sorted nearest-first: {} before {}",
            pair[0],
            pair[1]
        );
    }
}

/// AC5 (privacy, reused from R-0012 AC4): the match wire NEVER carries
/// `internal_name`, `sources`, or any famous research label.
#[sqlx::test(migrations = "../../migrations")]
async fn match_response_never_leaks_internal_name_or_sources(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (_id, token) = register_and_token(&app, "match-priv@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;
    png_upload(&app, &token, &session_id, Some("front")).await;

    let resp = common::post_json_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/match"),
        Some(&bearer(&token)),
        json!({}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    for m in body["matches"].as_array().unwrap() {
        assert!(
            m.get("internal_name").is_none(),
            "no match may carry internal_name"
        );
        assert!(m.get("sources").is_none(), "no match may carry sources");
    }
    let serialized = body.to_string();
    assert!(
        !serialized.contains("internal_name") && !serialized.contains("sources"),
        "the match JSON must not contain internal_name or sources"
    );
    for label in ["Yates", "Mentzer", "Arnold", "Columbu", "Cutler", "Heath"] {
        assert!(
            !serialized.contains(label),
            "the wire must never expose the internal research label {label:?}"
        );
    }
}

// ===========================================================================
// SAC6 / AC6: honest failure modes — both 422 triggers, 404, 401.
// ===========================================================================

/// AC6: a session with NO photos → 422 `{"error":"unprocessable","reason":"no_usable_photo"}`.
#[sqlx::test(migrations = "../../migrations")]
async fn match_on_a_session_with_no_photos_is_unprocessable(pool: PgPool) {
    // The pose would succeed if reached — but there is nothing to estimate, so
    // the handler must short-circuit BEFORE inference with no_usable_photo.
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (_id, token) = register_and_token(&app, "match-empty@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await; // no upload

    let resp = common::post_json_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/match"),
        Some(&bearer(&token)),
        json!({}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "unprocessable", "reason": "no_usable_photo" }),
        "an empty session must be 422 no_usable_photo"
    );
}

/// AC6: the estimator detects no person on every photo → 422
/// `{"error":"unprocessable","reason":"no_person_detected"}`.
#[sqlx::test(migrations = "../../migrations")]
async fn match_when_no_person_detected_is_unprocessable(pool: PgPool) {
    let fake = FakePoseEstimator::failing(PoseError::NoPersonDetected);
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (_id, token) = register_and_token(&app, "match-noperson@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;
    png_upload(&app, &token, &session_id, Some("front")).await;

    let resp = common::post_json_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/match"),
        Some(&bearer(&token)),
        json!({}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "unprocessable", "reason": "no_person_detected" }),
        "a photo with no detectable pose must be 422 no_person_detected"
    );
}

/// AC6: a missing session → 404 with the uniform body.
#[sqlx::test(migrations = "../../migrations")]
async fn match_on_a_missing_session_is_not_found(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (_id, token) = register_and_token(&app, "match-missing@b.com", "8charsmin").await;

    let unknown = uuid::Uuid::new_v4();
    let resp = common::post_json_with_auth(
        &app,
        &format!("/photo-sessions/{unknown}/match"),
        Some(&bearer(&token)),
        json!({}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await, json!({ "error": "not_found" }));
}

/// AC6/AC7: another user's session → 404 (cross-user is 404, never 403); no
/// foreign bytes are read or matched.
#[sqlx::test(migrations = "../../migrations")]
async fn match_on_a_foreign_session_is_not_found(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (_id_a, token_a) = register_and_token(&app, "match-ownerA@b.com", "8charsmin").await;
    let (_id_b, token_b) = register_and_token(&app, "match-intruderB@b.com", "8charsmin").await;

    let session_a = create_session(&app, &token_a).await;
    png_upload(&app, &token_a, &session_a, Some("front")).await;

    // B attempts to match A's session.
    let resp = common::post_json_with_auth(
        &app,
        &format!("/photo-sessions/{session_a}/match"),
        Some(&bearer(&token_b)),
        json!({}),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "a foreign session must be 404 (cross-user 404-not-403)"
    );
}

/// AC6: no token → 401.
#[sqlx::test(migrations = "../../migrations")]
async fn match_without_token_is_unauthorized(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (_id, token) = register_and_token(&app, "match-unauth@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;
    png_upload(&app, &token, &session_id, Some("front")).await;

    let resp = common::post_json_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/match"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(resp).await, json!({ "error": "unauthorized" }));
}

// ===========================================================================
// SAC6 / AC6: degenerate-frame branch — the estimator returns keypoints, but the
// geometry is degenerate (collapsed hips), so `derive_frame_features` rejects it
// → 422 degenerate_frame. This exercises the FrameError → Unprocessable mapping
// distinct from the NoPersonDetected path.
// ===========================================================================

/// AC6: a returned-but-degenerate pose (zero hip span) → 422 `degenerate_frame`.
#[sqlx::test(migrations = "../../migrations")]
async fn match_on_a_degenerate_frame_is_unprocessable(pool: PgPool) {
    // Both hips collapsed onto the same x ⇒ zero hip span ⇒ FrameError ⇒ 422.
    let degenerate = PoseKeypoints::new([
        kp(0.50, 0.08),
        kp(0.47, 0.06),
        kp(0.53, 0.06),
        kp(0.44, 0.07),
        kp(0.56, 0.07),
        kp(0.30, 0.22),
        kp(0.70, 0.22),
        kp(0.26, 0.40),
        kp(0.74, 0.40),
        kp(0.24, 0.55),
        kp(0.76, 0.55),
        kp(0.50, 0.55), // left_hip  — collapsed
        kp(0.50, 0.55), // right_hip — collapsed (zero hip span)
        kp(0.42, 0.75),
        kp(0.58, 0.75),
        kp(0.41, 0.95),
        kp(0.59, 0.95),
    ]);
    let fake = FakePoseEstimator::returning(degenerate);
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (_id, token) = register_and_token(&app, "match-degenerate@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;
    png_upload(&app, &token, &session_id, Some("front")).await;

    let resp = common::post_json_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/match"),
        Some(&bearer(&token)),
        json!({}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "unprocessable", "reason": "degenerate_frame" }),
        "a degenerate returned pose must be 422 degenerate_frame"
    );
}
