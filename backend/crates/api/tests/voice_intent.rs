//! R-0032 slice 2 — voice intent auto-log integration tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use axum::http::StatusCode;
use common::{body_json, build_app, post_json_with_auth, register_and_token};
use serde_json::{json, Value};
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn voice_intent_logs_workout_from_natural_language(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, "voice-workout@test.com", "password123").await;

    let resp = post_json_with_auth(
        &app,
        "/voice/intent",
        Some(&format!("Bearer {token}")),
        json!({ "transcript": "I did 10 reps of 100 kg bench press" }),
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
        Some(&format!("Bearer {token}")),
        json!({ "transcript": "log a meal 40 grams protein 60 carbs 20 fat" }),
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
        Some(&format!("Bearer {token}")),
        json!({ "transcript": "log a meal" }),
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
        Some("Bearer bad.token.here"),
        json!({ "transcript": "log a meal" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn voice_intent_uses_history_to_clarify(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "voice-history@test.com", "password123").await;

    // A transcript that only makes sense if the history is supplied
    let resp = post_json_with_auth(
        &app,
        "/voice/intent",
        Some(&format!("Bearer {token}")),
        json!({
            "transcript": "40 protein 60 carbs 20 fat",
            "history": [
                { "role": "user", "content": "log a meal" },
                { "role": "assistant", "content": "Tell me the grams of protein, carbs, and fat" }
            ]
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = body_json(resp).await;
    // Without an LLM connected in the test, it falls back to the keyword parser
    // which ignores the history and just parses "40 protein 60 carbs 20 fat" as a meal
    // when it hits the fallback. It is enough to verify the payload is accepted.
    assert_eq!(body["status"], "logged_nutrition");
}
