//! Integration tests for `GET /health`.
//!
//! Authored by qa agent during R-0001 step 3 (test planning).
//! Pre-implementation red state = compile failure: the `fitai_api` crate
//! does not exist yet. Implementation step 5 (SPEC-0001 §3.4–§3.6) makes
//! these green.
//!
//! Two tests: one via the in-process router (fast, no port), one via a
//! real `axum::serve` boot on `127.0.0.1:0` (literal AC2: "boots the
//! HTTP service in-process"). Both must pass.

#![allow(clippy::unwrap_used)]

use std::{sync::Arc, time::Duration};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

use fitai_api::{app, AppState};

/// Build an `AppState` with a *lazy* pool: the health route never touches the
/// database, so `connect_lazy` lets these tests run without a live Postgres.
fn health_app() -> axum::Router {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/fitai_health_test")
        .unwrap();
    let store = Arc::new(fitai_api::storage::LocalObjectStore::new(
        std::env::temp_dir().join("fitai-health-store"),
    ));
    app(AppState {
        pool,
        jwt_secret: Arc::from(b"health-test-secret".to_vec().into_boxed_slice()),
        jwt_ttl: Duration::from_hours(24),
        store,
        // The health route never estimates a pose; a default fake satisfies the
        // R-0013 `AppState.pose` field (mirrors the `store` field above).
        pose: Arc::new(fitai_api::pose::FakePoseEstimator::default()),
    })
}

#[tokio::test]
async fn health_returns_ok_via_router() {
    let app = health_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(body.is_empty(), "health body should be empty");
}

#[tokio::test]
async fn health_returns_ok_via_real_server() {
    // Bind ephemeral port, capture address, hand listener to axum::serve.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, health_app()).await.unwrap();
    });

    let url = format!("http://{addr}/health");
    let response = reqwest::get(&url).await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    server.abort();
}
