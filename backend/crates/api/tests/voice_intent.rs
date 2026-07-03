//! R-0032 slice 2 — voice intent auto-log integration tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use axum::http::StatusCode;
use common::{body_json, build_app, post_json_with_auth, register_and_token};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

#[sqlx::test(migrations = "../../migrations")]
async fn voice_intent_logs_workout_from_natural_language(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, "voice-workout@test.com", "password123").await;

    let resp = post_json_with_auth(
        &app,
        "/voice/intent",
        json!({ "transcript": "I did 10 reps of 100 kg bench press" }),
        &token,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = body_json(resp).await;
    assert_eq!(body["status"], "logged_workout");
    assert!(body["record_id"].is_string());

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workout_sessions ws \
         JOIN users u ON u.id = ws.user_id WHERE u.email = $1",
    )
    .bind("voice-workout@test.com")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn voice_intent_logs_meal_when_macros_present(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, "voice-meal@test.com", "password123").await;

    let resp = post_json_with_auth(
        &app,
        "/voice/intent",
        json!({ "transcript": "log a meal 40 grams protein 60 carbs 20 fat" }),
        &token,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = body_json(resp).await;
    assert_eq!(body["status"], "logged_nutrition");
}

#[sqlx::test(migrations = "../../migrations")]
async fn voice_intent_clarifies_incomplete_meal(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "voice-clarify@test.com", "password123").await;

    let resp = post_json_with_auth(
        &app,
        "/voice/intent",
        json!({ "transcript": "log a meal" }),
        &token,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = body_json(resp).await;
    assert_eq!(body["status"], "clarify");
    assert!(body["prompt"].is_string());
}

#[sqlx::test(migrations = "../../migrations")]
async fn voice_intent_requires_auth(pool: PgPool) {
    let app = build_app(pool);
    let resp = post_json_with_auth(
        &app,
        "/voice/intent",
        json!({ "transcript": "log a meal" }),
        "bad.token.here",
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
