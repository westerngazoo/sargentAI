//! Program + diet proposal endpoints (R-0014, SPEC-0014 §2.4).
//!
//! Routes:
//! - `GET /photo-sessions/:id/program-proposals` — top-3 proposals
//! - `POST /programs`                            — choose and persist
//! - `GET /programs/me/current`                  — active program
//! - `GET /programs/me`                          — history (paginated)

pub(crate) mod handlers;

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
        .route("/programs/me", get(handlers::get_history))
}
