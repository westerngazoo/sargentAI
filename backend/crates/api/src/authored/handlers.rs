//! Authored-program handlers — inline sqlx, one ownership-scoped read.
//!
//! Extractor order is an invariant, not a style choice: axum runs extractors
//! left to right, so `AuthenticatedUser` must precede `Path`/`Query`/`Json` in
//! every signature. Reversed, `GET /authored-programs/not-a-uuid` with no token
//! would return axum's 400 instead of AC8's 401.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use fitai_core::{
    authoring::validate, current_e1rm, materialize, AuthorError, AuthoredProgram,
    MaterializedCycle, UserId, DEFAULT_WINDOW_WEEKS,
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    db,
    error::{ApiError, ApiResult},
    AppState,
};

/// Plate increment used when `/materialized` is called without one (OQ-1): a
/// kg/lb gym is a client concern, not a migration.
const DEFAULT_PLATE_KG: f64 = 2.5;

/// Longest accepted program name. Request-level, not a domain invariant.
const MAX_NAME_CHARS: usize = 120;

// ---------------------------------------------------------------------------
// DTOs (SPEC-0041 §2.7)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct CreateRequest {
    program: AuthoredProgram,
}

/// Fetch-one (AC5) and create (AC2).
///
/// No `name` field: it is already inside `program`, and carrying it twice in one
/// payload is two sources for one fact that will diverge the day update lands.
/// The denormalized column exists for the list query, not for this response.
#[derive(Debug, Serialize)]
pub(crate) struct ProgramResponse {
    id: Uuid,
    /// The full round-trip value (AC9) — the `AuthoredProgram` itself, not a
    /// re-modelled copy.
    program: AuthoredProgram,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// List item (AC4) — the projection the list query reads, no JSONB touched.
#[derive(Debug, Serialize, FromRow)]
pub(crate) struct ProgramListItem {
    id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// `?plate_kg=` on `/materialized`, defaulted server-side.
#[derive(Debug, Deserialize)]
pub(crate) struct MaterializeQuery {
    plate_kg: Option<f64>,
}

/// One stored row, program still serialized.
#[derive(Debug, FromRow)]
struct ProgramRow {
    id: Uuid,
    name: String,
    program: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ProgramRow {
    /// Deserialize the stored program. A row that will not parse back into the
    /// type that wrote it is data corruption, surfaced as a logged 500 — the
    /// `db::corrupt` discipline, with the denormalized `name` as the human
    /// handle on which row went bad.
    fn program(&self) -> ApiResult<AuthoredProgram> {
        serde_json::from_value(self.program.clone()).map_err(|e| {
            tracing::error!(
                id = %self.id,
                name = %self.name,
                error = %e,
                "stored authored program failed to deserialize — data corruption"
            );
            ApiError::Internal(eyre::eyre!("deserialise authored program"))
        })
    }
}

impl TryFrom<ProgramRow> for ProgramResponse {
    type Error = ApiError;

    fn try_from(row: ProgramRow) -> Result<Self, Self::Error> {
        let program = row.program()?;
        Ok(Self {
            id: row.id,
            program,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /authored-programs` — validate through `core::authoring`, then store
/// (AC2, AC3). Validation runs before any write, so an invalid program never
/// reaches the database.
pub(crate) async fn create(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateRequest>,
) -> ApiResult<(StatusCode, Json<ProgramResponse>)> {
    check_name(&body.program.name)?;
    validate(&body.program).map_err(|e| to_unprocessable(&e))?;

    let json = serde_json::to_value(&body.program)
        .map_err(|e| ApiError::Internal(eyre::eyre!("serialise authored program: {e}")))?;
    let row: ProgramRow = sqlx::query_as(
        "INSERT INTO authored_programs (user_id, name, program) VALUES ($1, $2, $3) \
         RETURNING id, name, program, created_at, updated_at",
    )
    .bind(user.user_id.0)
    // Store what was validated. `check_name` measures the *trimmed* name, so
    // binding the raw one would let `"x"` plus 100k spaces pass a 120-char cap
    // and then come back out of the list endpoint at full length.
    .bind(body.program.name.trim())
    .bind(json)
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(row.try_into()?)))
}

/// `GET /authored-programs` — the caller's own programs, newest first (AC4).
/// `id DESC` breaks a `created_at` tie so the order is total and stable.
pub(crate) async fn list(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<Vec<ProgramListItem>>> {
    let rows: Vec<ProgramListItem> = sqlx::query_as(
        "SELECT id, name, created_at, updated_at FROM authored_programs \
         WHERE user_id = $1 ORDER BY created_at DESC, id DESC",
    )
    .bind(user.user_id.0)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

/// `GET /authored-programs/:id` — one full program (AC5), 404 if it is not the
/// caller's (AC6).
pub(crate) async fn fetch_one(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ProgramResponse>> {
    let row = load_owned(&state.pool, id, user.user_id).await?;
    Ok(Json(row.try_into()?))
}

/// `GET /authored-programs/:id/materialized` — the concrete cycle against the
/// caller's own e1RM (AC7). A lift with no session in the last
/// `DEFAULT_WINDOW_WEEKS` is absent from the map, so its sets carry a `null`
/// load rather than a stale anchor.
pub(crate) async fn materialized(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Query(q): Query<MaterializeQuery>,
) -> ApiResult<Json<MaterializedCycle>> {
    let row = load_owned(&state.pool, id, user.user_id).await?;
    let program = row.program()?;
    let sessions = db::find_sessions_by_user(&state.pool, user.user_id).await?;
    let e1rm = current_e1rm(Utc::now().date_naive(), DEFAULT_WINDOW_WEEKS, &sessions);
    let cycle = materialize(&program, &e1rm, q.plate_kg.unwrap_or(DEFAULT_PLATE_KG))
        .map_err(|e| to_unprocessable(&e))?;
    Ok(Json(cycle))
}

// ---------------------------------------------------------------------------
// Shared pieces
// ---------------------------------------------------------------------------

/// The one ownership-scoped read. Returns the **row**, not the program, so
/// `fetch_one` can build its response from the same query rather than issuing a
/// second one — otherwise the single-enforcement-point claim is false in exactly
/// the place it matters. A row owned by someone else yields zero rows and
/// therefore the same `NotFound` as a genuinely missing id (AC6).
async fn load_owned(pool: &PgPool, id: Uuid, user_id: UserId) -> ApiResult<ProgramRow> {
    sqlx::query_as(
        "SELECT id, name, program, created_at, updated_at \
         FROM authored_programs WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id.0)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)
}

/// Program-name checks are *request-level* validation, not a domain invariant —
/// deliberately not a new `AuthorError` variant, which would push an API concern
/// into the domain model (SPEC-0041 §2.4).
fn check_name(name: &str) -> ApiResult<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_NAME_CHARS {
        return Err(ApiError::Validation { field: "name" });
    }
    Ok(())
}

/// Each `AuthorError` variant maps to one fixed token (never free text —
/// CLAUDE.md §6). Exhaustive with no `_` arm, so a twelfth variant becomes a
/// compile error rather than a silently wrong token.
///
/// The two variants carrying a `String` lose that detail: `reason` is
/// `&'static str`. The offending name is recoverable client-side — the client
/// holds the program it just sent.
fn to_unprocessable(e: &AuthorError) -> ApiError {
    let reason = match *e {
        AuthorError::NoExercises => "no_exercises",
        AuthorError::BlankExercise => "blank_exercise",
        AuthorError::DuplicateExercise(_) => "duplicate_exercise",
        AuthorError::NoScheduleDays => "no_schedule_days",
        AuthorError::NoScheduledEntries => "no_scheduled_entries",
        AuthorError::UnknownExercise(_) => "unknown_exercise",
        AuthorError::EmptyWorkLines => "empty_work_lines",
        AuthorError::BadIntensity => "bad_intensity",
        AuthorError::BadReps => "bad_reps",
        AuthorError::BadSets => "bad_sets",
        AuthorError::TooManySets => "too_many_sets",
        AuthorError::CycleTooLarge => "cycle_too_large",
        AuthorError::BadPlate => "bad_plate",
    };
    ApiError::Unprocessable { reason }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn reason_of(e: &AuthorError) -> &'static str {
        match to_unprocessable(e) {
            ApiError::Unprocessable { reason } => reason,
            other => panic!("expected Unprocessable, got {other:?}"),
        }
    }

    /// SPEC-0041 §7.16 — all eleven variants map to eleven *distinct* tokens.
    #[test]
    fn every_author_error_maps_to_a_distinct_token() {
        let all = [
            AuthorError::NoExercises,
            AuthorError::BlankExercise,
            AuthorError::DuplicateExercise("Squat".into()),
            AuthorError::NoScheduleDays,
            AuthorError::NoScheduledEntries,
            AuthorError::UnknownExercise("Ghost".into()),
            AuthorError::EmptyWorkLines,
            AuthorError::BadIntensity,
            AuthorError::BadReps,
            AuthorError::BadSets,
            AuthorError::TooManySets,
            AuthorError::CycleTooLarge,
            AuthorError::BadPlate,
        ];
        let tokens: BTreeSet<&'static str> = all.iter().map(reason_of).collect();
        assert_eq!(
            tokens.len(),
            all.len(),
            "every variant needs its own distinct token"
        );
        // Pinned deliberately: `reason` is a public wire contract clients switch
        // on, so growing the set should be a conscious act, not a silent one.
        assert_eq!(tokens.len(), 13, "token count changed — update clients too");
    }

    #[test]
    fn name_validation_is_request_level() {
        for bad in ["", "   ", &"x".repeat(MAX_NAME_CHARS + 1)] {
            assert!(
                matches!(check_name(bad), Err(ApiError::Validation { field: "name" })),
                "expected a name validation error for {bad:?}"
            );
        }
        assert!(check_name("Push / Pull / Legs").is_ok());
        assert!(check_name(&"x".repeat(MAX_NAME_CHARS)).is_ok());
    }
}
