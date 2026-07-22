//! R-0015 / R-0017 — training summary + coach suggestions endpoints.
//!
//! Self-contained module in the measurements/synthetic style: one router, one
//! handlers file. Both endpoints share a single fetch path (sessions,
//! measurements, active program) and call the pure `fitai-core` layers —
//! `aggregate::summarize` for facts, `adjust::suggest` for suggestions.

mod handlers;

use axum::{routing::get, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/training-summary", get(handlers::training_summary))
        .route("/adjustments", get(handlers::adjustments))
}
