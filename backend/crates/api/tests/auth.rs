//! R-0002 auth integration suite — register, login, /auth/me.
//!
//! Authored by the qa agent during R-0002 step 3 (test planning), BEFORE the
//! auth implementation exists. Pre-implementation red state = compile failure
//! (`fitai_api::AppState`, `app(state)`, the `/auth/*` routes, and the new
//! deps are all absent). Implementation step 5 makes these green.
//!
//! Every test is `#[sqlx::test(migrations = "../../migrations")]` per
//! SPEC-0002 §2.5: sqlx provisions a fresh per-test database, applies the
//! migrations, and hands a connected `PgPool` to the test — trivially
//! isolated (SAC7).
//!
//! SAC → test traceability lives in the qa sign-off report; each test below is
//! tagged inline with the SAC branch it verifies.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use common::{
    body_bytes, body_json, build_app, get_with_auth, post_json, register_user, TEST_SECRET,
};
use serde_json::json;
use sqlx::{PgPool, Row};

// ---------------------------------------------------------------------------
// SAC1 → AC1: migration / users table shape.
// ---------------------------------------------------------------------------

/// SAC1: the migration runs (sqlx::test applied it) and the `users` table has
/// the four expected columns with the expected nullability.
#[sqlx::test(migrations = "../../migrations")]
async fn migration_creates_users_table_with_expected_columns(pool: PgPool) {
    let rows = sqlx::query(
        "SELECT column_name, is_nullable, data_type \
         FROM information_schema.columns WHERE table_name = 'users'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let mut columns: Vec<(String, String, String)> = rows
        .iter()
        .map(|r| {
            (
                r.get::<String, _>("column_name"),
                r.get::<String, _>("is_nullable"),
                r.get::<String, _>("data_type"),
            )
        })
        .collect();
    columns.sort();

    let names: Vec<&str> = columns.iter().map(|(n, _, _)| n.as_str()).collect();
    assert_eq!(
        names,
        vec!["created_at", "email", "id", "password_hash"],
        "users must have exactly id, email, password_hash, created_at"
    );

    for (name, nullable, _) in &columns {
        assert_eq!(nullable, "NO", "column `{name}` must be NOT NULL");
    }
}

/// SAC1: the UNIQUE constraint on `email` is enforced at the schema level.
#[sqlx::test(migrations = "../../migrations")]
async fn users_email_is_unique(pool: PgPool) {
    sqlx::query("INSERT INTO users (email, password_hash) VALUES ($1, $2)")
        .bind("dup@example.com")
        .bind("$argon2id$placeholder")
        .execute(&pool)
        .await
        .unwrap();

    let second = sqlx::query("INSERT INTO users (email, password_hash) VALUES ($1, $2)")
        .bind("dup@example.com")
        .bind("$argon2id$placeholder")
        .execute(&pool)
        .await;

    assert!(
        second.is_err(),
        "a second row with the same email must violate the UNIQUE constraint"
    );
}

// ---------------------------------------------------------------------------
// SAC2 → AC2: register.
// ---------------------------------------------------------------------------

/// SAC2: first-time valid register returns 201 + `{ user_id }`, and persists
/// exactly one row whose `password_hash` is argon2id (`$argon2id$…`).
#[sqlx::test(migrations = "../../migrations")]
async fn register_success_persists_argon2id_hash(pool: PgPool) {
    let app = build_app(pool.clone());

    let resp = post_json(
        &app,
        "/auth/register",
        json!({ "email": "a@b.com", "password": "8charsmin" }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    let user_id = body["user_id"].as_str().expect("user_id must be a string");
    assert!(
        uuid::Uuid::parse_str(user_id).is_ok(),
        "user_id must be a valid UUID, got {user_id}"
    );

    let row = sqlx::query("SELECT email, password_hash FROM users")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(row.len(), 1, "exactly one user row must exist after register");
    let hash: String = row[0].get("password_hash");
    assert!(
        hash.starts_with("$argon2id$"),
        "password_hash must be argon2id PHC format, got: {hash}"
    );
    let stored_email: String = row[0].get("email");
    assert_eq!(stored_email, "a@b.com", "stored email must be normalized");
}

/// SAC2: a second identical register returns 409 + `already_exists`.
#[sqlx::test(migrations = "../../migrations")]
async fn register_duplicate_email_conflicts(pool: PgPool) {
    let app = build_app(pool);

    register_user(&app, "dup@b.com", "8charsmin").await;

    let resp = post_json(
        &app,
        "/auth/register",
        json!({ "email": "dup@b.com", "password": "8charsmin" }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::CONFLICT);
    assert_eq!(body_json(resp).await, json!({ "error": "already_exists" }));
}

/// SAC2: the same address differing only in case/whitespace is the SAME
/// account — normalization (`core::Email`) makes the duplicate collide → 409.
#[sqlx::test(migrations = "../../migrations")]
async fn register_duplicate_is_case_insensitive(pool: PgPool) {
    let app = build_app(pool);

    register_user(&app, "Case@B.com", "8charsmin").await;

    let resp = post_json(
        &app,
        "/auth/register",
        json!({ "email": "  case@b.COM  ", "password": "8charsmin" }),
    )
    .await;

    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "normalized email must collide regardless of case/whitespace"
    );
}

/// SAC2: malformed email returns 400 + `{ error: validation, field: email }`.
#[sqlx::test(migrations = "../../migrations")]
async fn register_bad_email_is_bad_request(pool: PgPool) {
    let app = build_app(pool);

    let resp = post_json(
        &app,
        "/auth/register",
        json!({ "email": "not-an-email", "password": "8charsmin" }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "validation", "field": "email" })
    );
}

/// SAC2: missing `password` field returns 400 (mapped via JsonRejection, not
/// axum's default extractor rejection).
#[sqlx::test(migrations = "../../migrations")]
async fn register_missing_password_is_bad_request(pool: PgPool) {
    let app = build_app(pool);

    let resp = post_json(&app, "/auth/register", json!({ "email": "a@b.com" })).await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        body_json(resp).await["error"], "validation",
        "missing field must surface as a validation error, not a 422 rejection"
    );
}

/// SAC2: empty password (below min length) returns 400.
#[sqlx::test(migrations = "../../migrations")]
async fn register_empty_password_is_bad_request(pool: PgPool) {
    let app = build_app(pool);

    let resp = post_json(
        &app,
        "/auth/register",
        json!({ "email": "a@b.com", "password": "" }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// SAC2: a too-short (<8) password returns 400 (validator length(min=8)).
#[sqlx::test(migrations = "../../migrations")]
async fn register_short_password_is_bad_request(pool: PgPool) {
    let app = build_app(pool);

    let resp = post_json(
        &app,
        "/auth/register",
        json!({ "email": "a@b.com", "password": "short" }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------------------
// SAC3 → AC3: login.
// ---------------------------------------------------------------------------

/// SAC3: valid credentials return 200 + a non-empty token, the matching
/// user_id, and `expires_at` ≈ now + 24h (± 5 s).
#[sqlx::test(migrations = "../../migrations")]
async fn login_success_returns_token(pool: PgPool) {
    let app = build_app(pool);
    let user_id = register_user(&app, "login@b.com", "8charsmin").await;

    let resp = post_json(
        &app,
        "/auth/login",
        json!({ "email": "login@b.com", "password": "8charsmin" }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    assert!(
        !body["token"].as_str().unwrap().is_empty(),
        "token must be non-empty"
    );
    assert_eq!(body["user_id"].as_str().unwrap(), user_id);

    let expires_at: DateTime<Utc> = body["expires_at"]
        .as_str()
        .expect("expires_at must be an rfc3339 string")
        .parse()
        .expect("expires_at must parse as rfc3339");
    let delta = (expires_at - Utc::now()).num_seconds();
    assert!(
        (86_395..=86_405).contains(&delta),
        "expires_at must be ~24h out, was {delta}s"
    );
}

/// SAC3: wrong password returns 401 + `unauthorized`.
#[sqlx::test(migrations = "../../migrations")]
async fn login_wrong_password_unauthorized(pool: PgPool) {
    let app = build_app(pool);
    register_user(&app, "wp@b.com", "8charsmin").await;

    let resp = post_json(
        &app,
        "/auth/login",
        json!({ "email": "wp@b.com", "password": "wrongpassword" }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(resp).await, json!({ "error": "unauthorized" }));
}

/// SAC3: unknown email returns 401 with a body byte-for-byte identical to the
/// wrong-password case (enumeration-safe).
#[sqlx::test(migrations = "../../migrations")]
async fn login_unknown_email_is_indistinguishable(pool: PgPool) {
    let app = build_app(pool);
    register_user(&app, "known@b.com", "8charsmin").await;

    let wrong_pw = post_json(
        &app,
        "/auth/login",
        json!({ "email": "known@b.com", "password": "wrongpassword" }),
    )
    .await;
    let wrong_pw_status = wrong_pw.status();
    let wrong_pw_body = body_bytes(wrong_pw).await;

    let unknown = post_json(
        &app,
        "/auth/login",
        json!({ "email": "ghost@b.com", "password": "8charsmin" }),
    )
    .await;
    let unknown_status = unknown.status();
    let unknown_body = body_bytes(unknown).await;

    assert_eq!(unknown_status, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong_pw_status, unknown_status, "statuses must match");
    assert_eq!(
        wrong_pw_body, unknown_body,
        "bodies must be byte-for-byte identical to avoid email enumeration"
    );
}

// ---------------------------------------------------------------------------
// SAC5 → AC5: /auth/me extractor (1 success + 5 failure branches).
// ---------------------------------------------------------------------------

/// SAC5: valid Bearer token → 200 + `{ user_id }`.
#[sqlx::test(migrations = "../../migrations")]
async fn me_with_valid_token_succeeds(pool: PgPool) {
    let app = build_app(pool);
    let user_id = register_user(&app, "me@b.com", "8charsmin").await;

    let login = post_json(
        &app,
        "/auth/login",
        json!({ "email": "me@b.com", "password": "8charsmin" }),
    )
    .await;
    let token = body_json(login).await["token"].as_str().unwrap().to_string();

    let resp = get_with_auth(&app, "/auth/me", Some(&format!("Bearer {token}"))).await;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(body_json(resp).await["user_id"].as_str().unwrap(), user_id);
}

/// SAC5(a): missing Authorization header → 401.
#[sqlx::test(migrations = "../../migrations")]
async fn me_missing_header_unauthorized(pool: PgPool) {
    let app = build_app(pool);

    let resp = get_with_auth(&app, "/auth/me", None).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(resp).await, json!({ "error": "unauthorized" }));
}

/// SAC5(b): wrong scheme (`Token <jwt>` instead of `Bearer <jwt>`) → 401.
#[sqlx::test(migrations = "../../migrations")]
async fn me_wrong_scheme_unauthorized(pool: PgPool) {
    let app = build_app(pool);
    register_user(&app, "scheme@b.com", "8charsmin").await;

    let login = post_json(
        &app,
        "/auth/login",
        json!({ "email": "scheme@b.com", "password": "8charsmin" }),
    )
    .await;
    let token = body_json(login).await["token"].as_str().unwrap().to_string();

    let resp = get_with_auth(&app, "/auth/me", Some(&format!("Token {token}"))).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// SAC5(c): a valid token with one signature character flipped → 401.
#[sqlx::test(migrations = "../../migrations")]
async fn me_tampered_signature_unauthorized(pool: PgPool) {
    let app = build_app(pool);
    register_user(&app, "tamper@b.com", "8charsmin").await;

    let login = post_json(
        &app,
        "/auth/login",
        json!({ "email": "tamper@b.com", "password": "8charsmin" }),
    )
    .await;
    let token = body_json(login).await["token"].as_str().unwrap().to_string();

    // Flip the final character of the signature segment.
    let last = token.chars().last().unwrap();
    let replacement = if last == 'a' { 'b' } else { 'a' };
    let tampered: String = token[..token.len() - 1].to_string() + &replacement.to_string();

    let resp = get_with_auth(&app, "/auth/me", Some(&format!("Bearer {tampered}"))).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// SAC5(e): a structurally valid, correctly-signed token whose `sub` UUID is
/// not in `users` → 401.
///
/// We mint the token by registering then deleting the user, so the token's
/// signature is genuine but the subject no longer resolves.
#[sqlx::test(migrations = "../../migrations")]
async fn me_unknown_subject_unauthorized(pool: PgPool) {
    let app = build_app(pool.clone());
    let user_id = register_user(&app, "ghost@b.com", "8charsmin").await;

    let login = post_json(
        &app,
        "/auth/login",
        json!({ "email": "ghost@b.com", "password": "8charsmin" }),
    )
    .await;
    let token = body_json(login).await["token"].as_str().unwrap().to_string();

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&user_id).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    let resp = get_with_auth(&app, "/auth/me", Some(&format!("Bearer {token}"))).await;

    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "a token whose sub is no longer in users must be rejected"
    );
}

// ---------------------------------------------------------------------------
// SAC4 → AC4: token claims, signed with JWT_SECRET. Verified at the HTTP
// boundary by decoding the issued token with the same/other secret.
// ---------------------------------------------------------------------------

/// SAC4: the issued JWT decodes with the test secret to `sub = user_id`, and
/// `exp - iat` is 24h ± 5 s.
#[sqlx::test(migrations = "../../migrations")]
async fn issued_token_carries_expected_claims(pool: PgPool) {
    use jsonwebtoken::{decode, DecodingKey, Validation};
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Claims {
        sub: String,
        iat: i64,
        exp: i64,
    }

    let app = build_app(pool);
    let user_id = register_user(&app, "claims@b.com", "8charsmin").await;

    let login = post_json(
        &app,
        "/auth/login",
        json!({ "email": "claims@b.com", "password": "8charsmin" }),
    )
    .await;
    let token = body_json(login).await["token"].as_str().unwrap().to_string();

    let data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(TEST_SECRET),
        &Validation::default(),
    )
    .expect("token must decode with the signing secret");

    assert_eq!(data.claims.sub, user_id, "sub must be the user_id string");
    let window = data.claims.exp - data.claims.iat;
    assert!(
        (86_395..=86_405).contains(&window),
        "exp - iat must be 24h ± 5s, was {window}s"
    );
}

/// SAC4: decoding the issued token with a DIFFERENT secret fails with an
/// `InvalidSignature` error.
#[sqlx::test(migrations = "../../migrations")]
async fn issued_token_rejects_wrong_secret(pool: PgPool) {
    use jsonwebtoken::{decode, errors::ErrorKind, DecodingKey, Validation};
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Claims {
        sub: String,
        iat: i64,
        exp: i64,
    }

    let app = build_app(pool);
    register_user(&app, "sig@b.com", "8charsmin").await;

    let login = post_json(
        &app,
        "/auth/login",
        json!({ "email": "sig@b.com", "password": "8charsmin" }),
    )
    .await;
    let token = body_json(login).await["token"].as_str().unwrap().to_string();

    let err = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(b"a-completely-different-secret"),
        &Validation::default(),
    )
    .expect_err("decoding with the wrong secret must fail");

    assert!(
        matches!(err.kind(), ErrorKind::InvalidSignature),
        "wrong-secret decode must be InvalidSignature, was {:?}",
        err.kind()
    );
}
