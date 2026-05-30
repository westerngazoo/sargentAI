//! Auth surface: register, login, /auth/me.

mod extractor;
mod handlers;
mod password;
mod token;

pub use extractor::AuthenticatedUser;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(handlers::register))
        .route("/auth/login", post(handlers::login))
        .route("/auth/me", get(handlers::me))
}
