//! R-0034 body-measurement integration suite — POST/GET /measurements.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::float_cmp)]
#![allow(clippy::doc_markdown)]

mod common;

use axum::http::StatusCode;
use common::{body_json, build_app, get_with_auth, post_json_with_auth, register_and_token};
use serde_json::{json, Value};
use sqlx::PgPool;

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_returns_derived_lean_mass(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;

    let res = post_json_with_auth(
        &app,
        "/measurements",
        Some(&bearer(&token)),
        json!({"measured_on": "2026-05-01", "weight_kg": 80.0, "body_fat_percentage": 20.0}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    assert_eq!(body["weight_kg"], 80.0);
    // lean = 80 * (1 - 0.20) = 64.0
    assert_eq!(body["lean_mass_kg"], 64.0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn lean_mass_is_null_without_body_fat(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;

    let res = post_json_with_auth(
        &app,
        "/measurements",
        Some(&bearer(&token)),
        json!({"measured_on": "2026-05-01", "weight_kg": 82.0}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    assert!(body["lean_mass_kg"].is_null());
}

#[sqlx::test(migrations = "../../migrations")]
async fn same_day_upserts_rather_than_conflicts(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;

    for w in [80.0, 79.0] {
        let res = post_json_with_auth(
            &app,
            "/measurements",
            Some(&bearer(&token)),
            json!({"measured_on": "2026-05-01", "weight_kg": w}),
        )
        .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    let res = get_with_auth(&app, "/measurements", Some(&bearer(&token))).await;
    let body = body_json(res).await;
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1, "same day upserts to one row");
    assert_eq!(arr[0]["weight_kg"], 79.0, "keeps the latest weight");
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_is_oldest_first_for_charting(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;

    for (day, w) in [("2026-05-15", 79.0), ("2026-05-01", 80.0)] {
        post_json_with_auth(
            &app,
            "/measurements",
            Some(&bearer(&token)),
            json!({"measured_on": day, "weight_kg": w}),
        )
        .await;
    }
    let res = get_with_auth(&app, "/measurements", Some(&bearer(&token))).await;
    let body: Value = body_json(res).await;
    let arr = body.as_array().unwrap();
    assert_eq!(arr[0]["measured_on"], "2026-05-01");
    assert_eq!(arr[1]["measured_on"], "2026-05-15");
}

#[sqlx::test(migrations = "../../migrations")]
async fn rejects_out_of_range_weight(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;
    let res = post_json_with_auth(
        &app,
        "/measurements",
        Some(&bearer(&token)),
        json!({"measured_on": "2026-05-01", "weight_kg": 5.0}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn requires_auth(pool: PgPool) {
    let app = build_app(pool);
    let res = get_with_auth(&app, "/measurements", Some("Bearer not-a-token")).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
