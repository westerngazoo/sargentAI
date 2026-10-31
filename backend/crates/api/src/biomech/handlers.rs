//! `POST /biomech/squat/range` — validate, run the engine, wrap the result.

use axum::{extract::rejection::JsonRejection, Json};
use fitai_core::biomech;

use super::{request::RangeRequest, response::RangeResponse};
use crate::{auth::AuthenticatedUser, error::ApiResult, http::parse_body};

/// Authenticated and stateless: stores nothing and logs nothing from the body
/// (AC2, AC8). Always `200` for a valid request; a range that stops early or
/// a lifter the engine cannot place is a result, not an error (AC3).
///
/// # Errors
/// `401` without a session; `400 validation` naming the field, or `"body"`
/// for a body that does not parse.
pub(crate) async fn squat_range(
    _user: AuthenticatedUser,
    req: Result<Json<RangeRequest>, JsonRejection>,
) -> ApiResult<Json<RangeResponse>> {
    let squat = parse_body(req)?.validate()?;
    let response = match biomech::squat_range(&squat.body, squat.bar, squat.ankle, squat.frames) {
        Ok(range) => RangeResponse::from_range(squat, range),
        Err(rejection) => RangeResponse::rejected(rejection, squat.table_lengths),
    };
    Ok(Json(response))
}
