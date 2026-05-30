//! Shared test harness for the R-0002 auth integration suite.
//!
//! Authored by the qa agent during R-0002 step 3. These helpers build an
//! `AppState` around the per-test `PgPool` that `#[sqlx::test]` hands in, with
//! a known `JWT_SECRET` and a caller-chosen token TTL (so a test can mint an
//! already-expired token by passing `Duration::ZERO`).

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::{sync::Arc, time::Duration};

use axum::{
    body::Body,
    http::{Request, Response},
    Router,
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

use fitai_api::{app, AppState};

/// Stable secret the whole suite signs/decodes with. SAC4 asserts that a
/// *different* secret fails signature verification.
pub const TEST_SECRET: &[u8] = b"qa-test-secret-r0002";

/// 24h, the production TTL — used by every test except the expiry case.
pub const TTL_24H: Duration = Duration::from_secs(60 * 60 * 24);

/// Build a router over the supplied pool with the canonical test secret and a
/// chosen TTL.
pub fn app_with_ttl(pool: PgPool, ttl: Duration) -> Router {
    let state = AppState {
        pool,
        jwt_secret: Arc::from(TEST_SECRET.to_vec().into_boxed_slice()),
        jwt_ttl: ttl,
    };
    app(state)
}

/// Build a router with the production 24h TTL.
pub fn build_app(pool: PgPool) -> Router {
    app_with_ttl(pool, TTL_24H)
}

/// POST a JSON body to `path` and return the raw response.
pub async fn post_json(app: &Router, path: &str, body: Value) -> Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// GET `path` with an optional raw `Authorization` header value.
pub async fn get_with_auth(app: &Router, path: &str, auth: Option<&str>) -> Response<Body> {
    let mut builder = Request::builder().method("GET").uri(path);
    if let Some(value) = auth {
        builder = builder.header("authorization", value);
    }
    app.clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// Drain a response body into raw bytes.
pub async fn body_bytes(resp: Response<Body>) -> Vec<u8> {
    resp.into_body().collect().await.unwrap().to_bytes().to_vec()
}

/// Drain a response body and parse it as JSON.
pub async fn body_json(resp: Response<Body>) -> Value {
    let bytes = body_bytes(resp).await;
    serde_json::from_slice(&bytes).expect("response body must be valid JSON")
}

/// Register a user and return its `user_id` string. Convenience for tests that
/// need a seeded account before exercising login / `/auth/me`.
pub async fn register_user(app: &Router, email: &str, password: &str) -> String {
    let resp = post_json(
        app,
        "/auth/register",
        serde_json::json!({ "email": email, "password": password }),
    )
    .await;
    assert_eq!(
        resp.status(),
        axum::http::StatusCode::CREATED,
        "seed register expected 201"
    );
    body_json(resp).await["user_id"]
        .as_str()
        .expect("register response must carry a string user_id")
        .to_string()
}
