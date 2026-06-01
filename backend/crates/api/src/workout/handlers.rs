//! HTTP handlers for the workout-log endpoints.
//!
//! Handlers are thin: validation is `core`'s job (via the `NewWorkoutSession`
//! write model, built bottom-up so the first error carries the right `field()`)
//! and persistence is `db`'s. The stored `core` aggregate serializes directly
//! to the wire (SPEC-0004 §2.4).

use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    Json,
};
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use uuid::Uuid;

use fitai_core::{MuscleGroup, NewExercise, NewSet, NewWorkoutSession, WorkoutSession};

use crate::{
    auth::AuthenticatedUser,
    db,
    error::{ApiError, ApiResult},
    http::parse_body,
    AppState,
};

#[derive(Debug, Deserialize)]
pub(crate) struct SetRequest {
    reps: i32,
    #[serde(default)]
    weight_kg: Option<f64>,
    #[serde(default)]
    rpe: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ExerciseRequest {
    name: String,
    #[serde(default)]
    muscle_group: Option<MuscleGroup>,
    sets: Vec<SetRequest>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SessionRequest {
    performed_on: NaiveDate,
    exercises: Vec<ExerciseRequest>,
}

impl SessionRequest {
    /// Build the validated write model, innermost first, so the first
    /// validation error surfaces with the correct `field()`.
    fn into_new(self, today: NaiveDate) -> ApiResult<NewWorkoutSession> {
        let validation = |e: fitai_core::WorkoutError| ApiError::Validation { field: e.field() };

        let mut exercises = Vec::with_capacity(self.exercises.len());
        for ex in self.exercises {
            let mut sets = Vec::with_capacity(ex.sets.len());
            for s in ex.sets {
                sets.push(NewSet::new(s.reps, s.weight_kg, s.rpe).map_err(validation)?);
            }
            exercises.push(NewExercise::new(&ex.name, ex.muscle_group, sets).map_err(validation)?);
        }
        NewWorkoutSession::new(self.performed_on, exercises, today).map_err(validation)
    }
}

pub(crate) async fn create(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    req: Result<Json<SessionRequest>, JsonRejection>,
) -> ApiResult<(StatusCode, Json<WorkoutSession>)> {
    let new = parse_body(req)?.into_new(Utc::now().date_naive())?;
    let session = db::insert_session(&state.pool, user.user_id, &new).await?;
    Ok((StatusCode::CREATED, Json(session)))
}

pub(crate) async fn list(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<Vec<WorkoutSession>>> {
    let sessions = db::find_sessions_by_user(&state.pool, user.user_id).await?;
    Ok(Json(sessions))
}

pub(crate) async fn get_one(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<WorkoutSession>> {
    db::find_session_by_id(&state.pool, user.user_id, id)
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

pub(crate) async fn replace(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    req: Result<Json<SessionRequest>, JsonRejection>,
) -> ApiResult<Json<WorkoutSession>> {
    let new = parse_body(req)?.into_new(Utc::now().date_naive())?;
    db::replace_session(&state.pool, user.user_id, id, &new)
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

pub(crate) async fn delete(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    if db::delete_session(&state.pool, user.user_id, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}
