//! Authored-program persistence and serving (R-0041 / SPEC-0041).
//!
//! Self-contained module (like `measurements`/`synthetic`): its own request and
//! response DTOs and inline sqlx, no `core` write-model. Storage is one JSONB
//! column holding the whole [`AuthoredProgram`](fitai_core::AuthoredProgram), so
//! the type that goes in is the type that comes out. Every read is scoped
//! `WHERE id = $1 AND user_id = $2`, which makes a foreign id indistinguishable
//! from a missing one (404, never 403).

pub(crate) mod handlers;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/authored-programs",
            post(handlers::create).get(handlers::list),
        )
        // `:id`, not `{id}` — axum 0.7 (matchit 0.7) treats braces as a literal
        // segment, so `{id}` would never match a real UUID. Brace captures are
        // an axum 0.8 feature. Matches the convention in `archetype`/`photo`.
        .route("/authored-programs/:id", get(handlers::fetch_one))
        .route(
            "/authored-programs/:id/materialized",
            get(handlers::materialized),
        )
}
