//! Auth surface: register, login, /auth/me, and password reset (R-0060).

mod extractor;
pub mod google;
mod handlers;
mod password;
mod reset;
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
        .route("/auth/password/forgot", post(handlers::forgot_password))
        .route("/auth/password/reset", post(handlers::reset_password))
}
