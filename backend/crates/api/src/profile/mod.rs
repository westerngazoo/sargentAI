//! Profile surface: GET/PUT /profile/me.

mod handlers;

use axum::{routing::get, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/profile/me", get(handlers::get_me).put(handlers::put_me))
}
