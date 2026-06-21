//! R-0014 program + diet integration suite.
//!
//! Tests `GET /photo-sessions/:id/program-proposals`, `POST /programs`,
//! `GET /programs/me/current`, and `GET /programs/me` driven by the
//! deterministic [`FakePoseEstimator`] (SPEC-0014 §3.2).
//!
//! Authored by the qa agent during R-0014 step 3 (TDD red), BEFORE the
//! `api::program` module, the `user_programs` migration, the `ApiError::Conflict`
//! variant, and the program routes exist. Pre-implementation red state = this
//! crate fails to COMPILE (the route handlers and the `program` module import
//! path do not exist). Implementation step 5 makes it green.
//!
//! Every test is `#[sqlx::test(migrations = "../../migrations")]` — sqlx
//! provisions a fresh per-test database and applies all migrations (including
//! the new 00006_user_programs.sql once it exists). The app is built with
//! `build_app_with_pose` so ONNX is never loaded; the pose injection makes the
//! frame derivation deterministic.
//!
//! AC-coverage:
//! - AC1  (proposals endpoint 200, top-3, no `internal_name`)
//! - AC2  (derived program + diet in each proposal)
//! - AC3  (choose endpoint 201, deactivates previous, 409 wrong archetype)
//! - AC4  (GET /programs/me/current — 200 or 404)
//! - AC5  (GET /programs/me — history, newest-first, pagination)
//! - AC6  (ownership isolation — cross-user 404, 401 unauthenticated)
//! - AC10 (all integration test cases)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Test doc comments quote JSON keys and route paths as prose.
#![allow(clippy::doc_markdown)]

mod common;

use axum::http::StatusCode;
use common::{
    body_json, build_app_with_pose, create_session, png_upload, post_json_with_auth,
    register_and_token,
};
use fitai_api::pose::FakePoseEstimator;
use fitai_core::pose::{Keypoint, PoseKeypoints};
use serde_json::json;
use sqlx::PgPool;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

fn kp(x: f32, y: f32) -> Keypoint {
    Keypoint { x, y, score: 0.9 }
}

/// The same wide-frame pose used in R-0013 matching tests — gives a valid,
/// non-degenerate `shoulder_to_waist` ratio that ranks all six archetypes.
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

/// Create a profile for `token`'s user — required before proposals/choose work.
///
/// Uses a fixed 30-year-old male profile with the BuildMuscle goal, so the
/// template instantiation has concrete inputs to work with.
async fn create_profile(app: &axum::Router, token: &str) {
    let resp = common::put_json_with_auth(
        app,
        "/profile/me",
        Some(&bearer(token)),
        json!({
            "date_of_birth": "1996-06-20",
            "height_cm": 180,
            "weight_kg": 80.0,
            "sex": "male",
            "goals": ["build_muscle"]
        }),
    )
    .await;
    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::CREATED,
        "seed profile PUT expected 200/201, got {}",
        resp.status()
    );
}

/// Seed: register + login + create profile + create photo session + upload a photo.
/// Returns `(token, session_id)`.
async fn seed_with_session_and_photo(app: &axum::Router, email: &str) -> (String, String) {
    let (_, token) = register_and_token(app, email, "8charsmin").await;
    create_profile(app, &token).await;
    let session_id = create_session(app, &token).await;
    png_upload(app, &token, &session_id, Some("front")).await;
    (token, session_id)
}

/// POST /programs with the first archetype id from the proposals list.
/// Returns the response.
async fn choose_first_proposal(
    app: &axum::Router,
    token: &str,
    session_id: &str,
) -> axum::response::Response<axum::body::Body> {
    // First fetch proposals to get a valid archetype_id.
    let proposals_resp = common::get_with_auth(
        app,
        &format!("/photo-sessions/{session_id}/program-proposals"),
        Some(&bearer(token)),
    )
    .await;
    assert_eq!(
        proposals_resp.status(),
        StatusCode::OK,
        "proposals must succeed before choose"
    );
    let body = body_json(proposals_resp).await;
    let archetype_id = body["proposals"][0]["archetype_id"]
        .as_str()
        .expect("first proposal must have an archetype_id")
        .to_string();

    post_json_with_auth(
        app,
        "/programs",
        Some(&bearer(token)),
        json!({
            "photo_session_id": session_id,
            "archetype_id": archetype_id
        }),
    )
    .await
}

// ---------------------------------------------------------------------------
// AC1 — proposals endpoint 200, top-3, idempotent, no internal_name
// ---------------------------------------------------------------------------

/// AC1: GET /photo-sessions/:id/program-proposals → 200 + exactly 3 proposals,
/// scores descending, no `internal_name` or `sources` in the body.
#[sqlx::test(migrations = "../../migrations")]
async fn proposals_returns_top3_for_own_session(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (token, session_id) = seed_with_session_and_photo(&app, "prop-ok@b.com").await;

    let resp = common::get_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/program-proposals"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    let proposals = body["proposals"]
        .as_array()
        .expect("response must have a `proposals` array");
    assert_eq!(proposals.len(), 3, "proposals must return exactly 3 items");

    // Scores descend (nearest first).
    let scores: Vec<f64> = proposals
        .iter()
        .map(|p| {
            p["score"]
                .as_f64()
                .expect("each proposal must have a numeric score")
        })
        .collect();
    for pair in scores.windows(2) {
        assert!(
            pair[0] >= pair[1],
            "proposals must be ordered score descending: {} before {}",
            pair[0],
            pair[1]
        );
    }

    // Each proposal has the required fields.
    for p in proposals {
        assert!(p.get("archetype_id").and_then(|v| v.as_str()).is_some());
        assert!(p.get("display_name").and_then(|v| v.as_str()).is_some());
        assert!(p.get("score").and_then(serde_json::Value::as_f64).is_some());
        assert!(p
            .get("distance")
            .and_then(serde_json::Value::as_f64)
            .is_some());
        assert!(p.get("program").is_some(), "proposal must include program");
        assert!(p.get("diet").is_some(), "proposal must include diet");
    }

    // Privacy: no internal labels on the wire.
    let serialized = body.to_string();
    assert!(
        !serialized.contains("internal_name"),
        "proposals must not expose internal_name"
    );
    assert!(
        !serialized.contains("sources"),
        "proposals must not expose sources"
    );
    for label in ["Yates", "Mentzer", "Arnold", "Columbu", "Cutler", "Heath"] {
        assert!(
            !serialized.contains(label),
            "proposals must not expose internal research label {label:?}"
        );
    }
}

/// AC1: re-calling the proposals endpoint produces the same top-3 (idempotent).
#[sqlx::test(migrations = "../../migrations")]
async fn proposals_endpoint_is_idempotent(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (token, session_id) = seed_with_session_and_photo(&app, "prop-idem@b.com").await;

    let url = format!("/photo-sessions/{session_id}/program-proposals");
    let resp1 = common::get_with_auth(&app, &url, Some(&bearer(&token))).await;
    let resp2 = common::get_with_auth(&app, &url, Some(&bearer(&token))).await;
    assert_eq!(resp1.status(), StatusCode::OK);
    assert_eq!(resp2.status(), StatusCode::OK);
    let body1 = body_json(resp1).await;
    let body2 = body_json(resp2).await;
    assert_eq!(
        body1["proposals"], body2["proposals"],
        "proposals endpoint must be idempotent"
    );
}

// ---------------------------------------------------------------------------
// AC6 — ownership isolation on proposals endpoint
// ---------------------------------------------------------------------------

/// AC6: another user's session → 404 (cross-user is 404, never 403).
#[sqlx::test(migrations = "../../migrations")]
async fn proposals_cross_user_session_is_404(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);

    let (token_a, session_a) = seed_with_session_and_photo(&app, "prop-ownerA@b.com").await;
    let (_, token_b) = register_and_token(&app, "prop-intruderB@b.com", "8charsmin").await;
    create_profile(&app, &token_b).await;

    let resp = common::get_with_auth(
        &app,
        &format!("/photo-sessions/{session_a}/program-proposals"),
        Some(&bearer(&token_b)),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "cross-user proposals access must be 404, not 403"
    );
    // Silence unused variable warning for token_a — it seeds the session.
    let _ = token_a;
}

/// AC6: no token → 401.
#[sqlx::test(migrations = "../../migrations")]
async fn proposals_unauthenticated_is_401(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (token, session_id) = seed_with_session_and_photo(&app, "prop-unauth@b.com").await;

    let resp = common::get_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/program-proposals"),
        None, // no auth header
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(resp).await, json!({ "error": "unauthorized" }));
    let _ = token;
}

/// AC6: user with no profile → 404 (no profile to parameterise the diet).
#[sqlx::test(migrations = "../../migrations")]
async fn proposals_no_profile_is_404(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    // Register but do NOT create a profile.
    let (_, token) = register_and_token(&app, "prop-noprof@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;
    png_upload(&app, &token, &session_id, Some("front")).await;

    let resp = common::get_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/program-proposals"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "missing profile must yield 404 on proposals"
    );
}

/// AC1 / AC6: empty session (no photos) → 422 `no_usable_photo`.
#[sqlx::test(migrations = "../../migrations")]
async fn proposals_no_photo_is_422(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (_, token) = register_and_token(&app, "prop-nophoto@b.com", "8charsmin").await;
    create_profile(&app, &token).await;
    let session_id = create_session(&app, &token).await; // no upload

    let resp = common::get_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/program-proposals"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "unprocessable", "reason": "no_usable_photo" })
    );
}

// ---------------------------------------------------------------------------
// AC3 — choose endpoint: 201, deactivates previous, 409
// ---------------------------------------------------------------------------

/// AC3: POST /programs → 201, response body has `active: true` and expected
/// fields.
#[sqlx::test(migrations = "../../migrations")]
async fn choose_creates_active_user_program(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (token, session_id) = seed_with_session_and_photo(&app, "choose-ok@b.com").await;

    let resp = choose_first_proposal(&app, &token, &session_id).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "choose must return 201");
    let body = body_json(resp).await;
    assert_eq!(
        body["active"].as_bool(),
        Some(true),
        "new program must be active"
    );
    assert!(body.get("id").is_some(), "response must include id");
    assert!(body.get("archetype_id").is_some());
    assert!(body.get("program").is_some());
    assert!(body.get("diet").is_some());
    assert!(body.get("chosen_at").is_some());
}

/// AC3: choosing a second program deactivates the first — only one active at a
/// time.
#[sqlx::test(migrations = "../../migrations")]
async fn choose_deactivates_previous_program(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (token, session_id) = seed_with_session_and_photo(&app, "choose-deact@b.com").await;

    // First choose.
    let first_resp = choose_first_proposal(&app, &token, &session_id).await;
    assert_eq!(first_resp.status(), StatusCode::CREATED);
    let first_id = body_json(first_resp).await["id"]
        .as_str()
        .expect("first program must have an id")
        .to_string();

    // Second choose — a new session + photo for variety (any archetype from its
    // proposals will do), or re-use the same session with a different archetype.
    // Re-derive proposals from same session and pick the SECOND archetype.
    let proposals_resp = common::get_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/program-proposals"),
        Some(&bearer(&token)),
    )
    .await;
    let proposals_body = body_json(proposals_resp).await;
    let second_archetype_id = proposals_body["proposals"][1]["archetype_id"]
        .as_str()
        .expect("second proposal must have an archetype_id")
        .to_string();

    let second_resp = post_json_with_auth(
        &app,
        "/programs",
        Some(&bearer(&token)),
        json!({
            "photo_session_id": session_id,
            "archetype_id": second_archetype_id
        }),
    )
    .await;
    assert_eq!(second_resp.status(), StatusCode::CREATED);
    let second_id = body_json(second_resp).await["id"]
        .as_str()
        .expect("second program must have an id")
        .to_string();
    assert_ne!(
        first_id, second_id,
        "two choose calls must produce two rows"
    );

    // GET /programs/me/current must now return the second program.
    let current_resp =
        common::get_with_auth(&app, "/programs/me/current", Some(&bearer(&token))).await;
    assert_eq!(current_resp.status(), StatusCode::OK);
    let current = body_json(current_resp).await;
    assert_eq!(
        current["id"].as_str(),
        Some(second_id.as_str()),
        "current program must be the most recently chosen one"
    );
    assert_eq!(
        current["active"].as_bool(),
        Some(true),
        "current program must be active"
    );
}

/// AC3: choosing an archetype that is NOT in the session's top-3 proposals →
/// 409 with `{"error": "conflict", "reason": "archetype_not_in_proposals"}`.
#[sqlx::test(migrations = "../../migrations")]
async fn choose_archetype_not_in_top3_is_409(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (token, session_id) = seed_with_session_and_photo(&app, "choose-bad@b.com").await;

    // Use a slug that exists in the library but is not rank 1/2/3 for this
    // particular frame pose (the wide frame will rank some archetypes last).
    // We send all six slugs and assert exactly the non-top-3 ones 409.
    // For simplicity, we use a deliberately bogus slug that cannot be in any
    // top-3 because it is not in the library at all.
    let resp = post_json_with_auth(
        &app,
        "/programs",
        Some(&bearer(&token)),
        json!({
            "photo_session_id": session_id,
            "archetype_id": "does-not-exist-slug"
        }),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "unknown archetype must yield 409"
    );
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "conflict", "reason": "archetype_not_in_proposals" })
    );
}

/// AC6: choosing from another user's session → 404.
#[sqlx::test(migrations = "../../migrations")]
async fn choose_cross_user_session_is_404(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);

    let (token_a, session_a) = seed_with_session_and_photo(&app, "choose-ownerA@b.com").await;
    let (_, token_b) = register_and_token(&app, "choose-intruderB@b.com", "8charsmin").await;
    create_profile(&app, &token_b).await;

    let resp = post_json_with_auth(
        &app,
        "/programs",
        Some(&bearer(&token_b)),
        json!({
            "photo_session_id": session_a,
            "archetype_id": "heavy-duty-mass"
        }),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "cross-user choose must be 404"
    );
    let _ = token_a;
}

/// AC6: no profile for choosing user → 404.
#[sqlx::test(migrations = "../../migrations")]
async fn choose_no_profile_is_404(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    // Register but do NOT create a profile.
    let (_, token) = register_and_token(&app, "choose-noprof@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;
    png_upload(&app, &token, &session_id, Some("front")).await;

    let resp = post_json_with_auth(
        &app,
        "/programs",
        Some(&bearer(&token)),
        json!({
            "photo_session_id": session_id,
            "archetype_id": "heavy-duty-mass"
        }),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "missing profile must yield 404 on choose"
    );
}

/// AC6: no token → 401.
#[sqlx::test(migrations = "../../migrations")]
async fn choose_unauthenticated_is_401(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);

    let resp = post_json_with_auth(
        &app,
        "/programs",
        None, // no auth
        json!({
            "photo_session_id": uuid::Uuid::new_v4().to_string(),
            "archetype_id": "heavy-duty-mass"
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(resp).await, json!({ "error": "unauthorized" }));
}

// ---------------------------------------------------------------------------
// AC4 — GET /programs/me/current
// ---------------------------------------------------------------------------

/// AC4: after a successful choose → 200 with the active program.
#[sqlx::test(migrations = "../../migrations")]
async fn current_returns_active_program(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (token, session_id) = seed_with_session_and_photo(&app, "current-ok@b.com").await;

    choose_first_proposal(&app, &token, &session_id).await;

    let resp = common::get_with_auth(&app, "/programs/me/current", Some(&bearer(&token))).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["active"].as_bool(), Some(true));
    assert!(body.get("id").is_some());
    assert!(body.get("archetype_id").is_some());
    assert!(body.get("program").is_some());
    assert!(body.get("diet").is_some());
}

/// AC4: fresh user (no choose yet) → 404.
#[sqlx::test(migrations = "../../migrations")]
async fn current_no_program_is_404(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (_, token) = register_and_token(&app, "current-empty@b.com", "8charsmin").await;
    create_profile(&app, &token).await;

    let resp = common::get_with_auth(&app, "/programs/me/current", Some(&bearer(&token))).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "no chosen program must yield 404"
    );
}

/// AC6: no token → 401 on current.
#[sqlx::test(migrations = "../../migrations")]
async fn current_unauthenticated_is_401(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);

    let resp = common::get_with_auth(&app, "/programs/me/current", None).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// AC5 — GET /programs/me (history)
// ---------------------------------------------------------------------------

/// AC5: after two chooses → history returns both, newest first.
#[sqlx::test(migrations = "../../migrations")]
async fn history_returns_programs_newest_first(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (token, session_id) = seed_with_session_and_photo(&app, "hist-order@b.com").await;

    // First choose.
    let r1 = choose_first_proposal(&app, &token, &session_id).await;
    assert_eq!(r1.status(), StatusCode::CREATED);
    let first_id = body_json(r1).await["id"]
        .as_str()
        .expect("first id")
        .to_string();

    // Second choose — different archetype from same session.
    let proposals_resp = common::get_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/program-proposals"),
        Some(&bearer(&token)),
    )
    .await;
    let proposals = body_json(proposals_resp).await;
    let second_arch = proposals["proposals"][1]["archetype_id"]
        .as_str()
        .unwrap()
        .to_string();
    let r2 = post_json_with_auth(
        &app,
        "/programs",
        Some(&bearer(&token)),
        json!({ "photo_session_id": session_id, "archetype_id": second_arch }),
    )
    .await;
    assert_eq!(r2.status(), StatusCode::CREATED);
    let second_id = body_json(r2).await["id"]
        .as_str()
        .expect("second id")
        .to_string();

    // History must include both, newest (second) first.
    let hist_resp = common::get_with_auth(&app, "/programs/me", Some(&bearer(&token))).await;
    assert_eq!(hist_resp.status(), StatusCode::OK);
    let hist = body_json(hist_resp).await;
    let programs = hist["programs"].as_array().expect("`programs` array");
    assert_eq!(programs.len(), 2, "history must contain both programs");
    assert_eq!(
        programs[0]["id"].as_str(),
        Some(second_id.as_str()),
        "newest program must be first"
    );
    assert_eq!(
        programs[1]["id"].as_str(),
        Some(first_id.as_str()),
        "older program must be second"
    );
    // `total` must be present and match.
    assert_eq!(hist["total"].as_i64(), Some(2));
}

/// AC5: pagination — `limit=1` returns 1 item; `offset=1` skips the first.
#[sqlx::test(migrations = "../../migrations")]
async fn history_pagination_limit_offset(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (token, session_id) = seed_with_session_and_photo(&app, "hist-page@b.com").await;

    // Two chooses to have something to paginate.
    choose_first_proposal(&app, &token, &session_id).await;
    let proposals_resp = common::get_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/program-proposals"),
        Some(&bearer(&token)),
    )
    .await;
    let proposals = body_json(proposals_resp).await;
    let second_arch = proposals["proposals"][1]["archetype_id"]
        .as_str()
        .unwrap()
        .to_string();
    post_json_with_auth(
        &app,
        "/programs",
        Some(&bearer(&token)),
        json!({ "photo_session_id": session_id, "archetype_id": second_arch }),
    )
    .await;

    // limit=1 → exactly 1 program.
    let resp1 = common::get_with_auth(&app, "/programs/me?limit=1", Some(&bearer(&token))).await;
    assert_eq!(resp1.status(), StatusCode::OK);
    let body1 = body_json(resp1).await;
    assert_eq!(
        body1["programs"].as_array().unwrap().len(),
        1,
        "limit=1 must return exactly 1 program"
    );
    assert_eq!(
        body1["total"].as_i64(),
        Some(2),
        "`total` must still reflect all 2 programs"
    );

    // offset=1 → skips the first (newest) item.
    let resp2 = common::get_with_auth(&app, "/programs/me?offset=1", Some(&bearer(&token))).await;
    assert_eq!(resp2.status(), StatusCode::OK);
    let body2 = body_json(resp2).await;
    let programs2 = body2["programs"].as_array().unwrap();
    assert_eq!(programs2.len(), 1, "offset=1 must skip the first program");
    // The single result at offset=1 must be the OLDER (first-chosen) program —
    // i.e. NOT the same as the first item at offset=0.
    let item_at_0 = body1["programs"][0]["id"].as_str().unwrap();
    let item_at_1 = programs2[0]["id"].as_str().unwrap();
    assert_ne!(
        item_at_0, item_at_1,
        "offset=1 must return a different program than offset=0"
    );
}

/// AC6: no token → 401 on history.
#[sqlx::test(migrations = "../../migrations")]
async fn history_unauthenticated_is_401(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);

    let resp = common::get_with_auth(&app, "/programs/me", None).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// AC2 — template instantiation reflected in proposals
// ---------------------------------------------------------------------------

/// AC2: each proposal in the list carries a `program.days_per_week` field and a
/// `diet.estimated_kcal` field derived from the profile — both non-zero.
#[sqlx::test(migrations = "../../migrations")]
async fn proposals_contain_derived_program_and_diet_fields(pool: PgPool) {
    let fake = FakePoseEstimator::returning(wide_frame_pose());
    let (app, _store, _dir) = build_app_with_pose(pool, fake);
    let (token, session_id) = seed_with_session_and_photo(&app, "prop-derived@b.com").await;

    let resp = common::get_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/program-proposals"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    for p in body["proposals"].as_array().unwrap() {
        let dpw = p["program"]["days_per_week"]
            .as_u64()
            .expect("days_per_week must be a non-null u64");
        assert!(dpw > 0 && dpw <= 7, "days_per_week {dpw} must be in [1, 7]");
        let kcal = p["diet"]["estimated_kcal"]
            .as_u64()
            .expect("estimated_kcal must be a non-null u64");
        assert!(kcal > 0, "estimated_kcal must be > 0");
    }
}
