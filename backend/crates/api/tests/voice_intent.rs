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
async fn voice_intent_multi_turn_conversation_logs_meal(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, "voice-multi@test.com", "password123").await;

    // Send the first turn missing context
    let resp1 = post_json_with_auth(
        &app,
        "/voice/intent",
        Some(&format!("Bearer {token}")),
        json!({ "transcript": "log a meal" }),
    )
    .await;
    assert_eq!(resp1.status(), StatusCode::OK);
    let body1: Value = body_json(resp1).await;
    assert_eq!(body1["status"], "clarify");

    // Simulate user responding to the clarify with the missing macros and passing history
    let resp2 = post_json_with_auth(
        &app,
        "/voice/intent",
        Some(&format!("Bearer {token}")),
        json!({
            "transcript": "40 grams protein 60 carbs 20 fat",
            "history": [
                { "role": "user", "content": "log a meal" },
                { "role": "assistant", "content": body1["prompt"].as_str().unwrap_or("") }
            ]
        }),
    )
    .await;
    assert_eq!(resp2.status(), StatusCode::OK);
    let body2: Value = body_json(resp2).await;

    // In CI (where no LLM is present), it falls back to the keyword parser.
    // The keyword parser handles "40 grams protein..." independently (it matches LogMealIntent without needing the earlier 'log a meal').
    // So whether via LLM combining context, or regex matching the second utterance directly, we expect it to log.
    assert_eq!(body2["status"], "logged_nutrition");
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
