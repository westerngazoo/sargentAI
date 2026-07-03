//! HTTP handlers for the auth endpoints.

use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use fitai_core::{Email, UserId};

use crate::{
    auth::{
        google::{GoogleClaims, GoogleLoginRequest},
        password, token, AuthenticatedUser,
    },
    db,
    error::{ApiError, ApiResult},
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub(crate) struct AuthRequest {
    // Email format is *not* validated here: `core::Email::parse` is the single
    // normalization+validation authority (it trims and lowercases), so gating
    // the raw string with `#[validate(email)]` would reject a padded/mixed-case
    // address before it could be normalized — breaking case-insensitive
    // duplicate detection (SAC2).
    email: String,
    #[validate(length(min = 8))]
    password: String,
}

/// Extract the JSON body, mapping any serde rejection (missing/empty field,
/// malformed JSON, wrong content-type) to a 400 `Validation` error. Without
/// this, a body that omits `password` would be rejected by axum's own `Json`
/// extractor before the handler runs, yielding the wrong status (SAC2).
fn body(req: Result<Json<AuthRequest>, JsonRejection>) -> ApiResult<AuthRequest> {
    let Json(req) = req.map_err(|_| ApiError::Validation { field: "body" })?;
    req.validate()
        .map_err(|_| ApiError::Validation { field: "password" })?;
    Ok(req)
}

#[derive(Debug, Serialize)]
pub(crate) struct RegisterResponse {
    user_id: UserId,
}

#[derive(Debug, Serialize)]
pub(crate) struct LoginResponse {
    token: String,
    user_id: UserId,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(crate) struct MeResponse {
    user_id: UserId,
}

/// # Errors
/// 400 on a malformed body/email, 409 if the email is already registered,
/// 500 on a hashing or database failure.
pub(crate) async fn register(
    State(state): State<AppState>,
    req: Result<Json<AuthRequest>, JsonRejection>,
) -> ApiResult<(StatusCode, Json<RegisterResponse>)> {
    let req = body(req)?;
    let email = Email::parse(&req.email).map_err(|_| ApiError::Validation { field: "email" })?;

    let hash = password::hash(&req.password).map_err(ApiError::Internal)?;
    let user_id = db::insert_user(&state.pool, email.as_str(), &hash).await?;

    Ok((StatusCode::CREATED, Json(RegisterResponse { user_id })))
}

/// # Errors
/// 400 on a malformed body/email, 401 on unknown email or wrong password,
/// 500 on a database or token-signing failure.
pub(crate) async fn login(
    State(state): State<AppState>,
    req: Result<Json<AuthRequest>, JsonRejection>,
) -> ApiResult<Json<LoginResponse>> {
    let req = body(req)?;
    let email = Email::parse(&req.email).map_err(|_| ApiError::Validation { field: "email" })?;

    let lookup = db::find_row_by_email(&state.pool, email.as_str()).await?;

    let Some(row) = lookup else {
        // Best-effort timing-equalization: hash the supplied password even when
        // the email is unknown, so response latency doesn't leak account
        // existence. Not a hard constant-time guarantee — `hash` (salt-gen +
        // derive) and `verify` (PHC-parse + derive) differ; rate-limiting is
        // the real defence (deferred, see SPEC-0002 §4).
        let _ = password::hash(&req.password);
        return Err(ApiError::Unauthorized);
    };

    let Some(hash) = row.password_hash.as_deref() else {
        let _ = password::hash(&req.password);
        return Err(ApiError::Unauthorized);
    };

    if password::verify(&req.password, hash).is_err() {
        return Err(ApiError::Unauthorized);
    }

    let user_id = UserId(row.id);
    let (token, expires_at) =
        token::encode(user_id, state.jwt_ttl, &state.jwt_secret).map_err(ApiError::Internal)?;
    Ok(Json(LoginResponse {
        token,
        user_id,
        expires_at,
    }))
}

pub(crate) async fn me(user: AuthenticatedUser) -> Json<MeResponse> {
    Json(MeResponse {
        user_id: user.user_id,
    })
}

/// `POST /auth/google` — verify ID token, find-or-create by email, issue JWT.
pub(crate) async fn login_google(
    State(state): State<AppState>,
    req: Result<Json<GoogleLoginRequest>, JsonRejection>,
) -> ApiResult<Json<LoginResponse>> {
    let Json(req) = req.map_err(|_| ApiError::Validation { field: "body" })?;
    let audience = state
        .google
        .audience
        .as_deref()
        .ok_or(ApiError::Unauthorized)?;
    let GoogleClaims { email } = state
        .google
        .verifier
        .verify(&req.id_token, audience)
        .await
        .map_err(|()| ApiError::Unauthorized)?;
    let email = Email::parse(&email).map_err(|_| ApiError::Unauthorized)?;
    let user_id = db::find_or_create_google_user(&state.pool, email.as_str()).await?;
    let (token, expires_at) =
        token::encode(user_id, state.jwt_ttl, &state.jwt_secret).map_err(ApiError::Internal)?;
    Ok(Json(LoginResponse {
        token,
        user_id,
        expires_at,
    }))
}
