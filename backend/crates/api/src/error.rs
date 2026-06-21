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

    /// A well-formed request the server understood but cannot act on — e.g. a
    /// photo session with no usable photo, or one whose pose cannot be derived
    /// (R-0013). `reason` is a fixed token, never free text (no stringly-typed
    /// errors, CLAUDE.md §6).
    #[error("unprocessable: {reason}")]
    Unprocessable { reason: &'static str },

    /// The request conflicts with the derived state of an existing resource —
    /// e.g. the chosen archetype is not among the session's top-3 proposals
    /// (R-0014, SPEC-0014 §2.4.6). `reason` is a fixed token.
    #[error("conflict: {reason}")]
    Conflict { reason: &'static str },

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

impl From<crate::pose::PoseError> for ApiError {
    /// `NoPersonDetected` is a 422 the user can act on (try another photo); a
    /// decode/inference fault is a server-side 500 — a stored, content-validated
    /// photo (R-0006) that won't decode, or a model fault, is not bad user input
    /// (SPEC-0013 §2.4).
    fn from(e: crate::pose::PoseError) -> Self {
        use crate::pose::PoseError;
        match e {
            PoseError::NoPersonDetected => ApiError::Unprocessable {
                reason: "no_person_detected",
            },
            PoseError::Decode | PoseError::Inference => {
                tracing::error!(error = %e, "pose inference fault");
                ApiError::Internal(eyre::eyre!("pose inference error"))
            }
        }
    }
}

impl From<fitai_core::FrameError> for ApiError {
    /// A pose that yields no derivable frame (too few confident joints, a
    /// degenerate hip span) is a 422 — no fabricated match (SPEC-0013 §2.4).
    fn from(_e: fitai_core::FrameError) -> Self {
        ApiError::Unprocessable {
            reason: "degenerate_frame",
        }
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
            ApiError::Unprocessable { reason } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({"error": "unprocessable", "reason": reason}),
            ),
            ApiError::Conflict { reason } => (StatusCode::CONFLICT, json!({"error": reason})),
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
