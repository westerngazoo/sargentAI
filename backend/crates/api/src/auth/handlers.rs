//! HTTP handlers for the auth endpoints.

use axum::{
    extract::{rejection::JsonRejection, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::Instrument;
use validator::Validate;

use fitai_core::{Email, UserId};

use crate::{
    auth::{
        google::{GoogleClaims, GoogleLoginRequest},
        password,
        reset::{self, ResetToken, RESET_TOKEN_TTL},
        token, AuthenticatedUser,
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
///
/// Every request type it serves validates only `password`, so a validation
/// failure names that field.
fn body<T: DeserializeOwned + Validate>(req: Result<Json<T>, JsonRejection>) -> ApiResult<T> {
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
    /// The account's normalized address (SPEC-0061 §2.6, Cuenta panel).
    email: String,
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

/// `GET /auth/me` — the caller's own id and email.
///
/// # Errors
/// 401 when the token's user no longer exists (the extractor's convention for
/// a vanished account), 500 on a database failure or a corrupt stored email.
pub(crate) async fn me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<MeResponse>> {
    let account = db::find_user_by_id(&state.pool, user.user_id)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(MeResponse {
        user_id: account.id,
        email: account.email.as_str().to_owned(),
    }))
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

#[derive(Debug, Deserialize, Validate)]
pub(crate) struct ForgotRequest {
    // Normalized and validated by `core::Email::parse`, as in `AuthRequest`.
    email: String,
}

/// Requests for one email address per window (R-0060 AC5).
const RESET_LIMIT_PER_EMAIL: i64 = 3;
/// Requests from one client address per window (R-0060 AC5).
const RESET_LIMIT_PER_CLIENT: i64 = 10;

/// `POST /auth/password/forgot` — always `202`, empty body (R-0060 AC1).
///
/// No account, a Google-only account, a rate-limited request and a real
/// account all answer the same. Only the last stores a token and sends an
/// email, and the email goes out after the response so the slow network call
/// does not separate the cases by timing (SPEC-0060 §2.4).
///
/// # Errors
/// 400 on a malformed body or email (before any lookup, so it reveals
/// nothing), 500 on a database failure.
pub(crate) async fn forgot_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    req: Result<Json<ForgotRequest>, JsonRejection>,
) -> ApiResult<StatusCode> {
    let req = body(req)?;
    let email = Email::parse(&req.email).map_err(|_| ApiError::Validation { field: "email" })?;

    // Cloudflare's edge sets this on every request that reaches the container;
    // absent in tests and local runs, where only the email key applies.
    let client = headers
        .get("cf-connecting-ip")
        .and_then(|v| v.to_str().ok());
    if over_reset_limit(&state, email.as_str(), client).await? {
        return Ok(StatusCode::ACCEPTED);
    }

    let Some(row) = db::find_row_by_email(&state.pool, email.as_str()).await? else {
        return Ok(StatusCode::ACCEPTED);
    };
    if row.password_hash.is_none() {
        return Ok(StatusCode::ACCEPTED); // Google-only: nothing to reset (R-0060 OQ-2)
    }

    let token = ResetToken::generate();
    let expires_at = Utc::now()
        + chrono::Duration::from_std(RESET_TOKEN_TTL).map_err(|e| ApiError::Internal(e.into()))?;
    db::replace_reset_token(
        &state.pool,
        UserId(row.id),
        token.hash().as_bytes(),
        expires_at,
    )
    .await?;

    let message = reset::message(&state.mail.public_web_url, &row.email, &token);
    let mailer = state.mail.mailer.clone();
    // Detached on purpose (SPEC-0060 §2.4): a shutdown inside the send loses
    // that one email, and the stored token stays valid for a retry.
    tokio::spawn(
        async move {
            if let Err(e) = mailer.send(message).await {
                tracing::error!(error = %e, "password reset email not sent");
            }
        }
        .instrument(tracing::Span::current()),
    );
    Ok(StatusCode::ACCEPTED)
}

/// Record this request under its email key and, when known, its client
/// key; `true` if either is over its limit. Both keys are always recorded,
/// so the ledger counts unknown addresses too and the limit itself reveals
/// nothing.
async fn over_reset_limit(state: &AppState, email: &str, client: Option<&str>) -> ApiResult<bool> {
    let by_email =
        db::record_reset_request(&state.pool, &reset::ledger_key("email", email)).await?;
    let by_client = match client {
        Some(ip) => db::record_reset_request(&state.pool, &reset::ledger_key("ip", ip)).await?,
        None => 0,
    };
    Ok(by_email > RESET_LIMIT_PER_EMAIL || by_client > RESET_LIMIT_PER_CLIENT)
}

// No `Debug`: it holds the plaintext token and password, and nothing should
// be able to format them (R-0060 AC2).
#[derive(Deserialize, Validate)]
pub(crate) struct ResetRequest {
    token: String,
    #[validate(length(min = 8))]
    password: String,
}

/// `POST /auth/password/reset` — `204` on success (R-0060 AC3).
///
/// Unknown, malformed, used, expired and superseded tokens all get the same
/// `401 unauthorized`, so the response never says which.
///
/// # Errors
/// 400 on a malformed body or a password shorter than registration allows
/// (checked before the token, so a short password does not burn the link),
/// 401 on any bad token, 500 on a hashing or database failure.
pub(crate) async fn reset_password(
    State(state): State<AppState>,
    req: Result<Json<ResetRequest>, JsonRejection>,
) -> ApiResult<StatusCode> {
    let req = body(req)?;
    let token = ResetToken::parse(&req.token).ok_or(ApiError::Unauthorized)?;
    let hash = password::hash(&req.password).map_err(ApiError::Internal)?;
    let consumed =
        db::consume_reset_token(&state.pool, token.hash().as_bytes(), &hash, Utc::now()).await?;
    if consumed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::Unauthorized)
    }
}
