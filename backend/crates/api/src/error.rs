//! Typed application error → HTTP response.
//!
//! Every variant maps to exactly one status code and one stable error body
//! shape. `Unauthorized` is enumeration-safe: identical body across all
//! auth-failure causes (missing header, malformed token, expired, bad sig,
//! unknown sub).

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("database error")]
    Database(#[from] sqlx::Error),

    #[error("already exists")]
    AlreadyExists,

    #[error("validation error in field `{field}`")]
    Validation { field: &'static str },

    #[error("unauthorized")]
    Unauthorized,

    #[error("not found")]
    NotFound,

    #[error("internal error")]
    Internal(#[from] eyre::Report),

    #[error("integer conversion error")]
    IntConversion(#[from] std::num::TryFromIntError),
}

impl From<crate::storage::ObjectStoreError> for ApiError {
    /// Object-store failures (a missing object, an IO error, a rejected key) are
    /// server-side faults — they map to the opaque `Internal` (500) body, never
    /// a variant that leaks storage internals (SPEC-0006 §2.4, AC10).
    fn from(e: crate::storage::ObjectStoreError) -> Self {
        tracing::error!(error = %e, "object store error");
        ApiError::Internal(eyre::eyre!("object store error"))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            ApiError::Database(e) => {
                // Postgres unique-violation surfaces here when callers
                // didn't pre-check; map it to AlreadyExists.
                if let sqlx::Error::Database(db_err) = e {
                    if db_err.is_unique_violation() {
                        return ApiError::AlreadyExists.into_response();
                    }
                }
                tracing::error!(error = %e, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({"error": "internal"}),
                )
            }
            ApiError::AlreadyExists => (StatusCode::CONFLICT, json!({"error": "already_exists"})),
            ApiError::Validation { field } => (
                StatusCode::BAD_REQUEST,
                json!({"error": "validation", "field": field}),
            ),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, json!({"error": "unauthorized"})),
            ApiError::NotFound => (StatusCode::NOT_FOUND, json!({"error": "not_found"})),
            ApiError::Internal(e) => {
                tracing::error!(error = %e, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({"error": "internal"}),
                )
            }
            ApiError::IntConversion(e) => {
                tracing::error!(error = %e, "integer conversion error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({"error": "internal"}),
                )
            }
        };
        (status, Json(body)).into_response()
    }
}
