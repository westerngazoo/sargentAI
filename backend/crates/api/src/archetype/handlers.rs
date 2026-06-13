//! HTTP handlers for the read-only archetype library (R-0012, SPEC-0012 §2.5).
//!
//! The wire shape is owned by [`ArchetypeResponse`], which deliberately omits
//! the internal research label and the curation sources (AC4): the core
//! `Archetype` is not `Serialize`, so those fields cannot leak. Only the
//! provenance `confidence` level crosses the wire.

use axum::{extract::Path, Json};
use serde::Serialize;

use fitai_core::archetype::{
    find, library, Archetype, Confidence, DietTemplate, FrameProfile, ProgramTemplate,
};
use fitai_core::Goal;

use crate::{
    auth::AuthenticatedUser,
    error::{ApiError, ApiResult},
};

/// The user-facing archetype wire shape. Borrows from the `'static` library and
/// carries only abstracted content plus the provenance `confidence` — never the
/// `internal_name` or the curation `sources` (AC4).
#[derive(Debug, Serialize)]
pub(crate) struct ArchetypeResponse<'a> {
    id: &'a str,
    display_name: &'a str,
    summary: &'a str,
    frame_profile: &'a FrameProfile,
    program_template: &'a ProgramTemplate,
    diet_template: &'a DietTemplate,
    confidence: Confidence,
    goals_served: &'a [Goal],
}

impl<'a> From<&'a Archetype> for ArchetypeResponse<'a> {
    fn from(a: &'a Archetype) -> Self {
        Self {
            id: a.id,
            display_name: &a.display_name,
            summary: &a.summary,
            frame_profile: &a.frame_profile,
            program_template: &a.program_template,
            diet_template: &a.diet_template,
            confidence: a.provenance.confidence,
            goals_served: &a.goals_served,
        }
    }
}

/// `GET /archetypes` — the whole library in authored order (authenticated).
pub(crate) async fn list(_user: AuthenticatedUser) -> Json<Vec<ArchetypeResponse<'static>>> {
    Json(library().iter().map(ArchetypeResponse::from).collect())
}

/// `GET /archetypes/:id` — one archetype by slug; `404` for an unknown id.
pub(crate) async fn get_one(
    _user: AuthenticatedUser,
    Path(id): Path<String>,
) -> ApiResult<Json<ArchetypeResponse<'static>>> {
    find(&id)
        .map(|a| Json(ArchetypeResponse::from(a)))
        .ok_or(ApiError::NotFound)
}
