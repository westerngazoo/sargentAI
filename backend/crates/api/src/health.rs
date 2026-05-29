//! `GET /health` — the minimum readiness signal.

use axum::{http::StatusCode, routing::get, Router};

pub(crate) fn router() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> StatusCode {
    StatusCode::OK
}
