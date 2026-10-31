//! Program + diet proposal endpoints (R-0014, SPEC-0014 §2.4).
//!
//! Routes:
//! - `GET /photo-sessions/:id/program-proposals` — top-3 proposals
//! - `POST /programs`                            — choose and persist
//! - `GET /programs/me/current`                  — active program
//! - `GET /programs/me`                          — history (paginated)
//! - `GET /programs/me/schedule`                 — the plan projection (R-0061)

pub(crate) mod handlers;
pub(crate) mod schedule;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/photo-sessions/:id/program-proposals",
            get(handlers::get_proposals),
        )
        .route("/programs", post(handlers::choose_program))
        .route("/programs/me/current", get(handlers::get_current))
        .route("/programs/me/schedule", get(handlers::get_schedule))
        .route("/programs/me", get(handlers::get_history))
}
