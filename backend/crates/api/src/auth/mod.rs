//! Auth surface: register, login, /auth/me.

mod extractor;
pub mod google;
mod handlers;
mod password;
mod token;

pub use extractor::AuthenticatedUser;
pub use google::{GoogleAuthSettings, GoogleIdTokenVerifier};

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(handlers::register))
        .route("/auth/login", post(handlers::login))
        .route("/auth/google", post(handlers::login_google))
        .route("/auth/me", get(handlers::me))
}
