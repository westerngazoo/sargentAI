//! Archetype-library read surface: `GET /archetypes` + `GET /archetypes/:id`.
//!
//! Static reference data served from `fitai_core::archetype::library()` — no DB,
//! no `AppState` field (SPEC-0012 §2.3). Authenticated, consistent with the
//! auth-gated app (§2.5).

mod handlers;

use axum::{routing::get, Router};

use crate::AppState;

/// The user-facing archetype wire shape — reused by the R-0013 match response
/// (`RankedArchetype` flattens it), so the privacy contract (no `internal_name`/
/// `sources`) holds on both surfaces.
pub(crate) use handlers::ArchetypeResponse;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/archetypes", get(handlers::list))
        .route("/archetypes/:id", get(handlers::get_one))
}
