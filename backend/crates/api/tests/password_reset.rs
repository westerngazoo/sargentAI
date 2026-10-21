//! R-0060 password-reset integration suite — `/auth/password/forgot` and
//! `/auth/password/reset` (SPEC-0060 §2.4), session invalidation (§2.5), the
//! email (§2.6) and the schema (§2.1).
//!
//! Authored by the qa agent during R-0060 step 3 (test planning), BEFORE the
//! implementation exists. Pre-implementation red state = compile failure:
//! `fitai_api::mail::{Message, RecordingMailer, MailSettings}`, the `mail`
//! field on `AppState`, and the two routes are all absent. Step 5 makes these
//! green without edits to this file.
//!
//! Test ids are SPEC-0060 §7's T1–T14 (plus T9b, a boundary case of T9, and
//! the e2e run the spec assigns to QA). Each is tagged inline with the
//! R-0060 acceptance criterion it verifies.
//!
//! Contract this suite binds the test double to (SPEC-0060 §2.3):
//!   - `RecordingMailer::default()` records every `send`;
//!   - `async fn wait_for(&self, n: usize, timeout: Duration) -> bool` resolves
//!     `true` once at least `n` messages are recorded, `false` on timeout;
//!   - `fn sent(&self) -> Vec<Message>` is a snapshot of what was recorded.
//! Every assertion on `sent()` is preceded by `wait_for`, so nothing races
//! the spawned send (§2.4 step 4).
//!
//! Token hashing contract (SPEC-0060 §2.2): `token_hash` is the SHA-256 of
//! the 64-character hex string's bytes (the exact string carried in the link),
//! not of the 32 decoded bytes. T4 computes the digest independently in
//! Postgres (`sha256(convert_to(token, 'UTF8'))`), and U3 in `auth/reset.rs`
//! pins the same vector.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use common::{
    body_bytes, body_json, build_app, build_app_with_mailer, get_with_auth, post_json,
    register_user, TEST_PUBLIC_WEB_URL, TEST_SECRET,
};
use fitai_api::mail::{Message, RecordingMailer};
use serde_json::json;
use sqlx::{PgPool, Row};
use tracing_test::traced_test;
use uuid::Uuid;

const PASSWORD: &str = "old-password-1";
const NEW_PASSWORD: &str = "new-password-2";

/// How long a test waits for a spawned send to land (SPEC-0060 §7: 2 s).
const SEND_WAIT: Duration = Duration::from_secs(2);
/// How long a test waits before concluding that NO send happened (§7 T2).
const NO_SEND_WAIT: Duration = Duration::from_millis(300);

/// T11's bound on the median response-time difference between known and
/// unknown addresses (SPEC-0060 §7: "less than 50 ms"). If this flakes in CI
/// the spec's OQ-2 applies: keep it as a hand-run benchmark, keep the
/// structural assertion. Named so that decision is one line.
const TIMING_TOLERANCE: Duration = Duration::from_millis(50);

/// The 64-hex token carried by a recorded reset email (SPEC-0060 §2.6 link:
/// `{PUBLIC_WEB_URL}/#/reset-password?token=<token>`).
fn reset_token_in(message: &Message) -> String {
    let marker = "#/reset-password?token=";
    let start = message
        .text
        .find(marker)
        .expect("reset email text must carry the reset link")
        + marker.len();
    message.text[start..start + 64].to_string()
}

async fn forgot(app: &axum::Router, email: &str) -> axum::response::Response {
    post_json(app, "/auth/password/forgot", json!({ "email": email })).await
}

async fn reset(app: &axum::Router, token: &str, password: &str) -> axum::response::Response {
    post_json(
        app,
        "/auth/password/reset",
        json!({ "token": token, "password": password }),
    )
    .await
}

async fn login_status(app: &axum::Router, email: &str, password: &str) -> StatusCode {
    post_json(
        app,
        "/auth/login",
        json!({ "email": email, "password": password }),
    )
    .await
    .status()
}

/// Register `email`, ask for a reset, and return the emailed token. Seeds the
/// reset tests that start from "a valid token exists".
async fn registered_user_with_token(
    app: &axum::Router,
    mailer: &RecordingMailer,
    email: &str,
) -> (String, String) {
    let user_id = register_user(app, email, PASSWORD).await;
    assert_eq!(forgot(app, email).await.status(), StatusCode::ACCEPTED);
    assert!(
        mailer.wait_for(1, SEND_WAIT).await,
        "the reset email must be sent"
    );
    let token = reset_token_in(&mailer.sent()[0]);
    (user_id, token)
}

// ---------------------------------------------------------------------------
// AC1 — request a reset: identical response, email only for real accounts.
// ---------------------------------------------------------------------------

/// T1 → AC1: a known email → 202, one message recorded, addressed to it.
#[sqlx::test(migrations = "../../migrations")]
async fn t1_known_email_is_accepted_and_emailed(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool, mailer.clone());
    register_user(&app, "known@b.com", PASSWORD).await;

    let resp = forgot(&app, "known@b.com").await;

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    assert!(body_bytes(resp).await.is_empty(), "202 body must be empty");
    assert!(
        mailer.wait_for(1, SEND_WAIT).await,
        "one reset email must be sent"
    );
    let sent = mailer.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "known@b.com");
}

/// T2 → AC1: an unknown email → the same 202 with the same empty body, and
/// nothing is sent.
#[sqlx::test(migrations = "../../migrations")]
async fn t2_unknown_email_gets_identical_response_and_no_email(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool, mailer.clone());
    register_user(&app, "known@b.com", PASSWORD).await;

    let known = forgot(&app, "known@b.com").await;
    let known_status = known.status();
    let known_body = body_bytes(known).await;
    assert!(mailer.wait_for(1, SEND_WAIT).await);

    let unknown = forgot(&app, "ghost@b.com").await;
    let unknown_status = unknown.status();
    let unknown_body = body_bytes(unknown).await;

    assert_eq!(unknown_status, StatusCode::ACCEPTED);
    assert_eq!(known_status, unknown_status, "statuses must match");
    assert_eq!(
        known_body, unknown_body,
        "bodies must be byte-for-byte identical"
    );
    assert!(
        !mailer.wait_for(2, NO_SEND_WAIT).await,
        "an unknown address must not produce a second email"
    );
    assert_eq!(mailer.sent().len(), 1);
}

/// T3 → AC1 (OQ-2): a Google-only account (`password_hash IS NULL`) → 202,
/// nothing sent, no token row.
#[sqlx::test(migrations = "../../migrations")]
async fn t3_google_only_account_gets_nothing(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool.clone(), mailer.clone());
    sqlx::query("INSERT INTO users (email, password_hash) VALUES ($1, NULL)")
        .bind("google@b.com")
        .execute(&pool)
        .await
        .unwrap();

    let resp = forgot(&app, "google@b.com").await;

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    assert!(
        !mailer.wait_for(1, NO_SEND_WAIT).await,
        "no email for a Google-only account"
    );
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM password_reset_tokens")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 0, "no token row for a Google-only account");
}

/// AC1 edge: a malformed email is a 400 validation error — before any lookup,
/// so it reveals nothing — and sends nothing.
#[sqlx::test(migrations = "../../migrations")]
async fn forgot_with_malformed_email_is_bad_request(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool, mailer.clone());

    let resp = forgot(&app, "not-an-email").await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "validation");
    assert!(!mailer.wait_for(1, NO_SEND_WAIT).await);
}

/// AC1 edge: the address is normalized like registration, so a padded,
/// mixed-case spelling of a known address still reaches that account.
#[sqlx::test(migrations = "../../migrations")]
async fn forgot_normalizes_the_address_like_register(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool, mailer.clone());
    register_user(&app, "case@b.com", PASSWORD).await;

    let resp = forgot(&app, "  Case@B.COM  ").await;

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    assert!(
        mailer.wait_for(1, SEND_WAIT).await,
        "the normalized address must match"
    );
    assert_eq!(mailer.sent()[0].to, "case@b.com");
}

// ---------------------------------------------------------------------------
// AC2 + AC6 — the token in the email, and the email itself.
// ---------------------------------------------------------------------------

/// T4 → AC2, AC6: the message carries the `#/reset-password?token=` link at
/// the configured origin; the token's SHA-256 (computed independently in
/// Postgres) equals the stored `token_hash`; the plaintext token is not in
/// the database; the message carries no other account data.
#[sqlx::test(migrations = "../../migrations")]
async fn t4_email_carries_link_whose_token_hash_is_stored(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool.clone(), mailer.clone());
    let (user_id, token) = registered_user_with_token(&app, &mailer, "link@b.com").await;
    let message = &mailer.sent()[0];

    // The link, at the configured origin, token in the fragment (§2.6).
    let link = format!("{TEST_PUBLIC_WEB_URL}/#/reset-password?token={token}");
    assert!(
        message.text.contains(&link),
        "text must carry {link}, got: {}",
        message.text
    );
    assert!(
        message.html.contains(&link),
        "html must carry the same link"
    );
    assert_eq!(token.len(), 64);
    assert!(token
        .bytes()
        .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));

    // Stored as SHA-256 of the token string, never as the token (AC2).
    let matches: bool = sqlx::query_scalar(
        "SELECT token_hash = sha256(convert_to($1, 'UTF8')) \
         FROM password_reset_tokens WHERE user_id = $2",
    )
    .bind(&token)
    .bind(Uuid::parse_str(&user_id).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        matches,
        "token_hash must be the SHA-256 of the emailed token"
    );
    let stored_plaintext: bool = sqlx::query_scalar(
        "SELECT count(*) > 0 FROM password_reset_tokens \
         WHERE token_hash = convert_to($1, 'UTF8')",
    )
    .bind(&token)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        !stored_plaintext,
        "the plaintext token must never be stored"
    );

    // The email (AC6): subject, expiry, the "ignore" line, nothing else.
    assert_eq!(message.subject, "Reset your Goose Fitness Physics password");
    assert!(
        message.text.contains("30 minutes"),
        "text must state the expiry"
    );
    assert!(
        message.text.to_lowercase().contains("ignore"),
        "text must say it can be ignored"
    );
    for body in [&message.text, &message.html] {
        assert!(
            !body.contains(&user_id),
            "the message must not carry the user id"
        );
        assert!(
            !body.contains(PASSWORD),
            "the message must not carry a password"
        );
    }
}

/// AC2: the stored token expires 30 minutes after issue (± 5 s).
#[sqlx::test(migrations = "../../migrations")]
async fn stored_token_expires_thirty_minutes_after_issue(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool.clone(), mailer.clone());
    registered_user_with_token(&app, &mailer, "ttl@b.com").await;

    let row = sqlx::query("SELECT created_at, expires_at FROM password_reset_tokens")
        .fetch_one(&pool)
        .await
        .unwrap();
    let created: DateTime<Utc> = row.get("created_at");
    let expires: DateTime<Utc> = row.get("expires_at");
    let ttl = (expires - created).num_seconds();
    assert!(
        (1_795..=1_805).contains(&ttl),
        "expires_at - created_at must be 30 min, was {ttl}s"
    );
}

/// AC2: two requests produce two different tokens (the token is random).
#[sqlx::test(migrations = "../../migrations")]
async fn two_requests_produce_two_different_tokens(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool, mailer.clone());
    registered_user_with_token(&app, &mailer, "rand@b.com").await;
    assert_eq!(
        forgot(&app, "rand@b.com").await.status(),
        StatusCode::ACCEPTED
    );
    assert!(mailer.wait_for(2, SEND_WAIT).await);

    let sent = mailer.sent();
    assert_ne!(reset_token_in(&sent[0]), reset_token_in(&sent[1]));
}

// ---------------------------------------------------------------------------
// AC3 — set the new password; every bad token is one generic 401.
// ---------------------------------------------------------------------------

/// T5 → AC3: a valid token and a new password → 204; the new password logs
/// in, the old one does not, and the stored hash is argon2id.
#[sqlx::test(migrations = "../../migrations")]
async fn t5_reset_sets_the_new_password(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool.clone(), mailer.clone());
    let (_, token) = registered_user_with_token(&app, &mailer, "reset@b.com").await;

    let resp = reset(&app, &token, NEW_PASSWORD).await;

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(body_bytes(resp).await.is_empty(), "204 body must be empty");
    assert_eq!(
        login_status(&app, "reset@b.com", NEW_PASSWORD).await,
        StatusCode::OK
    );
    assert_eq!(
        login_status(&app, "reset@b.com", PASSWORD).await,
        StatusCode::UNAUTHORIZED
    );
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        hash.starts_with("$argon2id$"),
        "new password must be argon2id, got {hash}"
    );
}

/// T6 → AC2, AC3: the same token a second time → 401 `unauthorized`, and the
/// password set by the first use stays.
#[sqlx::test(migrations = "../../migrations")]
async fn t6_used_token_is_refused(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool, mailer.clone());
    let (_, token) = registered_user_with_token(&app, &mailer, "once@b.com").await;
    assert_eq!(
        reset(&app, &token, NEW_PASSWORD).await.status(),
        StatusCode::NO_CONTENT
    );

    let resp = reset(&app, &token, "third-password-3").await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(resp).await, json!({ "error": "unauthorized" }));
    assert_eq!(
        login_status(&app, "once@b.com", NEW_PASSWORD).await,
        StatusCode::OK
    );
    assert_eq!(
        login_status(&app, "once@b.com", "third-password-3").await,
        StatusCode::UNAUTHORIZED
    );
}

/// T7 → AC2, AC3: a token whose `expires_at` is moved into the past → 401,
/// and the password is unchanged.
#[sqlx::test(migrations = "../../migrations")]
async fn t7_expired_token_is_refused(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool.clone(), mailer.clone());
    let (_, token) = registered_user_with_token(&app, &mailer, "late@b.com").await;
    sqlx::query("UPDATE password_reset_tokens SET expires_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();

    let resp = reset(&app, &token, NEW_PASSWORD).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(resp).await, json!({ "error": "unauthorized" }));
    assert_eq!(
        login_status(&app, "late@b.com", PASSWORD).await,
        StatusCode::OK
    );
}

/// T8 → AC2, AC3: a newer request supersedes the older token (401), the
/// newest works (204); a random 64-hex token → 401; a 10-character string →
/// 401, not 400 (the token's shape is not confirmed by the error); and every
/// refusal has the same body.
#[sqlx::test(migrations = "../../migrations")]
async fn t8_superseded_guessed_and_malformed_tokens_get_one_generic_refusal(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool, mailer.clone());
    let (_, first) = registered_user_with_token(&app, &mailer, "twice@b.com").await;
    assert_eq!(
        forgot(&app, "twice@b.com").await.status(),
        StatusCode::ACCEPTED
    );
    assert!(mailer.wait_for(2, SEND_WAIT).await);
    let second = mailer
        .sent()
        .iter()
        .map(reset_token_in)
        .find(|t| *t != first)
        .expect("the second request must email a different token");

    let superseded = reset(&app, &first, NEW_PASSWORD).await;
    assert_eq!(superseded.status(), StatusCode::UNAUTHORIZED);
    let superseded_body = body_bytes(superseded).await;

    let guessed = reset(&app, &"a".repeat(64), NEW_PASSWORD).await;
    assert_eq!(
        guessed.status(),
        StatusCode::UNAUTHORIZED,
        "a guessed token is refused"
    );
    assert_eq!(body_bytes(guessed).await, superseded_body);

    let short = reset(&app, "0123456789", NEW_PASSWORD).await;
    assert_eq!(
        short.status(),
        StatusCode::UNAUTHORIZED,
        "a malformed token is 401, not 400"
    );
    assert_eq!(body_bytes(short).await, superseded_body);

    let uppercase = reset(&app, &second.to_uppercase(), NEW_PASSWORD).await;
    assert_eq!(
        uppercase.status(),
        StatusCode::UNAUTHORIZED,
        "the alphabet is lowercase hex"
    );

    assert_eq!(
        reset(&app, &second, NEW_PASSWORD).await.status(),
        StatusCode::NO_CONTENT
    );
}

/// AC3: the new password passes registration's validation — fewer than 8
/// characters is `400 validation/password`, and the token stays unused.
#[sqlx::test(migrations = "../../migrations")]
async fn reset_with_short_password_is_bad_request_and_keeps_the_token(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool, mailer.clone());
    let (_, token) = registered_user_with_token(&app, &mailer, "short@b.com").await;

    let resp = reset(&app, &token, "short").await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "validation", "field": "password" })
    );
    assert_eq!(
        reset(&app, &token, NEW_PASSWORD).await.status(),
        StatusCode::NO_CONTENT
    );
}

/// AC3: a body missing `token` is a 400 validation error, like every other
/// malformed auth body.
#[sqlx::test(migrations = "../../migrations")]
async fn reset_with_missing_token_field_is_bad_request(pool: PgPool) {
    let app = build_app(pool);

    let resp = post_json(
        &app,
        "/auth/password/reset",
        json!({ "password": NEW_PASSWORD }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["error"], "validation");
}

// ---------------------------------------------------------------------------
// AC4 — existing sessions end.
// ---------------------------------------------------------------------------

/// T9 → AC4: a JWT issued before the reset is refused on `/auth/me` after it,
/// with no clock tricks (fail-closed inside the same second); a fresh login
/// still succeeds.
#[sqlx::test(migrations = "../../migrations")]
async fn t9_jwt_issued_before_reset_is_refused_after_it(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool, mailer.clone());
    let (_, token) = registered_user_with_token(&app, &mailer, "session@b.com").await;
    let login = post_json(
        &app,
        "/auth/login",
        json!({ "email": "session@b.com", "password": PASSWORD }),
    )
    .await;
    let old_jwt = body_json(login).await["token"]
        .as_str()
        .unwrap()
        .to_string();
    let bearer = format!("Bearer {old_jwt}");
    assert_eq!(
        get_with_auth(&app, "/auth/me", Some(&bearer))
            .await
            .status(),
        StatusCode::OK
    );

    assert_eq!(
        reset(&app, &token, NEW_PASSWORD).await.status(),
        StatusCode::NO_CONTENT
    );

    let resp = get_with_auth(&app, "/auth/me", Some(&bearer)).await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "the pre-reset JWT must be dead"
    );
    assert_eq!(body_json(resp).await, json!({ "error": "unauthorized" }));
    assert_eq!(
        login_status(&app, "session@b.com", NEW_PASSWORD).await,
        StatusCode::OK
    );
}

/// T9b → AC4 (SPEC-0060 §2.5 boundary): a correctly signed JWT whose `iat`
/// equals `password_changed_at` is refused (fails closed), and one issued one
/// second later is accepted. Minted directly so the boundary is exact.
#[sqlx::test(migrations = "../../migrations")]
async fn t9b_iat_equal_to_password_changed_at_fails_closed(pool: PgPool) {
    use jsonwebtoken::{encode, EncodingKey, Header};

    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool.clone(), mailer.clone());
    let (user_id, token) = registered_user_with_token(&app, &mailer, "edge@b.com").await;
    assert_eq!(
        reset(&app, &token, NEW_PASSWORD).await.status(),
        StatusCode::NO_CONTENT
    );
    let changed_at: DateTime<Utc> = sqlx::query_scalar("SELECT password_changed_at FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    let t = changed_at.timestamp();

    let mint = |iat: i64| {
        let claims = json!({ "sub": user_id, "iat": iat, "exp": iat + 3_600 });
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(TEST_SECRET),
        )
        .unwrap()
    };

    let same_second = get_with_auth(&app, "/auth/me", Some(&format!("Bearer {}", mint(t)))).await;
    assert_eq!(
        same_second.status(),
        StatusCode::UNAUTHORIZED,
        "iat == changed_at must fail closed"
    );
    let after = get_with_auth(&app, "/auth/me", Some(&format!("Bearer {}", mint(t + 1)))).await;
    assert_eq!(after.status(), StatusCode::OK, "iat > changed_at must pass");
}

/// AC4 guard: a user who never reset (`password_changed_at IS NULL`) keeps
/// every existing session — the migration is additive.
#[sqlx::test(migrations = "../../migrations")]
async fn sessions_survive_when_the_password_was_never_changed(pool: PgPool) {
    let app = build_app(pool.clone());
    register_user(&app, "steady@b.com", PASSWORD).await;
    let login = post_json(
        &app,
        "/auth/login",
        json!({ "email": "steady@b.com", "password": PASSWORD }),
    )
    .await;
    let jwt = body_json(login).await["token"]
        .as_str()
        .unwrap()
        .to_string();
    let changed_at: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT password_changed_at FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        changed_at.is_none(),
        "a fresh account has no password_changed_at"
    );

    let resp = get_with_auth(&app, "/auth/me", Some(&format!("Bearer {jwt}"))).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// AC5 — rate limits.
// ---------------------------------------------------------------------------

/// T10 → AC5: four requests for one email within the window → four 202s and
/// exactly three emails. The fourth is answered like the others.
#[sqlx::test(migrations = "../../migrations")]
async fn t10_fourth_request_in_the_window_is_accepted_but_not_emailed(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool, mailer.clone());
    register_user(&app, "limit@b.com", PASSWORD).await;

    let mut bodies = Vec::new();
    for _ in 0..4 {
        let resp = forgot(&app, "limit@b.com").await;
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        bodies.push(body_bytes(resp).await);
    }

    assert!(
        bodies.iter().all(|b| *b == bodies[0]),
        "all four bodies must be identical"
    );
    assert!(
        mailer.wait_for(3, SEND_WAIT).await,
        "three emails must be sent"
    );
    assert!(
        !mailer.wait_for(4, NO_SEND_WAIT).await,
        "the fourth must not be sent"
    );
    assert_eq!(mailer.sent().len(), 3);
}

/// AC5: the per-email limit counts unknown addresses too — otherwise the
/// limit itself would reveal which addresses have accounts (§8). An unknown
/// address asked for four times still gets four identical 202s.
#[sqlx::test(migrations = "../../migrations")]
async fn rate_limited_unknown_address_is_answered_like_the_rest(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool.clone(), mailer.clone());

    for _ in 0..4 {
        assert_eq!(
            forgot(&app, "nobody@b.com").await.status(),
            StatusCode::ACCEPTED
        );
    }

    let ledger_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM password_reset_requests")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(ledger_rows, 4, "every request is counted, known or not");
    assert!(!mailer.wait_for(1, NO_SEND_WAIT).await);
}

/// AC5: the per-address limit — eleven requests from one `CF-Connecting-IP`
/// for distinct known addresses → eleven 202s and ten emails.
#[sqlx::test(migrations = "../../migrations")]
async fn eleventh_request_from_one_client_address_is_not_emailed(pool: PgPool) {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool, mailer.clone());
    for i in 0..11 {
        register_user(&app, &format!("ip{i}@b.com"), PASSWORD).await;
    }

    for i in 0..11 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/password/forgot")
                    .header("content-type", "application/json")
                    .header("cf-connecting-ip", "203.0.113.7")
                    .body(Body::from(
                        json!({ "email": format!("ip{i}@b.com") }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::ACCEPTED,
            "request {i} must be 202"
        );
    }

    assert!(
        mailer.wait_for(10, SEND_WAIT).await,
        "ten emails must be sent"
    );
    assert!(
        !mailer.wait_for(11, NO_SEND_WAIT).await,
        "the eleventh must not be sent"
    );
}

// ---------------------------------------------------------------------------
// AC1 + AC9 — adversarial: enumeration by timing.
// ---------------------------------------------------------------------------

/// T11 → AC1, AC9 (adversarial): 20 distinct registered addresses and 20
/// distinct unknown ones, one request each, alternating, so no request takes
/// the rate-limited early return. The medians differ by less than
/// [`TIMING_TOLERANCE`]. See SPEC-0060 OQ-2 if this flakes in CI.
#[sqlx::test(migrations = "../../migrations")]
async fn t11_known_and_unknown_addresses_answer_in_the_same_time(pool: PgPool) {
    const SAMPLES: usize = 20;

    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool, mailer.clone());
    for i in 0..SAMPLES {
        register_user(&app, &format!("known{i}@b.com"), PASSWORD).await;
    }

    let mut known = Vec::with_capacity(SAMPLES);
    let mut unknown = Vec::with_capacity(SAMPLES);
    for i in 0..SAMPLES {
        let started = Instant::now();
        assert_eq!(
            forgot(&app, &format!("known{i}@b.com")).await.status(),
            StatusCode::ACCEPTED
        );
        known.push(started.elapsed());

        let started = Instant::now();
        assert_eq!(
            forgot(&app, &format!("ghost{i}@b.com")).await.status(),
            StatusCode::ACCEPTED
        );
        unknown.push(started.elapsed());
    }
    known.sort();
    unknown.sort();
    let (known_median, unknown_median) = (known[SAMPLES / 2], unknown[SAMPLES / 2]);

    let gap = known_median.abs_diff(unknown_median);
    assert!(
        gap < TIMING_TOLERANCE,
        "median gap {gap:?} (known {known_median:?}, unknown {unknown_median:?}) must be under {TIMING_TOLERANCE:?}"
    );
    assert!(
        mailer.wait_for(SAMPLES, SEND_WAIT).await,
        "every known address gets an email"
    );
    assert_eq!(mailer.sent().len(), SAMPLES, "no unknown address gets one");
}

// ---------------------------------------------------------------------------
// AC6 — a send failure is logged without the address or the token.
// ---------------------------------------------------------------------------

/// T12 → AC6, AC2: with a mailer that always fails (`MailSettings::default()`
/// is the `UnconfiguredMailer`, SPEC-0060 §2.3), the forgot path still answers
/// 202 and logs an error that contains neither the address nor a token. The
/// send task is instrumented with the request span, so the capture sees it.
#[traced_test]
#[sqlx::test(migrations = "../../migrations")]
async fn t12_send_failure_is_logged_without_address_or_token(pool: PgPool) {
    const ADDRESS: &str = "leaky-recognisable@b.com";
    let app = build_app(pool);
    register_user(&app, ADDRESS, PASSWORD).await;

    let resp = forgot(&app, ADDRESS).await;
    assert_eq!(
        resp.status(),
        StatusCode::ACCEPTED,
        "a send failure is not reported"
    );

    // The send runs on a spawned task; yield until its error line lands.
    for _ in 0..1_000 {
        if logs_contain("password reset email not sent") {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        logs_contain("password reset email not sent"),
        "the failure must be logged"
    );

    logs_assert(|lines: &[&str]| {
        for line in lines {
            if line.contains(ADDRESS) {
                return Err(format!("tracing output leaked the address: {line}"));
            }
            let has_token_shaped_run = line
                .as_bytes()
                .windows(64)
                .any(|w| w.iter().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
            if has_token_shaped_run {
                return Err(format!(
                    "tracing output leaked a token-shaped value: {line}"
                ));
            }
        }
        Ok(())
    });
}

/// AC2 (no-plaintext rule): the happy path logs neither the token nor the new
/// password anywhere.
#[traced_test]
#[sqlx::test(migrations = "../../migrations")]
async fn reset_flow_logs_neither_token_nor_password(pool: PgPool) {
    const RECOGNISABLE_PW: &str = "recognisable-plaintext-pw";
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool, mailer.clone());
    let (_, token) = registered_user_with_token(&app, &mailer, "quiet@b.com").await;
    assert_eq!(
        reset(&app, &token, RECOGNISABLE_PW).await.status(),
        StatusCode::NO_CONTENT
    );

    logs_assert(|lines: &[&str]| {
        for line in lines {
            if line.contains(&token) {
                return Err(format!("tracing output leaked the reset token: {line}"));
            }
            if line.contains(RECOGNISABLE_PW) {
                return Err(format!("tracing output leaked the new password: {line}"));
            }
        }
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// Schema (SPEC-0060 §2.1).
// ---------------------------------------------------------------------------

/// T13 → AC2 (schema): deleting a user cascades to its reset-token rows.
#[sqlx::test(migrations = "../../migrations")]
async fn t13_token_rows_cascade_on_user_delete(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool.clone(), mailer.clone());
    let (user_id, _) = registered_user_with_token(&app, &mailer, "gone@b.com").await;
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM password_reset_tokens")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, 1);

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(Uuid::parse_str(&user_id).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM password_reset_tokens")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(after, 0, "token rows must cascade with the user");
}

/// T14 → AC5 (ledger hygiene): after any request, ledger rows older than an
/// hour are gone for every key, not only the requester's.
#[sqlx::test(migrations = "../../migrations")]
async fn t14_ledger_rows_older_than_an_hour_are_purged_on_request(pool: PgPool) {
    let app = build_app(pool.clone());
    sqlx::query(
        "INSERT INTO password_reset_requests (key_hash, requested_at) \
         VALUES (sha256('email:stale@b.com'::bytea), now() - interval '2 hours'), \
                (sha256('ip:198.51.100.1'::bytea), now() - interval '61 minutes'), \
                (sha256('email:recent@b.com'::bytea), now() - interval '59 minutes')",
    )
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(
        forgot(&app, "anyone@b.com").await.status(),
        StatusCode::ACCEPTED
    );

    let stale: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM password_reset_requests \
         WHERE requested_at < now() - interval '1 hour'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stale, 0, "rows older than the window must be purged");
    let kept: i64 = sqlx::query_scalar("SELECT count(*) FROM password_reset_requests")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(kept, 2, "the in-window row and this request's row remain");
}

/// Schema → AC2, AC5: the migration adds `users.password_changed_at`
/// (nullable) and both tables with the spec's columns.
#[sqlx::test(migrations = "../../migrations")]
async fn migration_adds_the_reset_tables_and_marker_column(pool: PgPool) {
    let nullable: String = sqlx::query_scalar(
        "SELECT is_nullable FROM information_schema.columns \
         WHERE table_name = 'users' AND column_name = 'password_changed_at'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        nullable, "YES",
        "password_changed_at must be nullable (additive migration)"
    );

    let mut token_columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name FROM information_schema.columns \
         WHERE table_name = 'password_reset_tokens'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    token_columns.sort();
    assert_eq!(
        token_columns,
        [
            "created_at",
            "expires_at",
            "id",
            "token_hash",
            "used_at",
            "user_id"
        ]
    );

    let mut ledger_columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name FROM information_schema.columns \
         WHERE table_name = 'password_reset_requests'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    ledger_columns.sort();
    assert_eq!(ledger_columns, ["key_hash", "requested_at"]);
}

// ---------------------------------------------------------------------------
// e2e — R-0060 end to end against a booted server (SPEC-0060 §7, QA-owned).
// ---------------------------------------------------------------------------

/// e2e → AC1, AC2, AC3, AC4: T1 → T4 → T5 over a real TCP listener with the
/// recording mailer: ask, open the emailed link's token, set the password,
/// log in with it, and see the old session die.
#[sqlx::test(migrations = "../../migrations")]
async fn e2e_forgot_email_reset_login_over_a_real_server(pool: PgPool) {
    let mailer = Arc::new(RecordingMailer::default());
    let app = build_app_with_mailer(pool, mailer.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let http = reqwest::Client::new();

    let register = http
        .post(format!("{base}/auth/register"))
        .header("content-type", "application/json")
        .body(json!({ "email": "e2e@b.com", "password": PASSWORD }).to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(register.status(), reqwest::StatusCode::CREATED);
    let login = http
        .post(format!("{base}/auth/login"))
        .header("content-type", "application/json")
        .body(json!({ "email": "e2e@b.com", "password": PASSWORD }).to_string())
        .send()
        .await
        .unwrap();
    let old_jwt = serde_json::from_str::<serde_json::Value>(&login.text().await.unwrap()).unwrap()
        ["token"]
        .as_str()
        .unwrap()
        .to_string();

    // T1: ask.
    let forgot = http
        .post(format!("{base}/auth/password/forgot"))
        .header("content-type", "application/json")
        .body(json!({ "email": "e2e@b.com" }).to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(forgot.status(), reqwest::StatusCode::ACCEPTED);
    assert!(
        mailer.wait_for(1, SEND_WAIT).await,
        "the reset email must be sent"
    );

    // T4: the link.
    let message = &mailer.sent()[0];
    assert_eq!(message.to, "e2e@b.com");
    let token = reset_token_in(message);
    assert!(message.text.contains(&format!(
        "{TEST_PUBLIC_WEB_URL}/#/reset-password?token={token}"
    )));

    // T5: set the password; the new one logs in, the old session is dead.
    let reset = http
        .post(format!("{base}/auth/password/reset"))
        .header("content-type", "application/json")
        .body(json!({ "token": token, "password": NEW_PASSWORD }).to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(reset.status(), reqwest::StatusCode::NO_CONTENT);
    let relogin = http
        .post(format!("{base}/auth/login"))
        .header("content-type", "application/json")
        .body(json!({ "email": "e2e@b.com", "password": NEW_PASSWORD }).to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(relogin.status(), reqwest::StatusCode::OK);
    let me = http
        .get(format!("{base}/auth/me"))
        .bearer_auth(old_jwt)
        .send()
        .await
        .unwrap();
    assert_eq!(me.status(), reqwest::StatusCode::UNAUTHORIZED);

    server.abort();
}
