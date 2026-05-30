//! SAC5(d) → AC5: expired-token rejection.
//!
//! Authored by the qa agent during R-0002 step 3. Separated from `auth.rs`
//! because it needs a router whose `jwt_ttl` is `Duration::ZERO` — minting a
//! token that is already expired at issuance — which exercises
//! `Validation::default()`'s `exp` check in the extractor.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::time::Duration;

use axum::http::StatusCode;
use common::{app_with_ttl, body_json, get_with_auth, post_json};
use serde_json::json;
use sqlx::PgPool;

/// SAC5(d): a token issued with `jwt_ttl = 0` is already expired; `/auth/me`
/// must reject it with 401.
#[sqlx::test(migrations = "../../migrations")]
async fn me_expired_token_unauthorized(pool: PgPool) {
    // TTL = 0 → exp == iat == now, so the token is expired on arrival.
    let app = app_with_ttl(pool, Duration::ZERO);

    let reg = post_json(
        &app,
        "/auth/register",
        json!({ "email": "expired@b.com", "password": "8charsmin" }),
    )
    .await;
    assert_eq!(reg.status(), StatusCode::CREATED);

    let login = post_json(
        &app,
        "/auth/login",
        json!({ "email": "expired@b.com", "password": "8charsmin" }),
    )
    .await;
    assert_eq!(login.status(), StatusCode::OK);
    let token = body_json(login).await["token"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = get_with_auth(&app, "/auth/me", Some(&format!("Bearer {token}"))).await;

    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "an already-expired token must be rejected by /auth/me"
    );
}
