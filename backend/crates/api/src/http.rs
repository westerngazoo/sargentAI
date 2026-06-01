//! Small HTTP-extractor helpers shared across resource modules.
//!
//! Kept separate from `crate::error` so that module stays free of axum
//! extractor types (it only knows `IntoResponse`).

use axum::{extract::rejection::JsonRejection, Json};

use crate::error::{ApiError, ApiResult};

/// Map any serde/body rejection (missing field, bad type, malformed JSON,
/// unknown enum value) to a `400` with field `"body"` — without this, axum's
/// `Json` extractor rejects with its own response before the handler runs.
///
/// Handlers take `Result<Json<T>, JsonRejection>` and funnel it through here so
/// structural failures are reported uniformly as `"body"` while semantic
/// failures (caught later by the `core` write model) report their leaf field.
///
/// # Errors
/// [`ApiError::Validation`] with `field: "body"` when the body failed to
/// deserialize into `T`.
pub(crate) fn parse_body<T>(req: Result<Json<T>, JsonRejection>) -> ApiResult<T> {
    req.map(|Json(r)| r)
        .map_err(|_| ApiError::Validation { field: "body" })
}
