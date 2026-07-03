//! Body-measurement handlers — inline sqlx, self-contained validation.

use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    error::{ApiError, ApiResult},
    http::parse_body,
    AppState,
};

#[derive(Debug, Deserialize)]
pub(crate) struct MeasurementRequest {
    measured_on: NaiveDate,
    weight_kg: f64,
    body_fat_percentage: Option<f64>,
}

impl MeasurementRequest {
    fn validate(&self) -> ApiResult<()> {
        if !(20.0..=400.0).contains(&self.weight_kg) {
            return Err(ApiError::Validation { field: "weight_kg" });
        }
        if let Some(bf) = self.body_fat_percentage {
            if !(2.0..=75.0).contains(&bf) {
                return Err(ApiError::Validation {
                    field: "body_fat_percentage",
                });
            }
        }
        Ok(())
    }
}

/// Wire shape. `lean_mass_kg` is derived (weight × (1 − bf%)) when bf% is
/// present, so charts get muscle-up as a first-class series.
#[derive(Debug, Serialize, FromRow)]
pub(crate) struct MeasurementRow {
    id: Uuid,
    user_id: Uuid,
    measured_on: NaiveDate,
    weight_kg: f64,
    body_fat_percentage: Option<f64>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(crate) struct MeasurementResponse {
    id: Uuid,
    user_id: Uuid,
    measured_on: NaiveDate,
    weight_kg: f64,
    body_fat_percentage: Option<f64>,
    lean_mass_kg: Option<f64>,
    created_at: DateTime<Utc>,
}

impl From<MeasurementRow> for MeasurementResponse {
    fn from(r: MeasurementRow) -> Self {
        let lean = r
            .body_fat_percentage
            .map(|bf| r.weight_kg * (1.0 - bf / 100.0));
        Self {
            id: r.id,
            user_id: r.user_id,
            measured_on: r.measured_on,
            weight_kg: r.weight_kg,
            body_fat_percentage: r.body_fat_percentage,
            lean_mass_kg: lean,
            created_at: r.created_at,
        }
    }
}

/// `POST /measurements` — upsert one row per day (re-weighing overwrites).
pub(crate) async fn create(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    req: Result<Json<MeasurementRequest>, JsonRejection>,
) -> ApiResult<(StatusCode, Json<MeasurementResponse>)> {
    let req = parse_body(req)?;
    req.validate()?;
    let row = sqlx::query_as::<_, MeasurementRow>(
        "INSERT INTO body_measurements \
           (id, user_id, measured_on, weight_kg, body_fat_percentage) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (user_id, measured_on) DO UPDATE SET \
           weight_kg = EXCLUDED.weight_kg, \
           body_fat_percentage = EXCLUDED.body_fat_percentage \
         RETURNING id, user_id, measured_on, weight_kg, \
           body_fat_percentage, created_at",
    )
    .bind(Uuid::new_v4())
    .bind(user.user_id.0)
    .bind(req.measured_on)
    .bind(req.weight_kg)
    .bind(req.body_fat_percentage)
    .fetch_one(&state.pool)
    .await
    .map_err(ApiError::Database)?;
    Ok((StatusCode::CREATED, Json(row.into())))
}

/// `GET /measurements` — the caller's measurements, oldest first (for charts).
pub(crate) async fn list(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<Vec<MeasurementResponse>>> {
    let rows = sqlx::query_as::<_, MeasurementRow>(
        "SELECT id, user_id, measured_on, weight_kg, \
           body_fat_percentage, created_at \
         FROM body_measurements WHERE user_id = $1 ORDER BY measured_on ASC",
    )
    .bind(user.user_id.0)
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}
