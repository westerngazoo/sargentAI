//! Body-measurement log for progress charts (R-0034). Self-contained module
//! (like `synthetic`/`foods`): its own request/response DTOs and inline sqlx,
//! no `core` write-model. `POST /measurements` upserts one row per day;
//! `GET /measurements` lists them oldest-first for charting.

pub(crate) mod handlers;

use axum::{routing::post, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/measurements", post(handlers::create).get(handlers::list))
}
