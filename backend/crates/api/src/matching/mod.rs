//! Photo→archetype match surface: `POST /photo-sessions/:id/match` (R-0013,
//! SPEC-0013 §2.5).

mod handlers;

use axum::{routing::post, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/photo-sessions/:id/match", post(handlers::match_session))
}
