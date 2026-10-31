//! R-0059 / SPEC-0059 — lift biomechanics over HTTP. Stateless: the request
//! carries the lifter's measurements, the response carries the model's
//! frames, and nothing is stored or logged (AC2, AC8).

mod handlers;
mod lever;
mod request;
mod response;

use axum::{routing::post, Router};

use crate::AppState;

/// `POST /biomech/squat/range`.
pub fn routes() -> Router<AppState> {
    Router::new().route("/biomech/squat/range", post(handlers::squat_range))
}
