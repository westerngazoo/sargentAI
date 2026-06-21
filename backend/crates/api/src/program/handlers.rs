//! Route handlers for the program proposal + choose flow (R-0014, SPEC-0014
//! §2.4).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use fitai_core::archetype::library;
use fitai_core::matching::{rank, RankedMatch};
use fitai_core::pose::derive_frame_features;
use fitai_core::program::{instantiate, GeneratedDiet, GeneratedProgram, ProgramProposal};
use fitai_core::Profile;

use crate::{
    auth::AuthenticatedUser,
    db::{self, UserProgramRow},
    error::{ApiError, ApiResult},
    AppState,
};

// ---------------------------------------------------------------------------
// Wire shapes
// ---------------------------------------------------------------------------

/// Request body for `POST /programs`.
#[derive(Deserialize)]
pub(crate) struct ChooseRequest {
    pub photo_session_id: Uuid,
    pub archetype_id: String,
}

/// Pagination query for `GET /programs/me`.
#[derive(Deserialize)]
pub(crate) struct HistoryQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

/// The wire response for a single persisted program row.
#[derive(Serialize)]
pub(crate) struct UserProgramResponse {
    pub id: Uuid,
    pub archetype_id: String,
    pub source_session_id: Option<Uuid>,
    pub program: GeneratedProgram,
    pub diet: GeneratedDiet,
    pub active: bool,
    pub chosen_at: chrono::DateTime<Utc>,
}

impl TryFrom<UserProgramRow> for UserProgramResponse {
    type Error = ApiError;

    fn try_from(row: UserProgramRow) -> Result<Self, Self::Error> {
        let program: GeneratedProgram = serde_json::from_value(row.program)
            .map_err(|e| ApiError::Internal(eyre::eyre!("deserialise program: {e}")))?;
        let diet: GeneratedDiet = serde_json::from_value(row.diet)
            .map_err(|e| ApiError::Internal(eyre::eyre!("deserialise diet: {e}")))?;
        Ok(Self {
            id: row.id,
            archetype_id: row.archetype_id,
            source_session_id: row.source_session_id,
            program,
            diet,
            active: row.active,
            chosen_at: row.chosen_at,
        })
    }
}

/// Response for `GET /programs/me`.
#[derive(Serialize)]
pub(crate) struct HistoryResponse {
    pub programs: Vec<UserProgramResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

// ---------------------------------------------------------------------------
// Shared helper (SPEC-0014 §2.4.1)
// ---------------------------------------------------------------------------

/// Run the ONNX matching pipeline once and return the top-3 proposals.
///
/// Delegates to [`crate::matching::estimate_first_usable`] so the ONNX model
/// is invoked at most once per request regardless of which endpoint calls this.
async fn derive_proposals(
    state: &AppState,
    user_id: fitai_core::UserId,
    session_id: Uuid,
    profile: &Profile,
) -> ApiResult<Vec<ProgramProposal>> {
    if !db::session_exists_for_user(&state.pool, user_id, session_id).await? {
        return Err(ApiError::NotFound);
    }
    let candidates = db::match_candidates_for_session(&state.pool, user_id, session_id).await?;
    if candidates.is_empty() {
        return Err(ApiError::Unprocessable {
            reason: "no_usable_photo",
        });
    }

    let keypoints = crate::matching::estimate_first_usable(state, candidates).await?;
    let features = derive_frame_features(&keypoints)?;
    let today = Utc::now().date_naive();

    let proposals: Vec<ProgramProposal> = top3(rank(&features, library()))
        .into_iter()
        .map(|m| {
            let score = 1.0 - m.distance;
            instantiate(m.archetype, profile, score, m.distance, today)
        })
        .collect();

    Ok(proposals)
}

/// Keep the three nearest-ranked matches.
fn top3(ranked: Vec<RankedMatch>) -> Vec<RankedMatch> {
    ranked.into_iter().take(3).collect()
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /photo-sessions/:id/program-proposals`
///
/// Returns the top-3 archetype proposals (program + diet) derived from the
/// session's best photo. Requires an existing profile.
pub(crate) async fn get_proposals(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(session_id): Path<Uuid>,
) -> ApiResult<Json<Vec<ProgramProposal>>> {
    let profile = require_profile(&state, user.user_id).await?;
    let proposals = derive_proposals(&state, user.user_id, session_id, &profile).await?;
    Ok(Json(proposals))
}

/// `POST /programs`
///
/// Chooses one of the top-3 proposals, deactivates any existing active
/// program, and inserts the new one. Returns 201 on success.
///
/// Returns 409 if the chosen `archetype_id` was not among the session's top-3.
pub(crate) async fn choose_program(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<ChooseRequest>,
) -> ApiResult<(StatusCode, Json<UserProgramResponse>)> {
    let profile = require_profile(&state, user.user_id).await?;
    let proposals = derive_proposals(&state, user.user_id, body.photo_session_id, &profile).await?;

    let chosen = proposals
        .into_iter()
        .find(|p| p.archetype_id == body.archetype_id)
        .ok_or(ApiError::Conflict {
            reason: "archetype_not_in_proposals",
        })?;

    let row = db::insert_program(
        &state.pool,
        user.user_id,
        &chosen.archetype_id,
        Some(body.photo_session_id),
        &chosen.program,
        &chosen.diet,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(UserProgramResponse::try_from(row)?),
    ))
}

/// `GET /programs/me/current`
///
/// Returns the caller's active program (200) or 404 if none exists.
pub(crate) async fn get_current(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<UserProgramResponse>> {
    let row = db::get_current_program(&state.pool, user.user_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(UserProgramResponse::try_from(row)?))
}

/// `GET /programs/me`
///
/// Returns the caller's program history, newest first, with limit/offset
/// pagination.
pub(crate) async fn get_history(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(q): Query<HistoryQuery>,
) -> ApiResult<Json<HistoryResponse>> {
    let (rows, total) =
        db::get_program_history(&state.pool, user.user_id, q.limit, q.offset).await?;

    let programs: Result<Vec<_>, _> = rows
        .into_iter()
        .map(UserProgramResponse::try_from)
        .collect();
    Ok(Json(HistoryResponse {
        programs: programs?,
        total,
        limit: q.limit,
        offset: q.offset,
    }))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

async fn require_profile(state: &AppState, user_id: fitai_core::UserId) -> ApiResult<Profile> {
    db::find_profile_by_user(&state.pool, user_id)
        .await?
        .ok_or(ApiError::NotFound)
}
