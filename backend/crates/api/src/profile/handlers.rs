//! HTTP handlers for the profile endpoints.

use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use fitai_core::{BodyFatPercentage, Goal, NewProfile, Profile, Sex, UserId};

use crate::{
    auth::AuthenticatedUser,
    db,
    error::{ApiError, ApiResult},
    AppState,
};

#[derive(Debug, Deserialize)]
pub(crate) struct ProfileRequest {
    date_of_birth: NaiveDate,
    height_cm: i32,
    weight_kg: f64,
    goals: Vec<Goal>,
    #[serde(default)]
    sex: Option<Sex>,
    #[serde(default)]
    body_fat_percentage: Option<f64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ProfileResponse {
    user_id: UserId,
    date_of_birth: NaiveDate,
    age: i32,
    height_cm: i32,
    weight_kg: f64,
    sex: Option<Sex>,
    body_fat_percentage: Option<f64>,
    goals: Vec<Goal>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ProfileResponse {
    fn from_profile(p: &Profile, today: NaiveDate) -> Self {
        Self {
            age: p.age_on(today),
            user_id: p.user_id,
            date_of_birth: p.date_of_birth,
            height_cm: p.height_cm.get(),
            weight_kg: p.weight_kg.get(),
            sex: p.sex,
            body_fat_percentage: p.body_fat_percentage.map(BodyFatPercentage::get),
            goals: p.goals.as_slice().to_vec(),
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

/// Map any serde/body rejection (missing field, bad type, malformed JSON) to a
/// 400 — without this, axum's `Json` extractor rejects before the handler runs.
fn parse_body(req: Result<Json<ProfileRequest>, JsonRejection>) -> ApiResult<ProfileRequest> {
    req.map(|Json(r)| r)
        .map_err(|_| ApiError::Validation { field: "body" })
}

pub(crate) async fn get_me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<ProfileResponse>> {
    let profile = db::find_profile_by_user(&state.pool, user.user_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let today = Utc::now().date_naive();
    Ok(Json(ProfileResponse::from_profile(&profile, today)))
}

pub(crate) async fn put_me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    req: Result<Json<ProfileRequest>, JsonRejection>,
) -> ApiResult<(StatusCode, Json<ProfileResponse>)> {
    let req = parse_body(req)?;
    let today = Utc::now().date_naive();
    let new = NewProfile::new(
        req.date_of_birth,
        req.height_cm,
        req.weight_kg,
        req.goals,
        req.sex,
        req.body_fat_percentage,
        today,
    )
    .map_err(|e| ApiError::Validation { field: e.field() })?;

    let (profile, inserted) = db::upsert_profile(&state.pool, user.user_id, &new).await?;
    let status = if inserted {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(ProfileResponse::from_profile(&profile, today))))
}
