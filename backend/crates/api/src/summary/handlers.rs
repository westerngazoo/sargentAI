//! Handlers for `GET /training-summary` (R-0015 AC8) and `GET /adjustments`
//! (R-0017 AC7). Thin edge: fetch rows, map to core inputs, call the pure
//! functions, serialize. The window is fixed at the R-0015 default for v1.

use axum::{extract::State, Json};
use chrono::{NaiveDate, Utc};
use fitai_core::{suggest, summarize, Adjustment, BodyPoint, TrainingSummary};
use serde::Serialize;
use sqlx::PgPool;

use fitai_core::{GeneratedDiet, GeneratedProgram};

use crate::{
    auth::AuthenticatedUser,
    db::{self, UserProgramRow},
    error::{ApiError, ApiResult},
    AppState,
};

/// Default aggregation window (SPEC-0015 OQ-1).
const WINDOW_WEEKS: u32 = 8;

/// `GET /training-summary` — the caller's aggregated facts (R-0015).
pub(crate) async fn training_summary(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<TrainingSummary>> {
    let inputs = fetch_inputs(&state.pool, user).await?;
    Ok(Json(inputs.summarize(Utc::now().date_naive())))
}

/// Response for `GET /adjustments` (R-0017 AC7).
#[derive(Serialize)]
pub(crate) struct AdjustmentsResponse {
    window_weeks: u32,
    suggestions: Vec<Adjustment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
}

/// `GET /adjustments` — coach suggestions from the heuristic engine.
pub(crate) async fn adjustments(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<AdjustmentsResponse>> {
    let inputs = fetch_inputs(&state.pool, user).await?;
    let Some(parts) = &inputs.program else {
        return Ok(Json(AdjustmentsResponse {
            window_weeks: WINDOW_WEEKS,
            suggestions: Vec::new(),
            reason: Some("no_active_program"),
        }));
    };
    let summary = inputs.summarize(Utc::now().date_naive());
    let suggestions = suggest(&summary, &parts.program, &parts.diet);
    Ok(Json(AdjustmentsResponse {
        window_weeks: WINDOW_WEEKS,
        suggestions,
        reason: None,
    }))
}

/// The active program's deserialized halves — what the engine reads.
struct CurrentProgramParts {
    program: GeneratedProgram,
    diet: GeneratedDiet,
}

impl TryFrom<UserProgramRow> for CurrentProgramParts {
    type Error = ApiError;

    fn try_from(row: UserProgramRow) -> Result<Self, Self::Error> {
        let program: GeneratedProgram = serde_json::from_value(row.program)
            .map_err(|e| ApiError::Internal(eyre::eyre!("deserialise program: {e}")))?;
        let diet: GeneratedDiet = serde_json::from_value(row.diet)
            .map_err(|e| ApiError::Internal(eyre::eyre!("deserialise diet: {e}")))?;
        Ok(Self { program, diet })
    }
}

/// Everything both endpoints need, fetched once.
struct SummaryInputs {
    sessions: Vec<fitai_core::WorkoutSession>,
    measurements: Vec<BodyPoint>,
    program: Option<CurrentProgramParts>,
}

impl SummaryInputs {
    fn summarize(&self, today: NaiveDate) -> TrainingSummary {
        let target = self
            .program
            .as_ref()
            .map_or(0, |p| u32::from(p.program.days_per_week));
        summarize(
            today,
            WINDOW_WEEKS,
            &self.sessions,
            &self.measurements,
            target,
        )
    }
}

#[derive(sqlx::FromRow)]
struct BodyRow {
    measured_on: NaiveDate,
    weight_kg: f64,
    body_fat_percentage: Option<f64>,
}

async fn fetch_inputs(pool: &PgPool, user: AuthenticatedUser) -> ApiResult<SummaryInputs> {
    let sessions = db::find_sessions_by_user(pool, user.user_id).await?;

    let rows: Vec<BodyRow> = sqlx::query_as(
        "SELECT measured_on, weight_kg, body_fat_percentage \
         FROM body_measurements WHERE user_id = $1 ORDER BY measured_on ASC",
    )
    .bind(user.user_id.0)
    .fetch_all(pool)
    .await
    .map_err(ApiError::Database)?;
    let measurements = rows
        .into_iter()
        .map(|r| BodyPoint {
            on: r.measured_on,
            weight_kg: r.weight_kg,
            body_fat_pct: r.body_fat_percentage,
        })
        .collect();

    let program = match db::get_current_program(pool, user.user_id).await? {
        Some(row) => Some(CurrentProgramParts::try_from(row)?),
        None => None,
    };

    Ok(SummaryInputs {
        sessions,
        measurements,
        program,
    })
}
