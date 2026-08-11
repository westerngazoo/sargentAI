//! R-0042 / SPEC-0042 — goal targets & pace reports over HTTP.
//!
//! Self-contained module (like `authored`/`measurements`): its own request and
//! response DTOs and inline sqlx. Storage is one JSONB column holding the whole
//! `core::goals::Goal`; every read recomputes the pace report from the trailing
//! signal — no derived state is stored (AC8).

pub(crate) mod handlers;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/goals", post(handlers::create).get(handlers::list))
        // `:id`, not `{id}` — axum 0.7 (matchit 0.7) treats braces as a literal
        // segment, so `{id}` would never match a real UUID.
        .route(
            "/goals/:id",
            get(handlers::fetch_one).delete(handlers::delete),
        )
}
