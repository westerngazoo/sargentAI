//! HS256 JWT encode + decode for fitai-api.

use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use jsonwebtoken::{decode, encode as jwt_encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use fitai_core::UserId;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Claims {
    /// User id as a string (Uuid).
    pub(crate) sub: String,
    /// Issued-at, seconds since epoch.
    pub(crate) iat: i64,
    /// Expiry, seconds since epoch.
    pub(crate) exp: i64,
}

/// Encode an HS256 JWT for `user_id` valid for `ttl`. Returns the token **and**
/// the exact `exp` instant it carries, so callers report the token's real
/// expiry rather than recomputing `now + ttl` from a second clock read.
///
/// # Errors
/// Returns an error if JWT signing fails or the computed `exp` is out of range.
pub(crate) fn encode(
    user_id: UserId,
    ttl: Duration,
    secret: &[u8],
) -> eyre::Result<(String, DateTime<Utc>)> {
    let iat = Utc::now().timestamp();
    let exp = iat + i64::try_from(ttl.as_secs()).unwrap_or(i64::MAX);
    let claims = Claims {
        sub: user_id.0.to_string(),
        iat,
        exp,
    };
    let token = jwt_encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )?;
    let expires_at = Utc
        .timestamp_opt(exp, 0)
        .single()
        .ok_or_else(|| eyre::eyre!("exp timestamp out of range"))?;
    Ok((token, expires_at))
}

/// Decode and validate an HS256 JWT. Validates the signature and the `exp`
/// claim with **zero leeway** — a token is rejected the moment it expires,
/// rather than `Validation::default()`'s 60-second grace window.
///
/// # Errors
/// Returns an error if the signature is invalid, the token is expired, or the
/// claims are malformed.
pub(crate) fn decode_token(token: &str, secret: &[u8]) -> eyre::Result<Claims> {
    let mut validation = Validation::default(); // HS256 + exp check
    validation.leeway = 0;
    let data = decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)?;
    Ok(data.claims)
}
