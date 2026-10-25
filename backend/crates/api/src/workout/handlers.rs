//! HTTP handlers for the workout-log endpoints.
//!
//! Handlers are thin: validation is `core`'s job (via the `NewWorkoutSession`
//! write model, built bottom-up so the first error carries the right `field()`)
//! and persistence is `db`'s. The stored `core` aggregate serializes directly
//! to the wire (SPEC-0004 §2.4).
//!
//! A session may name the plan day it was seeded with (`user_program_id` +
//! `plan_day_index`, R-0061 / SPEC-0061 §2.4). The link is checked here, in
//! the spec's order, so an error never reveals another user's program.

use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    Json,
};
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use uuid::Uuid;

use fitai_core::archetype::find;
use fitai_core::{MuscleGroup, NewExercise, NewSet, NewWorkoutSession, WorkoutSession};

use crate::{
    auth::AuthenticatedUser,
    db::{self, PlanLink},
    error::{ApiError, ApiResult},
    http::parse_body,
    program::schedule::is_training_day,
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
    #[serde(default)]
    user_program_id: Option<Uuid>,
    #[serde(default)]
    plan_day_index: Option<u32>,
}

/// The plan link as requested, before any lookup: both fields or neither.
struct PlanRequest {
    user_program_id: Uuid,
    plan_day_index: u32,
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

    /// The plan fields come both or neither; one alone is a 422.
    fn plan_request(&self) -> ApiResult<Option<PlanRequest>> {
        match (self.user_program_id, self.plan_day_index) {
            (None, None) => Ok(None),
            (Some(user_program_id), Some(plan_day_index)) => Ok(Some(PlanRequest {
                user_program_id,
                plan_day_index,
            })),
            _ => Err(ApiError::Unprocessable {
                reason: "plan_fields_both_or_neither",
            }),
        }
    }
}

/// Check a requested plan link in SPEC-0061 §2.4's order and return what to
/// store: a foreign program is a 404 before anything about it is read; the
/// caller's own superseded program a 409; a program whose archetype has no
/// cycle a 422 `plan_unavailable`; then the index must name a training day
/// and `performed_on` must not precede `starts_on`.
async fn resolve_plan_link(
    state: &AppState,
    user: &AuthenticatedUser,
    req: PlanRequest,
    performed_on: NaiveDate,
) -> ApiResult<PlanLink> {
    let program = db::get_program_by_id(&state.pool, user.user_id, req.user_program_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if !program.active {
        return Err(ApiError::Conflict {
            reason: "program_not_current",
        });
    }
    let archetype = find(&program.archetype_id).ok_or(ApiError::Unprocessable {
        reason: "plan_unavailable",
    })?;
    if !is_training_day(&archetype.program_template.cycle, req.plan_day_index) {
        return Err(ApiError::Unprocessable {
            reason: "plan_day_not_training",
        });
    }
    if performed_on < program.starts_on {
        return Err(ApiError::Unprocessable {
            reason: "performed_on_before_starts_on",
        });
    }
    Ok(PlanLink {
        user_program_id: program.id,
        // A training day's index is below the 14-day cycle bound (§2.1).
        plan_day_index: i16::try_from(req.plan_day_index)?,
    })
}

pub(crate) async fn create(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    req: Result<Json<SessionRequest>, JsonRejection>,
) -> ApiResult<(StatusCode, Json<WorkoutSession>)> {
    let req = parse_body(req)?;
    let plan_request = req.plan_request()?;
    let new = req.into_new(Utc::now().date_naive())?;
    let plan = match plan_request {
        Some(p) => Some(resolve_plan_link(&state, &user, p, new.performed_on).await?),
        None => None,
    };
    let session = db::insert_session(&state.pool, user.user_id, &new, plan).await?;
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
    // The body carries no plan fields, so the stored link is kept as it is;
    // only its date rule is checked again (SPEC-0061 §2.4).
    let starts_on = db::session_plan_starts_on(&state.pool, user.user_id, id).await?;
    if starts_on.is_some_and(|starts_on| new.performed_on < starts_on) {
        return Err(ApiError::Unprocessable {
            reason: "performed_on_before_starts_on",
        });
    }
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
