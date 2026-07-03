//! R-0033 Google Sign-In integration tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::{sync::Arc, time::Duration};

use axum::http::StatusCode;
use chrono::{Duration as ChronoDuration, Utc};
use common::{body_json, build_app_with_google, post_json, TEST_SECRET};
use fitai_api::auth::google::StaticGoogleVerifier;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

const TEST_AUD: &str = "test-google-client-id";
const TEST_EMAIL: &str = "google-user@example.com";

fn mint_google_token(email: &str, aud: &str, exp_offset: chrono::Duration) -> String {
    let pem = include_bytes!("fixtures/google_test_rsa.pem");
    let key = EncodingKey::from_rsa_pem(pem).unwrap();
    let exp = (Utc::now() + exp_offset).timestamp();
    let claims = json!({
        "iss": "https://accounts.google.com",
        "aud": aud,
        "email": email,
        "email_verified": true,
        "exp": exp,
    });
    encode(
        &Header::new(Algorithm::RS256),
        &claims,
        &key,
    )
    .unwrap()
}

fn google_app(pool: PgPool) -> axum::Router {
    let pub_pem = include_bytes!("fixtures/google_test_rsa_pub.pem");
    let verifier = Arc::new(StaticGoogleVerifier::from_rsa_pem(pub_pem));
    build_app_with_google(pool, TEST_AUD, verifier)
}

#[sqlx::test(migrations = "../../migrations")]
async fn google_sign_in_creates_user_and_returns_jwt(pool: PgPool) {
    let app = google_app(pool.clone());
    let token = mint_google_token(TEST_EMAIL, TEST_AUD, ChronoDuration::hours(1));

    let resp = post_json(
        &app,
        "/auth/google",
        json!({ "id_token": token }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = body_json(resp).await;
    assert!(body["token"].is_string());
    assert!(body["user_id"].is_string());

    let row = sqlx::query("SELECT password_hash FROM users WHERE email = $1")
        .bind(TEST_EMAIL)
        .fetch_one(&pool)
        .await
        .unwrap();
    let hash: Option<String> = row.get("password_hash");
    assert!(hash.is_none(), "Google-only user has no password hash");
}

#[sqlx::test(migrations = "../../migrations")]
async fn google_sign_in_reuses_existing_email(pool: PgPool) {
    let app = google_app(pool.clone());
    let token = mint_google_token(TEST_EMAIL, TEST_AUD, ChronoDuration::hours(1));

    let first = post_json(
        &app,
        "/auth/google",
        json!({ "id_token": token }),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first_body: Value = body_json(first).await;
    let user_id = first_body["user_id"].as_str().unwrap();

    let token2 = mint_google_token(TEST_EMAIL, TEST_AUD, ChronoDuration::hours(1));
    let second = post_json(
        &app,
        "/auth/google",
        json!({ "id_token": token2 }),
    )
    .await;
    assert_eq!(second.status(), StatusCode::OK);
    let second_body: Value = body_json(second).await;
    assert_eq!(second_body["user_id"].as_str().unwrap(), user_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn google_only_user_cannot_password_login(pool: PgPool) {
    let app = google_app(pool);
    let token = mint_google_token(TEST_EMAIL, TEST_AUD, ChronoDuration::hours(1));
    let resp = post_json(
        &app,
        "/auth/google",
        json!({ "id_token": token }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let login = post_json(
        &app,
        "/auth/login",
        json!({ "email": TEST_EMAIL, "password": "password123" }),
    )
    .await;
    assert_eq!(login.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn bad_google_token_returns_401(pool: PgPool) {
    let app = google_app(pool);
    let resp = post_json(
        &app,
        "/auth/google",
        json!({ "id_token": "not.a.jwt" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn expired_google_token_returns_401(pool: PgPool) {
    let app = google_app(pool);
    let token = mint_google_token(TEST_EMAIL, TEST_AUD, ChronoDuration::hours(-1));
    let resp = post_json(
        &app,
        "/auth/google",
        json!({ "id_token": token }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// Silence unused import warning for TEST_SECRET re-export sanity.
#[allow(dead_code)]
const _: &[u8] = TEST_SECRET;
