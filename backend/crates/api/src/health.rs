//! `GET /health` — the minimum readiness signal.

use axum::{http::StatusCode, routing::get, Router};

/// Generic over the router's state type so it composes into the stateful
/// application router (`Router<AppState>`) — the health route itself needs no
/// state.
pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/health", get(health))
}

async fn health() -> StatusCode {
    StatusCode::OK
}
