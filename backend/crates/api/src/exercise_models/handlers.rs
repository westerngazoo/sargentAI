//! The three handlers: find the model, describe it, or validate and run it.

use axum::{extract::rejection::JsonRejection, extract::Path, Json};
use fitai_core::{
    biomech::Joint,
    exercise::{find, library, Description, Equipment, ExerciseModel, ExerciseRange},
};
use serde::Serialize;

use super::request::RangeRequest;
use crate::{
    auth::AuthenticatedUser,
    error::{ApiError, ApiResult},
    http::parse_body,
};

/// One row of the list: enough to pick an exercise (R-0066 AC9). Both names
/// travel, so the app picks by its language.
#[derive(Serialize)]
pub(crate) struct Summary {
    id: &'static str,
    name_es: &'static str,
    name_en: &'static str,
    equipment: Equipment,
    joints: &'static [Joint],
}

impl From<Description> for Summary {
    fn from(d: Description) -> Self {
        Self {
            id: d.id,
            name_es: d.name_es,
            name_en: d.name_en,
            equipment: d.equipment,
            joints: d.joints,
        }
    }
}

/// Every modelled exercise, in the library's order.
///
/// # Errors
/// `401` without a session (the extractor's).
pub(crate) async fn list(_user: AuthenticatedUser) -> Json<Vec<Summary>> {
    Json(library().iter().map(|m| m.describe().into()).collect())
}

/// One exercise's description: its body measures and controls (R-0066 AC9).
///
/// # Errors
/// `401` without a session; `404` for an id the library does not have.
pub(crate) async fn describe(
    _user: AuthenticatedUser,
    Path(id): Path<String>,
) -> ApiResult<Json<Description>> {
    Ok(Json(model(&id)?.describe()))
}

/// One exercise's range (R-0066 AC8). A range that stops early or cannot
/// start is a result: `200`, as SPEC-0059 AC3.
///
/// # Errors
/// `401` without a session; `404` for an unknown id, before the body is
/// validated (a body that does not parse still gets the `404`);
/// `400 validation` naming the field, or `"body"` for a body that does not
/// parse; `500` when the kernel's cross-check fails or the model refuses
/// settings that passed validation (SPEC-0066 §2.4).
pub(crate) async fn range(
    _user: AuthenticatedUser,
    Path(id): Path<String>,
    req: Result<Json<RangeRequest>, JsonRejection>,
) -> ApiResult<Json<ExerciseRange>> {
    let model = model(&id)?;
    let input = parse_body(req)?.validate(&model.describe())?;
    let range = model.range(&input.body, &input.measures, &input.settings, input.frames)?;
    Ok(Json(range))
}

fn model(id: &str) -> ApiResult<&'static dyn ExerciseModel> {
    find(id).ok_or(ApiError::NotFound)
}
