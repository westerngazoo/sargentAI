//! R-0030 synthetic body-type picker integration suite (retro backfill).
//!
//! Tests `POST /match/synthetic` (ranked proposals, no photo) and
//! `POST /programs/synthetic` (commit a chosen proposal with
//! `source_session_id = NULL`). Authored during R-0057 to backfill the backend
//! tests R-0030 shipped without; asserts against the CURRENT shipped behaviour
//! (no feature change).
//!
//! Every test is `#[sqlx::test(migrations = "../../migrations")]`, so sqlx
//! provisions a fresh per-test database and applies all migrations. The app is
//! built with `build_app` — the synthetic path never loads the ONNX model
//! (features come from the pure lookup table), so no pose injection is needed.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Test doc comments quote JSON keys and route paths as prose.
#![allow(clippy::doc_markdown)]

mod common;

use axum::http::StatusCode;
use common::{body_json, build_app, post_json_with_auth, register_and_token};
use serde_json::json;
use sqlx::PgPool;

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

/// Create a profile for `token`'s user — required before match/choose work
/// (both handlers `find_profile_by_user` and 404 without one). Mirrors the seed
/// used by the R-0014 program suite.
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

// ---------------------------------------------------------------------------
// POST /match/synthetic
// ---------------------------------------------------------------------------

/// Happy path: a valid shape+band → 200 with the top-3 ranked proposals,
/// echoing back the chosen shape and band, and never leaking internal labels.
#[sqlx::test(migrations = "../../migrations")]
async fn match_synthetic_returns_top3_proposals(pool: PgPool) {
    let app = build_app(pool);
    let (_, token) = register_and_token(&app, "syn-match-ok@b.com", "8charsmin").await;
    create_profile(&app, &token).await;

    let resp = post_json_with_auth(
        &app,
        "/match/synthetic",
        Some(&bearer(&token)),
        json!({ "shape": "mesomorph", "fat_band": "lean" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    assert_eq!(body["shape"], "mesomorph", "response echoes the shape");
    assert_eq!(body["fat_band"], "lean", "response echoes the fat band");

    let proposals = body["proposals"]
        .as_array()
        .expect("response must carry a `proposals` array");
    assert_eq!(proposals.len(), 3, "must return exactly the top-3");

    // Scores descend (nearest first).
    let scores: Vec<f64> = proposals
        .iter()
        .map(|p| p["score"].as_f64().expect("each proposal has a score"))
        .collect();
    for pair in scores.windows(2) {
        assert!(
            pair[0] >= pair[1],
            "proposals must be ordered score descending: {} before {}",
            pair[0],
            pair[1]
        );
    }

    for p in proposals {
        assert!(p.get("archetype_id").and_then(|v| v.as_str()).is_some());
        assert!(p.get("program").is_some(), "proposal must include program");
        assert!(p.get("diet").is_some(), "proposal must include diet");
    }

    // Privacy: no internal research labels on the wire.
    let serialized = body.to_string();
    assert!(
        !serialized.contains("internal_name"),
        "proposals must not expose internal_name"
    );
    for label in ["Yates", "Mentzer", "Arnold", "Columbu", "Cutler", "Heath"] {
        assert!(
            !serialized.contains(label),
            "proposals must not expose internal research label {label:?}"
        );
    }
}

/// No token → 401.
#[sqlx::test(migrations = "../../migrations")]
async fn match_synthetic_unauthenticated_is_401(pool: PgPool) {
    let app = build_app(pool);
    let resp = post_json_with_auth(
        &app,
        "/match/synthetic",
        None, // no auth header
        json!({ "shape": "mesomorph", "fat_band": "lean" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(resp).await, json!({ "error": "unauthorized" }));
}

/// Authenticated user with no profile → 404 (no profile to parameterise the
/// diet).
#[sqlx::test(migrations = "../../migrations")]
async fn match_synthetic_no_profile_is_404(pool: PgPool) {
    let app = build_app(pool);
    // Register but do NOT create a profile.
    let (_, token) = register_and_token(&app, "syn-match-noprof@b.com", "8charsmin").await;

    let resp = post_json_with_auth(
        &app,
        "/match/synthetic",
        Some(&bearer(&token)),
        json!({ "shape": "ectomorph", "fat_band": "moderate" }),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "missing profile must yield 404 on match/synthetic"
    );
    assert_eq!(body_json(resp).await, json!({ "error": "not_found" }));
}

/// An out-of-range shape choice (not one of the three enum variants) →
/// 422 from the JSON body extractor; no proposals are fabricated.
#[sqlx::test(migrations = "../../migrations")]
async fn match_synthetic_invalid_shape_is_rejected(pool: PgPool) {
    let app = build_app(pool);
    let (_, token) = register_and_token(&app, "syn-match-bad@b.com", "8charsmin").await;
    create_profile(&app, &token).await;

    let resp = post_json_with_auth(
        &app,
        "/match/synthetic",
        Some(&bearer(&token)),
        json!({ "shape": "cyborg", "fat_band": "lean" }),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "an out-of-range shape must be rejected, not matched"
    );
}

// ---------------------------------------------------------------------------
// POST /programs/synthetic
// ---------------------------------------------------------------------------

/// Fetch the top-3 for a shape+band and return the first proposal's
/// `archetype_id` (a slug guaranteed to be in-proposals).
async fn first_synthetic_archetype(app: &axum::Router, token: &str) -> String {
    let resp = post_json_with_auth(
        app,
        "/match/synthetic",
        Some(&bearer(token)),
        json!({ "shape": "mesomorph", "fat_band": "lean" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "match must precede choose");
    body_json(resp).await["proposals"][0]["archetype_id"]
        .as_str()
        .expect("first proposal must have an archetype_id")
        .to_string()
}

/// Happy path: choosing a top-3 archetype → 201 and the persisted row has
/// `source_session_id IS NULL` (no photo session backs a synthetic commit).
#[sqlx::test(migrations = "../../migrations")]
async fn choose_synthetic_commits_program_with_null_source_session(pool: PgPool) {
    // Keep a handle to the pool so we can inspect the row after the commit; the
    // app takes its own clone.
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "syn-choose-ok@b.com", "8charsmin").await;
    create_profile(&app, &token).await;

    let archetype_id = first_synthetic_archetype(&app, &token).await;

    let resp = post_json_with_auth(
        &app,
        "/programs/synthetic",
        Some(&bearer(&token)),
        json!({
            "archetype_id": archetype_id,
            "shape": "mesomorph",
            "fat_band": "lean"
        }),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "choose/synthetic must return 201"
    );
    let body = body_json(resp).await;
    assert_eq!(
        body["active"].as_bool(),
        Some(true),
        "the committed program must be active"
    );
    assert_eq!(body["archetype_id"].as_str(), Some(archetype_id.as_str()));
    assert!(body.get("program").is_some());
    assert!(body.get("diet").is_some());

    // The persisted row must have a NULL source_session_id.
    let program_id = uuid::Uuid::parse_str(body["id"].as_str().expect("response id"))
        .expect("id must be a uuid");
    let source_session_id: Option<uuid::Uuid> =
        sqlx::query_scalar("SELECT source_session_id FROM user_programs WHERE id = $1")
            .bind(program_id)
            .fetch_one(&pool)
            .await
            .expect("the committed program row must be readable");
    assert!(
        source_session_id.is_none(),
        "a synthetic commit must persist source_session_id = NULL"
    );

    // Sanity: the row belongs to the caller.
    let owner: uuid::Uuid = sqlx::query_scalar("SELECT user_id FROM user_programs WHERE id = $1")
        .bind(program_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(owner.to_string(), user_id, "row must belong to the caller");
}

/// Choosing an archetype that is NOT in this selection's top-3 → 409 with
/// `{"error": "conflict", "reason": "archetype_not_in_proposals"}`.
#[sqlx::test(migrations = "../../migrations")]
async fn choose_synthetic_archetype_not_in_top3_is_409(pool: PgPool) {
    let app = build_app(pool);
    let (_, token) = register_and_token(&app, "syn-choose-bad@b.com", "8charsmin").await;
    create_profile(&app, &token).await;

    let resp = post_json_with_auth(
        &app,
        "/programs/synthetic",
        Some(&bearer(&token)),
        json!({
            "archetype_id": "does-not-exist-slug",
            "shape": "mesomorph",
            "fat_band": "lean"
        }),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "an out-of-proposals archetype must yield 409"
    );
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "conflict", "reason": "archetype_not_in_proposals" })
    );
}

/// No token → 401.
#[sqlx::test(migrations = "../../migrations")]
async fn choose_synthetic_unauthenticated_is_401(pool: PgPool) {
    let app = build_app(pool);
    let resp = post_json_with_auth(
        &app,
        "/programs/synthetic",
        None, // no auth
        json!({
            "archetype_id": "heavy-duty-mass",
            "shape": "mesomorph",
            "fat_band": "lean"
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(resp).await, json!({ "error": "unauthorized" }));
}

/// Authenticated user with no profile → 404 on choose.
#[sqlx::test(migrations = "../../migrations")]
async fn choose_synthetic_no_profile_is_404(pool: PgPool) {
    let app = build_app(pool);
    // Register but do NOT create a profile.
    let (_, token) = register_and_token(&app, "syn-choose-noprof@b.com", "8charsmin").await;

    let resp = post_json_with_auth(
        &app,
        "/programs/synthetic",
        Some(&bearer(&token)),
        json!({
            "archetype_id": "heavy-duty-mass",
            "shape": "mesomorph",
            "fat_band": "lean"
        }),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "missing profile must yield 404 on choose/synthetic"
    );
    assert_eq!(body_json(resp).await, json!({ "error": "not_found" }));
}
