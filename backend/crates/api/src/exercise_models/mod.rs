//! R-0066 / SPEC-0066 §2.4 — the exercise model library over HTTP: the list,
//! each exercise's description, and its range. Stateless, as SPEC-0059: the
//! request carries the body, and nothing is stored or logged.

mod handlers;
mod request;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

/// `GET /exercise-models`, `GET /exercise-models/:id`,
/// `POST /exercise-models/:id/range`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/exercise-models", get(handlers::list))
        .route("/exercise-models/:id", get(handlers::describe))
        .route("/exercise-models/:id/range", post(handlers::range))
}
