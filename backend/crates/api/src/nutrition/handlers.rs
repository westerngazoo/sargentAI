//! HTTP handlers for the nutrition-log endpoints.
//!
//! Handlers are thin: validation is `core`'s job (via the `NewNutritionLog`
//! write model) and persistence is `db`'s. Because the wire shape carries a
//! derived `calories` the `core` aggregate does not store, the `NutritionResponse`
//! DTO owns serialization (the R-0003 `ProfileResponse`/`age` precedent,
//! SPEC-0005 §2.4).

use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use fitai_core::{NewNutritionLog, NutritionLog, UserId};

use crate::{
    auth::AuthenticatedUser,
    db,
    error::{ApiError, ApiResult},
    http::parse_body,
    AppState,
};

#[derive(Debug, Deserialize)]
pub(crate) struct NutritionRequest {
    performed_on: NaiveDate,
    protein_g: f64,
    carbs_g: f64,
    fat_g: f64,
}

impl NutritionRequest {
    fn into_new(self, today: NaiveDate) -> ApiResult<NewNutritionLog> {
        NewNutritionLog::new(
            self.performed_on,
            self.protein_g,
            self.carbs_g,
            self.fat_g,
            today,
        )
        .map_err(|e| ApiError::Validation { field: e.field() })
    }
}

/// Wire shape (AC7). Adds the derived `calories`; the `core` aggregate stores
/// only the macros it is computed from.
#[derive(Debug, Serialize)]
pub(crate) struct NutritionResponse {
    id: Uuid,
    user_id: UserId,
    performed_on: NaiveDate,
    protein_g: f64,
    carbs_g: f64,
    fat_g: f64,
    calories: f64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl NutritionResponse {
    fn from_log(log: &NutritionLog) -> Self {
        Self {
            id: log.id,
            user_id: log.user_id,
            performed_on: log.performed_on,
            protein_g: log.macros.protein.get(),
            carbs_g: log.macros.carbs.get(),
            fat_g: log.macros.fat.get(),
            calories: log.calories(),
            created_at: log.created_at,
            updated_at: log.updated_at,
        }
    }
}

pub(crate) async fn create(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    req: Result<Json<NutritionRequest>, JsonRejection>,
) -> ApiResult<(StatusCode, Json<NutritionResponse>)> {
    let new = parse_body(req)?.into_new(Utc::now().date_naive())?;
    let log = db::insert_nutrition_log(&state.pool, user.user_id, &new).await?;
    Ok((StatusCode::CREATED, Json(NutritionResponse::from_log(&log))))
}

pub(crate) async fn list(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<Vec<NutritionResponse>>> {
    let logs = db::find_nutrition_logs_by_user(&state.pool, user.user_id).await?;
    Ok(Json(logs.iter().map(NutritionResponse::from_log).collect()))
}

pub(crate) async fn get_one(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<NutritionResponse>> {
    db::find_nutrition_log_by_id(&state.pool, user.user_id, id)
        .await?
        .map(|log| Json(NutritionResponse::from_log(&log)))
        .ok_or(ApiError::NotFound)
}

pub(crate) async fn replace(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    req: Result<Json<NutritionRequest>, JsonRejection>,
) -> ApiResult<Json<NutritionResponse>> {
    let new = parse_body(req)?.into_new(Utc::now().date_naive())?;
    db::update_nutrition_log(&state.pool, user.user_id, id, &new)
        .await?
        .map(|log| Json(NutritionResponse::from_log(&log)))
        .ok_or(ApiError::NotFound)
}

pub(crate) async fn delete(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    if db::delete_nutrition_log(&state.pool, user.user_id, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}
