//! `AuthenticatedUser` extractor — turns a Bearer header into a user id.

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use uuid::Uuid;

use fitai_core::UserId;

use crate::{auth::token, db, error::ApiError, AppState};

pub struct AuthenticatedUser {
    pub user_id: UserId,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;

        let token_str = header
            .strip_prefix("Bearer ")
            .ok_or(ApiError::Unauthorized)?;
        let claims = token::decode_token(token_str, &state.jwt_secret)
            .map_err(|_| ApiError::Unauthorized)?;

        let uuid = Uuid::parse_str(&claims.sub).map_err(|_| ApiError::Unauthorized)?;
        let user_id = UserId(uuid);

        // AC5: confirm the user still exists.
        let user = db::find_auth_row_by_id(&state.pool, user_id)
            .await
            .map_err(|_| ApiError::Unauthorized)?
            .ok_or(ApiError::Unauthorized)?;

        // R-0060 AC4: a token issued at or before the last password change is
        // dead. `<=` fails closed on the same second, the second an attacker
        // holding the old password would use (SPEC-0060 §2.5).
        if user
            .password_changed_at
            .is_some_and(|changed| claims.iat <= changed.timestamp())
        {
            return Err(ApiError::Unauthorized);
        }

        Ok(AuthenticatedUser {
            user_id: UserId(user.id),
        })
    }
}
