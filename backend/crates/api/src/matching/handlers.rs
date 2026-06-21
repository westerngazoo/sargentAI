//! `POST /photo-sessions/:id/match` (R-0013, SPEC-0013 §2.5).
//!
//! Runs server-side pose estimation over a session's photos, derives the
//! matchable frame profile, and returns the archetype library ranked
//! nearest-first. The wire shape ([`RankedArchetype`]) flattens R-0012's
//! `ArchetypeResponse`, so `internal_name`/`sources` never cross the wire.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use fitai_core::archetype::library;
use fitai_core::matching::{rank, RankedMatch};
use fitai_core::pose::derive_frame_features;

use crate::{
    archetype::ArchetypeResponse,
    auth::AuthenticatedUser,
    db,
    error::{ApiError, ApiResult},
    AppState,
};

/// The ranked match wire shape: every library archetype scored against the
/// caller's photo, nearest first.
#[derive(Serialize)]
pub(crate) struct MatchResponse {
    matches: Vec<RankedArchetype>,
}

/// One archetype in the ranking — the user-facing `ArchetypeResponse` fields
/// plus the match `distance` and its `score` (`1 - distance`).
#[derive(Serialize)]
struct RankedArchetype {
    #[serde(flatten)]
    archetype: ArchetypeResponse<'static>,
    distance: f64,
    score: f64,
}

impl From<RankedMatch> for RankedArchetype {
    fn from(m: RankedMatch) -> Self {
        Self {
            archetype: ArchetypeResponse::from(m.archetype),
            distance: m.distance,
            score: 1.0 - m.distance,
        }
    }
}

impl MatchResponse {
    fn from_ranked(ranked: Vec<RankedMatch>) -> Self {
        Self {
            matches: ranked.into_iter().map(RankedArchetype::from).collect(),
        }
    }
}

pub(crate) async fn match_session(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(session_id): Path<Uuid>,
) -> ApiResult<Json<MatchResponse>> {
    // Ownership first: a missing or foreign session is 404 (never 403), before
    // any photo bytes are read.
    if !db::session_exists_for_user(&state.pool, user.user_id, session_id).await? {
        return Err(ApiError::NotFound);
    }

    let candidates =
        db::match_candidates_for_session(&state.pool, user.user_id, session_id).await?;
    if candidates.is_empty() {
        return Err(ApiError::Unprocessable {
            reason: "no_usable_photo",
        });
    }

    let keypoints = super::estimate_first_usable(&state, candidates).await?;
    let features = derive_frame_features(&keypoints)?; // FrameError → 422 degenerate_frame
    Ok(Json(MatchResponse::from_ranked(rank(&features, library()))))
}
