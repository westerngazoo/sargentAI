//! SAC6 → AC6: plaintext passwords never appear in tracing output.
//!
//! Authored by the qa agent during R-0002 step 3. `#[traced_test]` (from the
//! `tracing-test` dev-dependency) installs a capturing subscriber for the
//! duration of the test; `logs_assert` inspects everything emitted while the
//! register path runs with a recognisable password.
//!
//! The argon2 PHC hash and the `user_id` MAY appear; the literal plaintext
//! password MUST NOT.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use axum::http::StatusCode;
use common::{build_app, post_json};
use serde_json::json;
use sqlx::PgPool;
use tracing_test::traced_test;

const RECOGNISABLE_PW: &str = "recognisable-plaintext-pw";

/// SAC6: register a user with a recognisable plaintext password and assert the
/// captured tracing output never contains that plaintext.
#[traced_test]
#[sqlx::test(migrations = "../../migrations")]
async fn register_does_not_log_plaintext_password(pool: PgPool) {
    let app = build_app(pool);

    let resp = post_json(
        &app,
        "/auth/register",
        json!({ "email": "logsafe@b.com", "password": RECOGNISABLE_PW }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    logs_assert(|lines: &[&str]| {
        for line in lines {
            if line.contains(RECOGNISABLE_PW) {
                return Err(format!(
                    "tracing output leaked the plaintext password: {line}"
                ));
            }
        }
        Ok(())
    });
}

/// SAC6: the same guarantee must hold for a failed login (wrong password is
/// also plaintext that must not be logged).
#[traced_test]
#[sqlx::test(migrations = "../../migrations")]
async fn failed_login_does_not_log_plaintext_password(pool: PgPool) {
    let app = build_app(pool);

    let reg = post_json(
        &app,
        "/auth/register",
        json!({ "email": "logsafe2@b.com", "password": "8charsmin" }),
    )
    .await;
    assert_eq!(reg.status(), StatusCode::CREATED);

    let resp = post_json(
        &app,
        "/auth/login",
        json!({ "email": "logsafe2@b.com", "password": RECOGNISABLE_PW }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    logs_assert(|lines: &[&str]| {
        for line in lines {
            if line.contains(RECOGNISABLE_PW) {
                return Err(format!(
                    "tracing output leaked the plaintext password: {line}"
                ));
            }
        }
        Ok(())
    });
}
