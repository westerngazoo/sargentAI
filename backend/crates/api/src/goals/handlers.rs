//! Goal handlers — inline sqlx, one ownership-scoped read.
//!
//! Extractor order is an invariant, not a style choice: axum runs extractors
//! left to right, so `AuthenticatedUser` must precede `Path`/`Json` in every
//! signature — a malformed id with no token must be 401, never axum's 400.

use std::collections::BTreeMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use fitai_core::{
    goals::{assess, validate, Baseline, Goal, GoalError, GoalKind, GoalSignal, PaceReport},
    summarize, BodyPoint, TrainingSummary, WorkoutSession, DEFAULT_WINDOW_WEEKS,
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

// ---------------------------------------------------------------------------
// DTOs (SPEC-0042 §2.6, §3)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct CreateGoalRequest {
    kind: GoalKind,
    target_date: NaiveDate,
}

/// Create (AC1) and fetch-one. The domain `Goal` round-trips whole — no
/// re-modelled copy, no duplicated fields (the R-0041 AC9 pattern).
#[derive(Debug, Serialize)]
pub(crate) struct GoalResponse {
    id: Uuid,
    goal: Goal,
    report: PaceReport,
    created_at: DateTime<Utc>,
}

/// One stored row, goal still serialized.
#[derive(Debug, FromRow)]
struct GoalRow {
    id: Uuid,
    goal: serde_json::Value,
    created_at: DateTime<Utc>,
}

impl GoalRow {
    /// Deserialize the stored goal. A row that will not parse back into the
    /// type that wrote it is data corruption, surfaced as a logged 500.
    fn goal(&self) -> ApiResult<Goal> {
        serde_json::from_value(self.goal.clone()).map_err(|e| {
            tracing::error!(
                id = %self.id,
                error = %e,
                "stored goal failed to deserialize — data corruption"
            );
            ApiError::Internal(eyre::eyre!("deserialise goal"))
        })
    }
}

// ---------------------------------------------------------------------------
// Signal assembly (SPEC-0042 §2.6)
// ---------------------------------------------------------------------------

/// The raw observations a pace report is computed from, fetched once per
/// request. Summaries are then derived per anchor date (§2.2.4), so `GET
/// /goals` never re-queries per goal.
struct SignalInputs {
    sessions: Vec<WorkoutSession>,
    measurements: Vec<BodyPoint>,
}

impl SignalInputs {
    /// Aggregate at `anchor` — `min(today, target_date)` per goal, so an
    /// expired goal's terminal verdict is a stable function of the history
    /// that existed at its deadline, not of whatever the window holds today.
    ///
    /// The adherence target is irrelevant here (a Consistency goal compares
    /// against its own rate), so it is passed as zero.
    fn summary_at(&self, anchor: NaiveDate) -> TrainingSummary {
        summarize(
            anchor,
            DEFAULT_WINDOW_WEEKS,
            &self.sessions,
            &self.measurements,
            0,
        )
    }
}

#[derive(FromRow)]
struct BodyRow {
    measured_on: NaiveDate,
    weight_kg: f64,
    body_fat_percentage: Option<f64>,
}

async fn fetch_signal_inputs(pool: &PgPool, user: &AuthenticatedUser) -> ApiResult<SignalInputs> {
    let sessions = db::find_sessions_by_user(pool, user.user_id).await?;
    let rows: Vec<BodyRow> = sqlx::query_as(
        "SELECT measured_on, weight_kg, body_fat_percentage \
         FROM body_measurements WHERE user_id = $1 ORDER BY measured_on ASC",
    )
    .bind(user.user_id.0)
    .fetch_all(pool)
    .await?;
    let measurements = rows
        .into_iter()
        .map(|r| BodyPoint {
            on: r.measured_on,
            weight_kg: r.weight_kg,
            body_fat_pct: r.body_fat_percentage,
        })
        .collect();
    Ok(SignalInputs {
        sessions,
        measurements,
    })
}

/// Anchor for a goal's signal: expired goals are assessed against the window
/// that existed at their deadline (§2.2.4).
fn anchor_for(goal: &Goal, today: NaiveDate) -> NaiveDate {
    today.min(goal.target_date)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /goals` — validate, capture the baseline server-side from logged data
/// (AC1 — never from the request), store, and return the initial report.
pub(crate) async fn create(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateGoalRequest>,
) -> ApiResult<(StatusCode, Json<GoalResponse>)> {
    let today = Utc::now().date_naive();
    validate(&body.kind, today, body.target_date).map_err(|e| to_unprocessable(&e))?;

    let inputs = fetch_signal_inputs(&state.pool, &user).await?;
    let summary = inputs.summary_at(today);
    let baseline = Baseline::capture(&body.kind, &summary);
    let goal = Goal {
        kind: body.kind,
        baseline,
        set_on: today,
        target_date: body.target_date,
    };

    let json = serde_json::to_value(&goal)
        .map_err(|e| ApiError::Internal(eyre::eyre!("serialise goal: {e}")))?;
    let row: GoalRow = sqlx::query_as(
        "INSERT INTO goals (user_id, goal) VALUES ($1, $2) \
         RETURNING id, goal, created_at",
    )
    .bind(user.user_id.0)
    .bind(json)
    .fetch_one(&state.pool)
    .await?;

    let report = assess(&goal, &GoalSignal::from(&summary), today);
    Ok((
        StatusCode::CREATED,
        Json(GoalResponse {
            id: row.id,
            goal,
            report,
            created_at: row.created_at,
        }),
    ))
}

/// `GET /goals` — the caller's goals, newest first, each with its current
/// report. Inputs are fetched once and summarized once per distinct anchor
/// date (§2.6): one aggregation for all active goals, plus one per distinct
/// expired deadline.
pub(crate) async fn list(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<Vec<GoalResponse>>> {
    let rows: Vec<GoalRow> = sqlx::query_as(
        "SELECT id, goal, created_at FROM goals \
         WHERE user_id = $1 ORDER BY created_at DESC, id DESC",
    )
    .bind(user.user_id.0)
    .fetch_all(&state.pool)
    .await?;

    let today = Utc::now().date_naive();
    let inputs = fetch_signal_inputs(&state.pool, &user).await?;
    let mut summaries: BTreeMap<NaiveDate, TrainingSummary> = BTreeMap::new();

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let goal = row.goal()?;
        let anchor = anchor_for(&goal, today);
        let summary = summaries
            .entry(anchor)
            .or_insert_with(|| inputs.summary_at(anchor));
        let report = assess(&goal, &GoalSignal::from(&*summary), today);
        out.push(GoalResponse {
            id: row.id,
            goal,
            report,
            created_at: row.created_at,
        });
    }
    Ok(Json(out))
}

/// `GET /goals/:id` — one goal + report, 404 if not the caller's (AC3).
pub(crate) async fn fetch_one(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<GoalResponse>> {
    let row = load_owned(&state.pool, id, &user).await?;
    let goal = row.goal()?;
    let today = Utc::now().date_naive();
    let inputs = fetch_signal_inputs(&state.pool, &user).await?;
    let summary = inputs.summary_at(anchor_for(&goal, today));
    let report = assess(&goal, &GoalSignal::from(&summary), today);
    Ok(Json(GoalResponse {
        id: row.id,
        goal,
        report,
        created_at: row.created_at,
    }))
}

/// `DELETE /goals/:id` — abandon a goal (AC11). Ownership is enforced by the
/// same `WHERE user_id` predicate as the reads: a foreign id deletes zero rows
/// and 404s, never 403.
pub(crate) async fn delete(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let result = sqlx::query("DELETE FROM goals WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.user_id.0)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Shared pieces
// ---------------------------------------------------------------------------

/// The one ownership-scoped read. Missing and foreign ids take the identical
/// query and the identical `NotFound`, so ids are not enumerable (AC3).
async fn load_owned(pool: &PgPool, id: Uuid, user: &AuthenticatedUser) -> ApiResult<GoalRow> {
    sqlx::query_as("SELECT id, goal, created_at FROM goals WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.user_id.0)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)
}

/// Each `GoalError` variant maps to one fixed token (never free text —
/// CLAUDE.md §6). Exhaustive with no `_` arm, so a twelfth variant becomes a
/// compile error rather than a silently wrong token.
fn to_unprocessable(e: &GoalError) -> ApiError {
    let reason = match e {
        GoalError::TargetDateNotAfterCreation => "target_date_not_after_creation",
        GoalError::TargetDateTooFar => "target_date_too_far",
        GoalError::NonFiniteTarget => "non_finite_target",
        GoalError::NonPositiveTarget => "non_positive_target",
        GoalError::TargetTooLarge => "target_too_large",
        GoalError::BodyFatOutOfRange => "body_fat_out_of_range",
        GoalError::BodyTargetEmpty => "body_target_empty",
        GoalError::BlankLift => "blank_lift",
        GoalError::LiftTooLong => "lift_too_long",
        GoalError::ZeroSessions => "zero_sessions",
        GoalError::TooManySessions => "too_many_sessions",
    };
    ApiError::Unprocessable { reason }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn reason_of(e: &GoalError) -> &'static str {
        match to_unprocessable(e) {
            ApiError::Unprocessable { reason } => reason,
            other => panic!("expected Unprocessable, got {other:?}"),
        }
    }

    #[test]
    fn every_goal_error_maps_to_a_distinct_token() {
        let all = [
            GoalError::TargetDateNotAfterCreation,
            GoalError::TargetDateTooFar,
            GoalError::NonFiniteTarget,
            GoalError::NonPositiveTarget,
            GoalError::TargetTooLarge,
            GoalError::BodyFatOutOfRange,
            GoalError::BodyTargetEmpty,
            GoalError::BlankLift,
            GoalError::LiftTooLong,
            GoalError::ZeroSessions,
            GoalError::TooManySessions,
        ];
        let tokens: BTreeSet<&'static str> = all.iter().map(reason_of).collect();
        assert_eq!(
            tokens.len(),
            all.len(),
            "every variant needs its own distinct token"
        );
        // Pinned deliberately: `reason` is a public wire contract clients
        // switch on, so growing the set should be a conscious act.
        assert_eq!(tokens.len(), 11, "token count changed — update clients too");
    }
}
