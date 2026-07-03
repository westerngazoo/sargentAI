//! R-0032 slice 2 — voice intent parsing and auto-log.

mod handlers;
mod parse;

pub use handlers::VoiceIntentSettings;

use axum::{routing::post, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/voice/intent", post(handlers::intent))
}
