//! R-0015 AC8 / R-0017 AC7 — `GET /training-summary` and `GET /adjustments`.
//!
//! Every test is `#[sqlx::test(migrations = "../../migrations")]` (fresh DB per
//! test). Sessions are seeded through the real `/workouts` endpoint with dates
//! relative to today so they land inside the 8-week window regardless of when
//! the suite runs.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::doc_markdown)]

mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use common::{body_json, build_app, get_with_auth, post_json_with_auth, register_and_token};
use serde_json::{json, Value};
use sqlx::PgPool;

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

/// A dated bench session `days_ago` days back, at a fixed load (stall fodder).
fn bench_session(days_ago: i64) -> Value {
    let on = (Utc::now().date_naive() - Duration::days(days_ago)).to_string();
    json!({
        "performed_on": on,
        "exercises": [{
            "name": "Bench Press",
            "muscle_group": "chest",
            "sets": [{ "reps": 5, "weight_kg": 100.0 }]
        }]
    })
}

async fn seed_profile(app: &axum::Router, token: &str) {
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
    assert!(resp.status() == StatusCode::OK || resp.status() == StatusCode::CREATED);
}

/// Match + choose a synthetic program so the user has an active program.
async fn seed_program(app: &axum::Router, token: &str) {
    let matched = post_json_with_auth(
        app,
        "/match/synthetic",
        Some(&bearer(token)),
        json!({ "shape": "mesomorph", "fat_band": "lean" }),
    )
    .await;
    assert_eq!(matched.status(), StatusCode::OK);
    let body = body_json(matched).await;
    let archetype_id = body["proposals"][0]["archetype_id"]
        .as_str()
        .unwrap()
        .to_string();
    let chosen = post_json_with_auth(
        app,
        "/programs/synthetic",
        Some(&bearer(token)),
        json!({ "archetype_id": archetype_id, "shape": "mesomorph", "fat_band": "lean" }),
    )
    .await;
    assert_eq!(chosen.status(), StatusCode::CREATED);
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn both_endpoints_require_auth(pool: PgPool) {
    let app = build_app(pool);
    for path in ["/training-summary", "/adjustments"] {
        let res = get_with_auth(&app, path, None).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "{path}");
    }
}

// ---------------------------------------------------------------------------
// Empty user
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn empty_user_gets_well_formed_summary(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;
    let res = get_with_auth(&app, "/training-summary", Some(&bearer(&token))).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["window_weeks"], 8);
    assert_eq!(body["lifts"].as_array().unwrap().len(), 0);
    assert_eq!(body["muscle_volume"].as_array().unwrap().len(), 0);
    assert_eq!(body["adherence"]["ratio"], 0.0);
    assert!(body["body"]["body_fat_slope"].is_null());
}

#[sqlx::test(migrations = "../../migrations")]
async fn adjustments_without_program_say_why(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;
    let res = get_with_auth(&app, "/adjustments", Some(&bearer(&token))).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["suggestions"].as_array().unwrap().len(), 0);
    assert_eq!(body["reason"], "no_active_program");
}

// ---------------------------------------------------------------------------
// Seeded: a stalled bench across four weeks
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn stalled_bench_shows_in_summary_and_earns_a_deload(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;
    seed_profile(&app, &token).await;
    seed_program(&app, &token).await;

    // Four weekly sessions at the same load → stalled per SPEC-0015 (no new
    // peak in the last STALL_N sessions), inside the 8-week window.
    for days_ago in [28, 21, 14, 7] {
        let res = post_json_with_auth(
            &app,
            "/workouts",
            Some(&bearer(&token)),
            bench_session(days_ago),
        )
        .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    // The summary reports the facts.
    let res = get_with_auth(&app, "/training-summary", Some(&bearer(&token))).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let lift = &body["lifts"][0];
    assert_eq!(lift["name"], "Bench Press");
    assert_eq!(lift["sessions"], 4);
    assert_eq!(lift["stalled"], true);
    assert_eq!(body["muscle_volume"][0]["group"], "chest");

    // The engine turns the stall into a deload suggestion.
    let res = get_with_auth(&app, "/adjustments", Some(&bearer(&token))).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert!(body["reason"].is_null());
    let suggestions = body["suggestions"].as_array().unwrap();
    assert!(!suggestions.is_empty());
    let deload = suggestions
        .iter()
        .find(|s| s["change"]["kind"] == "deload_lift")
        .expect("expected a deload suggestion for the stalled bench");
    assert_eq!(deload["change"]["lift"], "Bench Press");
    assert_eq!(deload["severity"], "action");
    assert!(deload["rationale"]
        .as_str()
        .unwrap()
        .contains("Bench Press"));
}
