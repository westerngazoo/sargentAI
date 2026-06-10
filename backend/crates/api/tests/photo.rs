//! R-0006 photo-session integration suite — POST/GET/DELETE /photo-sessions and
//! the multipart photo upload / byte download through the object-store seam.
//!
//! Authored by the qa agent during R-0006 step 3 (test planning), BEFORE the
//! photo implementation exists. Pre-implementation red state = the
//! `core::photo` module, the `api::storage` seam, the `AppState.store` field,
//! the `00005_photo_sessions.sql` migration, and the `/photo-sessions` routes
//! are all absent, so this crate fails to COMPILE. Implementation step 5 makes
//! it green.
//!
//! Every test is `#[sqlx::test(migrations = "../../migrations")]` per SPEC-0006
//! §6 (the R-0002..R-0005 harness): sqlx provisions a fresh per-test database,
//! applies the migrations (including the new photo tables once they exist), and
//! hands a connected `PgPool` to the test — trivially isolated. The app is built
//! with `build_app_with_store`, which roots a `LocalObjectStore` in a per-test
//! `TempDir`, so the whole suite runs cloud-free (AC8) and can assert directly
//! on the stored bytes.
//!
//! SAC → test traceability lives in the qa sign-off report; each test below is
//! tagged inline with the SAC/AC branch it verifies.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Test doc comments quote JSON/array literals and content-type strings as prose.
#![allow(clippy::doc_markdown)]
// Small loop indices are cast to compare against JSON `as_i64()` values.
#![allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]

mod common;

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use common::{
    body_bytes, body_json, build_app_with_store, content_type, delete_with_auth, get_with_auth,
    multipart_body, multipart_body_no_file, post_json_with_auth, post_multipart_with_auth,
    register_and_token,
};
use fitai_api::storage::ObjectStore;
use serde_json::json;
use sqlx::{PgPool, Row};

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

/// A tiny but well-formed PNG payload (signature + a single IHDR-shaped chunk).
/// Content validation is by the declared part content-type, not magic-byte
/// sniffing (SPEC-0006 §2.3), but realistic bytes make the round-trip assertions
/// meaningful.
fn png_bytes() -> Vec<u8> {
    let mut v = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    v.extend_from_slice(b"qa-png-payload-distinct-content");
    v
}

/// A tiny JPEG-shaped payload (SOI marker + distinct bytes), declared
/// `image/jpeg` so the upload + download content-type assertions differ from the
/// PNG case.
fn jpeg_bytes() -> Vec<u8> {
    let mut v = vec![0xff, 0xd8, 0xff, 0xe0];
    v.extend_from_slice(b"qa-jpeg-payload-distinct-content");
    v
}

/// COUNT(*) over a photo table — used to assert "writes nothing" / cascade.
async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query(&format!("SELECT COUNT(*) AS n FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
        .get("n")
}

/// The single `storage_key` recorded for a photo id, read straight from the DB
/// (it never crosses the wire) — used to assert the key shape (SAC9) and to
/// probe / mutate the object store out-of-band.
async fn storage_key_of(pool: &PgPool, photo_id: &str) -> String {
    sqlx::query("SELECT storage_key FROM photo_session_photos WHERE id = $1")
        .bind(uuid::Uuid::parse_str(photo_id).unwrap())
        .fetch_one(pool)
        .await
        .unwrap()
        .get("storage_key")
}

/// Create a session for `token` and return its id string.
async fn create_session(app: &axum::Router, token: &str) -> String {
    let resp = post_json_with_auth(app, "/photo-sessions", Some(&bearer(token)), json!({})).await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "session create must be 201"
    );
    body_json(resp).await["id"].as_str().unwrap().to_string()
}

/// Upload `bytes` (declared `content_type`, optional `angle`) to a session,
/// returning the raw response for the caller to assert on.
async fn upload(
    app: &axum::Router,
    token: &str,
    session_id: &str,
    angle: Option<&str>,
    content_type_value: &str,
    bytes: &[u8],
) -> axum::http::Response<axum::body::Body> {
    let (body, header) = multipart_body(angle, content_type_value, bytes);
    post_multipart_with_auth(
        app,
        &format!("/photo-sessions/{session_id}/photos"),
        Some(&bearer(token)),
        body,
        &header,
    )
    .await
}

// ===========================================================================
// SAC1 / AC1: migration applied — two tables, columns, FK cascades.
// ===========================================================================

/// AC1: `photo_sessions` and `photo_session_photos` exist with the expected
/// columns; `photo_session_photos` carries `storage_key` server-side.
#[sqlx::test(migrations = "../../migrations")]
async fn migration_creates_photo_tables_with_expected_columns(pool: PgPool) {
    let columns = |table: &'static str| {
        let pool = pool.clone();
        async move {
            let rows = sqlx::query(
                "SELECT column_name FROM information_schema.columns WHERE table_name = $1",
            )
            .bind(table)
            .fetch_all(&pool)
            .await
            .unwrap();
            let mut names: Vec<String> = rows
                .iter()
                .map(|r| r.get::<String, _>("column_name"))
                .collect();
            names.sort();
            names
        }
    };

    assert_eq!(
        columns("photo_sessions").await,
        vec!["created_at", "id", "performed_on", "updated_at", "user_id"],
        "photo_sessions must have exactly the five expected columns"
    );
    assert_eq!(
        columns("photo_session_photos").await,
        vec![
            "angle",
            "byte_size",
            "content_type",
            "created_at",
            "id",
            "session_id",
            "storage_key",
        ],
        "photo_session_photos must have exactly the seven expected columns"
    );
}

/// AC1: deleting a `users` row cascades to sessions and their photos.
#[sqlx::test(migrations = "../../migrations")]
async fn deleting_user_cascades_to_sessions_and_photos(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool.clone());
    let (user_id, token) = register_and_token(&app, "cascade@b.com", "8charsmin").await;

    let session_id = create_session(&app, &token).await;
    let resp = upload(
        &app,
        &token,
        &session_id,
        Some("front"),
        "image/png",
        &png_bytes(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    assert_eq!(count(&pool, "photo_sessions").await, 1);
    assert_eq!(count(&pool, "photo_session_photos").await, 1);

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&user_id).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        count(&pool, "photo_sessions").await,
        0,
        "sessions must cascade on user delete"
    );
    assert_eq!(
        count(&pool, "photo_session_photos").await,
        0,
        "photos must cascade on user delete"
    );
}

/// AC1: deleting a session row cascades to its photo rows.
#[sqlx::test(migrations = "../../migrations")]
async fn deleting_session_row_cascades_to_photo_rows(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool.clone());
    let (_id, token) = register_and_token(&app, "sescascade@b.com", "8charsmin").await;

    let session_id = create_session(&app, &token).await;
    upload(&app, &token, &session_id, None, "image/png", &png_bytes()).await;
    assert_eq!(count(&pool, "photo_session_photos").await, 1);

    sqlx::query("DELETE FROM photo_sessions WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&session_id).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        count(&pool, "photo_session_photos").await,
        0,
        "photo rows must cascade on session delete"
    );
}

// ===========================================================================
// SAC2 / AC2: POST /photo-sessions — 201 empty session owned by caller, 401.
// ===========================================================================

/// AC2: POST creates an empty session (performed_on = today) owned by the caller.
#[sqlx::test(migrations = "../../migrations")]
async fn create_session_returns_201_empty_and_owned(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool.clone());
    let (user_id, token) = register_and_token(&app, "create@b.com", "8charsmin").await;

    let resp = post_json_with_auth(&app, "/photo-sessions", Some(&bearer(&token)), json!({})).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;

    assert_eq!(
        body["user_id"].as_str().unwrap(),
        user_id,
        "session owned by caller"
    );
    assert!(
        body["id"].as_str().unwrap().parse::<uuid::Uuid>().is_ok(),
        "session id must be a server-generated UUID"
    );
    assert_eq!(
        body["photos"],
        json!([]),
        "a fresh session must have an empty photos array"
    );
    assert_eq!(
        body["performed_on"].as_str().unwrap(),
        Utc::now().date_naive().to_string(),
        "performed_on must default to today"
    );
    assert!(
        body["created_at"]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .is_ok(),
        "created_at must be RFC3339"
    );
    assert!(
        body["updated_at"]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .is_ok(),
        "updated_at must be RFC3339"
    );

    let owned: i64 = sqlx::query("SELECT COUNT(*) AS n FROM photo_sessions WHERE user_id = $1")
        .bind(uuid::Uuid::parse_str(&user_id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("n");
    assert_eq!(
        owned, 1,
        "the session must be persisted owned by the caller"
    );
}

/// AC2: POST with no token -> 401 and writes nothing.
#[sqlx::test(migrations = "../../migrations")]
async fn create_session_without_token_is_unauthorized(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool.clone());

    let resp = post_json_with_auth(&app, "/photo-sessions", None, json!({})).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        count(&pool, "photo_sessions").await,
        0,
        "unauthorized create writes nothing"
    );
}

// ===========================================================================
// SAC3 / AC3: POST /photo-sessions/:id/photos — multipart upload.
// ===========================================================================

/// AC3: a valid PNG upload -> 201 + metadata (NO storage_key in the JSON); the
/// bytes land in the object store under a UUID-shaped, user-namespaced key; the
/// upload→download round trip is byte-identical with the stored content type.
#[sqlx::test(migrations = "../../migrations")]
async fn upload_png_returns_metadata_and_stores_retrievable_bytes(pool: PgPool) {
    let (app, store, _dir) = build_app_with_store(pool.clone());
    let (user_id, token) = register_and_token(&app, "png@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;

    let bytes = png_bytes();
    let resp = upload(
        &app,
        &token,
        &session_id,
        Some("front"),
        "image/png",
        &bytes,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let meta = body_json(resp).await;

    assert!(
        meta["id"].as_str().unwrap().parse::<uuid::Uuid>().is_ok(),
        "photo id must be a server-generated UUID"
    );
    assert_eq!(meta["angle"].as_str().unwrap(), "front");
    assert_eq!(meta["content_type"].as_str().unwrap(), "image/png");
    assert_eq!(meta["byte_size"].as_i64().unwrap(), bytes.len() as i64);
    assert!(meta["created_at"]
        .as_str()
        .unwrap()
        .parse::<DateTime<Utc>>()
        .is_ok());
    // AC3/AC9: the metadata must NEVER leak the storage key.
    assert!(
        meta.get("storage_key").is_none(),
        "the upload response must NOT carry a storage_key"
    );

    let photo_id = meta["id"].as_str().unwrap();
    let key = storage_key_of(&pool, photo_id).await;
    // SAC9: key is `{user}/{session}/{photo}`, all UUIDs, user-namespaced.
    assert_eq!(
        key,
        format!("{user_id}/{session_id}/{photo_id}"),
        "the storage key must be the user-namespaced UUID triple"
    );

    // The exact bytes are retrievable from the store under that key.
    let stored = store.get(&key).await.expect("bytes must be in the store");
    assert_eq!(
        stored.as_ref(),
        bytes.as_slice(),
        "stored bytes must be byte-identical to the upload"
    );
}

/// AC3: a JPEG upload -> 201 with content_type image/jpeg; the download streams
/// the exact bytes back with that content type.
#[sqlx::test(migrations = "../../migrations")]
async fn upload_jpeg_then_download_is_byte_identical(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool.clone());
    let (_id, token) = register_and_token(&app, "jpeg@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;

    let bytes = jpeg_bytes();
    let resp = upload(&app, &token, &session_id, None, "image/jpeg", &bytes).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let meta = body_json(resp).await;
    assert_eq!(meta["content_type"].as_str().unwrap(), "image/jpeg");
    let photo_id = meta["id"].as_str().unwrap().to_string();

    let download = get_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/photos/{photo_id}"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(download.status(), StatusCode::OK);
    assert_eq!(
        content_type(&download).as_deref(),
        Some("image/jpeg"),
        "download must carry the stored content type"
    );
    let downloaded = body_bytes(download).await;
    assert_eq!(
        downloaded, bytes,
        "the download must return the exact uploaded bytes"
    );
}

/// AC3: a non-image content type -> 400 naming "content_type" or "file"; nothing
/// is written and no bytes are stored.
#[sqlx::test(migrations = "../../migrations")]
async fn upload_non_image_content_type_is_rejected(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool.clone());
    let (_id, token) = register_and_token(&app, "notimage@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;

    let resp = upload(
        &app,
        &token,
        &session_id,
        None,
        "application/json",
        b"{\"not\":\"an image\"}",
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    assert_eq!(body["error"].as_str().unwrap(), "validation");
    let field = body["field"].as_str().unwrap();
    assert!(
        field == "content_type" || field == "file",
        "a non-image upload must name field content_type or file, got {field:?}"
    );
    assert_eq!(
        count(&pool, "photo_session_photos").await,
        0,
        "a rejected upload writes no photo row"
    );
}

/// AC3: a multipart body with NO file part -> 400 (field "file").
#[sqlx::test(migrations = "../../migrations")]
async fn upload_missing_file_part_is_rejected(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool.clone());
    let (_id, token) = register_and_token(&app, "nofile@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;

    let (body, header) = multipart_body_no_file(Some("front"));
    let resp = post_multipart_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/photos"),
        Some(&bearer(&token)),
        body,
        &header,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["field"].as_str().unwrap(), "file");
    assert_eq!(count(&pool, "photo_session_photos").await, 0);
}

/// AC3: an oversized buffered body (over MAX_BYTES but within the
/// `DefaultBodyLimit` slack so it reaches the handler) -> 400 (field "file"),
/// writes nothing. The pure transport-limit 413 case (beyond the slack) is
/// noted in the qa report as scoped to the layer config, not driven here.
#[sqlx::test(migrations = "../../migrations")]
async fn upload_oversized_within_slack_is_bad_request(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool.clone());
    let (_id, token) = register_and_token(&app, "toobig@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;

    // MAX_BYTES = 10 MiB; one byte over the cap, well within the +1 MiB slack the
    // route's DefaultBodyLimit allows (SPEC-0006 §2.3), so the handler buffers it
    // and the length check yields 400 rather than a transport-layer 413.
    let oversized = vec![0u8; (10 * 1024 * 1024) + 1];
    let resp = upload(&app, &token, &session_id, None, "image/png", &oversized).await;

    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "a body just over MAX_BYTES (within the slack) must be 400"
    );
    assert_eq!(body_json(resp).await["field"].as_str().unwrap(), "file");
    assert_eq!(
        count(&pool, "photo_session_photos").await,
        0,
        "an oversized upload writes no row"
    );
}

/// AC3: uploading to a missing session -> 404.
#[sqlx::test(migrations = "../../migrations")]
async fn upload_to_missing_session_is_not_found(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool);
    let (_id, token) = register_and_token(&app, "missingses@b.com", "8charsmin").await;

    let unknown = uuid::Uuid::new_v4();
    let resp = upload(
        &app,
        &token,
        &unknown.to_string(),
        None,
        "image/png",
        &png_bytes(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// AC3/AC9: uploading to another user's session -> 404 (never 403); nothing is
/// written and no bytes are stored against the foreign session.
#[sqlx::test(migrations = "../../migrations")]
async fn upload_to_foreign_session_is_not_found(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool.clone());
    let (_id_a, token_a) = register_and_token(&app, "uownerA@b.com", "8charsmin").await;
    let (_id_b, token_b) = register_and_token(&app, "uintruderB@b.com", "8charsmin").await;

    let session_a = create_session(&app, &token_a).await;
    let resp = upload(&app, &token_b, &session_a, None, "image/png", &png_bytes()).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        count(&pool, "photo_session_photos").await,
        0,
        "a foreign upload writes no row"
    );
}

/// AC3: upload with no token -> 401.
#[sqlx::test(migrations = "../../migrations")]
async fn upload_without_token_is_unauthorized(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool);
    let (_id, token) = register_and_token(&app, "uunauth@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;

    let (body, header) = multipart_body(Some("front"), "image/png", &png_bytes());
    let resp = post_multipart_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/photos"),
        None,
        body,
        &header,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// SAC4 / AC4: angle — controlled set, unknown -> 400, absent allowed, multiple
// photos per session.
// ===========================================================================

/// AC4: each controlled angle is accepted and stored.
#[sqlx::test(migrations = "../../migrations")]
async fn upload_accepts_every_controlled_angle(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool);
    let (_id, token) = register_and_token(&app, "angles@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;

    for angle in ["front", "back", "left", "right", "other"] {
        let resp = upload(
            &app,
            &token,
            &session_id,
            Some(angle),
            "image/png",
            &png_bytes(),
        )
        .await;
        assert_eq!(
            resp.status(),
            StatusCode::CREATED,
            "angle {angle:?} must be accepted"
        );
        assert_eq!(body_json(resp).await["angle"].as_str().unwrap(), angle);
    }
}

/// AC4: an unknown angle -> 400 (field "angle"); writes nothing.
#[sqlx::test(migrations = "../../migrations")]
async fn upload_unknown_angle_is_rejected(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool.clone());
    let (_id, token) = register_and_token(&app, "badangle@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;

    let resp = upload(
        &app,
        &token,
        &session_id,
        Some("sideways"),
        "image/png",
        &png_bytes(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await["field"].as_str().unwrap(), "angle");
    assert_eq!(count(&pool, "photo_session_photos").await, 0);
}

/// AC4: a photo with NO angle is accepted and serializes angle as null.
#[sqlx::test(migrations = "../../migrations")]
async fn upload_without_angle_is_accepted(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool);
    let (_id, token) = register_and_token(&app, "noangle@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;

    let resp = upload(&app, &token, &session_id, None, "image/png", &png_bytes()).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    assert!(
        body_json(resp).await["angle"].is_null(),
        "an omitted angle must serialize as null"
    );
}

/// AC4: a single session accepts multiple photos.
#[sqlx::test(migrations = "../../migrations")]
async fn session_accepts_multiple_photos(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool.clone());
    let (_id, token) = register_and_token(&app, "multi@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;

    for (angle, ct, bytes) in [
        (Some("front"), "image/png", png_bytes()),
        (Some("back"), "image/jpeg", jpeg_bytes()),
        (None, "image/png", png_bytes()),
    ] {
        let resp = upload(&app, &token, &session_id, angle, ct, &bytes).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }
    assert_eq!(
        count(&pool, "photo_session_photos").await,
        3,
        "all three photos must be recorded under the one session"
    );

    let got = get_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}"),
        Some(&bearer(&token)),
    )
    .await;
    let body = body_json(got).await;
    assert_eq!(
        body["photos"].as_array().unwrap().len(),
        3,
        "the session must list all three photos"
    );
}

// ===========================================================================
// SAC5 / AC5: GET /photo-sessions list + GET /:id.
// ===========================================================================

/// AC5: GET list with no sessions -> 200 [].
#[sqlx::test(migrations = "../../migrations")]
async fn list_when_empty_returns_empty_array(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool);
    let (_id, token) = register_and_token(&app, "emptylist@b.com", "8charsmin").await;

    let resp = get_with_auth(&app, "/photo-sessions", Some(&bearer(&token))).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(body_json(resp).await, json!([]), "no sessions -> []");
}

/// AC5: GET list -> caller's sessions newest performed_on first, each with its
/// photos as metadata only (no storage_key key anywhere in the JSON).
#[sqlx::test(migrations = "../../migrations")]
async fn list_returns_caller_sessions_newest_first_metadata_only(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool.clone());
    let (_id, token) = register_and_token(&app, "order@b.com", "8charsmin").await;

    // Two sessions; backdate the first so ordering is unambiguous regardless of
    // creation order (both default to today's performed_on otherwise).
    let older = create_session(&app, &token).await;
    sqlx::query("UPDATE photo_sessions SET performed_on = DATE '2026-05-01' WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&older).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let newer = create_session(&app, &token).await;
    sqlx::query("UPDATE photo_sessions SET performed_on = DATE '2026-05-15' WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&newer).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    upload(
        &app,
        &token,
        &newer,
        Some("front"),
        "image/png",
        &png_bytes(),
    )
    .await;

    let resp = get_with_auth(&app, "/photo-sessions", Some(&bearer(&token))).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let sessions = body.as_array().unwrap();
    assert_eq!(sessions.len(), 2);
    assert_eq!(
        sessions[0]["performed_on"].as_str().unwrap(),
        "2026-05-15",
        "newest performed_on must come first"
    );
    assert_eq!(sessions[1]["performed_on"].as_str().unwrap(), "2026-05-01");

    // Photos travel as metadata only; storage_key never appears anywhere.
    let photo = &sessions[0]["photos"][0];
    assert_eq!(photo["content_type"].as_str().unwrap(), "image/png");
    assert!(photo.get("storage_key").is_none());
    assert!(
        !body.to_string().contains("storage_key"),
        "the list JSON must not contain the substring storage_key"
    );
}

/// AC5: GET /:id owned -> 200; missing -> 404; foreign -> 404 (never 403).
#[sqlx::test(migrations = "../../migrations")]
async fn get_one_owned_missing_and_foreign(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool);
    let (_id_a, token_a) = register_and_token(&app, "g1ownerA@b.com", "8charsmin").await;
    let (_id_b, token_b) = register_and_token(&app, "g1intruderB@b.com", "8charsmin").await;

    let session_a = create_session(&app, &token_a).await;

    // Owned -> 200.
    let owned = get_with_auth(
        &app,
        &format!("/photo-sessions/{session_a}"),
        Some(&bearer(&token_a)),
    )
    .await;
    assert_eq!(owned.status(), StatusCode::OK);
    assert_eq!(body_json(owned).await["id"].as_str().unwrap(), session_a);

    // Missing -> 404 with the uniform body.
    let unknown = uuid::Uuid::new_v4();
    let missing = get_with_auth(
        &app,
        &format!("/photo-sessions/{unknown}"),
        Some(&bearer(&token_a)),
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_json(missing).await, json!({ "error": "not_found" }));

    // Foreign -> 404 (indistinguishable from missing; never 403).
    let foreign = get_with_auth(
        &app,
        &format!("/photo-sessions/{session_a}"),
        Some(&bearer(&token_b)),
    )
    .await;
    assert_eq!(foreign.status(), StatusCode::NOT_FOUND);
}

/// AC5: GET list with no token -> 401.
#[sqlx::test(migrations = "../../migrations")]
async fn list_without_token_is_unauthorized(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool);
    let resp = get_with_auth(&app, "/photo-sessions", None).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// AC5: GET /:id with no token -> 401.
#[sqlx::test(migrations = "../../migrations")]
async fn get_one_without_token_is_unauthorized(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool);
    let id = uuid::Uuid::new_v4();
    let resp = get_with_auth(&app, &format!("/photo-sessions/{id}"), None).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// SAC6 / AC6: GET /:id/photos/:photoId — owner download, 404 foreign/missing.
// ===========================================================================

/// AC6: download of a foreign photo -> 404, and NO foreign bytes are served.
#[sqlx::test(migrations = "../../migrations")]
async fn download_foreign_photo_is_not_found_and_serves_no_bytes(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool);
    let (_id_a, token_a) = register_and_token(&app, "downA@b.com", "8charsmin").await;
    let (_id_b, token_b) = register_and_token(&app, "downB@b.com", "8charsmin").await;

    let session_a = create_session(&app, &token_a).await;
    let bytes = jpeg_bytes();
    let up = upload(&app, &token_a, &session_a, None, "image/jpeg", &bytes).await;
    let photo_id = body_json(up).await["id"].as_str().unwrap().to_string();

    let resp = get_with_auth(
        &app,
        &format!("/photo-sessions/{session_a}/photos/{photo_id}"),
        Some(&bearer(&token_b)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let served = body_bytes(resp).await;
    assert_ne!(
        served, bytes,
        "a foreign download must never return the owner's bytes"
    );
}

/// AC6: download of a missing photo id under an owned session -> 404.
#[sqlx::test(migrations = "../../migrations")]
async fn download_missing_photo_is_not_found(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool);
    let (_id, token) = register_and_token(&app, "downmissing@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;

    let unknown = uuid::Uuid::new_v4();
    let resp = get_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/photos/{unknown}"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// AC6: download with no token -> 401.
#[sqlx::test(migrations = "../../migrations")]
async fn download_without_token_is_unauthorized(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool);
    let (_id, token) = register_and_token(&app, "downunauth@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;
    let up = upload(&app, &token, &session_id, None, "image/png", &png_bytes()).await;
    let photo_id = body_json(up).await["id"].as_str().unwrap().to_string();

    let resp = get_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/photos/{photo_id}"),
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// SAC7 / AC7: DELETE photo + DELETE session, with stored bytes removed.
// ===========================================================================

/// AC7: DELETE a photo -> 204 + bytes gone from the store; a second DELETE ->
/// 404; the download then -> 404.
#[sqlx::test(migrations = "../../migrations")]
async fn delete_photo_removes_row_and_bytes_then_404(pool: PgPool) {
    let (app, store, _dir) = build_app_with_store(pool.clone());
    let (_id, token) = register_and_token(&app, "delphoto@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;

    let up = upload(&app, &token, &session_id, None, "image/png", &png_bytes()).await;
    let photo_id = body_json(up).await["id"].as_str().unwrap().to_string();
    let key = storage_key_of(&pool, &photo_id).await;
    assert!(
        store.get(&key).await.is_ok(),
        "bytes must be present before delete"
    );

    let first = delete_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/photos/{photo_id}"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(first.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        count(&pool, "photo_session_photos").await,
        0,
        "the photo row must be gone"
    );
    assert!(
        store.get(&key).await.is_err(),
        "the stored bytes must be gone from the object store"
    );

    // Second delete -> 404.
    let second = delete_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/photos/{photo_id}"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(second.status(), StatusCode::NOT_FOUND);

    // Download after delete -> 404.
    let download = get_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/photos/{photo_id}"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(download.status(), StatusCode::NOT_FOUND);
}

/// AC7/AC9: deleting another user's photo -> 404 (never 403); the photo and its
/// bytes survive.
#[sqlx::test(migrations = "../../migrations")]
async fn delete_foreign_photo_is_not_found_and_untouched(pool: PgPool) {
    let (app, store, _dir) = build_app_with_store(pool.clone());
    let (_id_a, token_a) = register_and_token(&app, "dpownerA@b.com", "8charsmin").await;
    let (_id_b, token_b) = register_and_token(&app, "dpintruderB@b.com", "8charsmin").await;

    let session_a = create_session(&app, &token_a).await;
    let up = upload(&app, &token_a, &session_a, None, "image/png", &png_bytes()).await;
    let photo_id = body_json(up).await["id"].as_str().unwrap().to_string();
    let key = storage_key_of(&pool, &photo_id).await;

    let resp = delete_with_auth(
        &app,
        &format!("/photo-sessions/{session_a}/photos/{photo_id}"),
        Some(&bearer(&token_b)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        count(&pool, "photo_session_photos").await,
        1,
        "B's delete must not touch A's photo row"
    );
    assert!(
        store.get(&key).await.is_ok(),
        "B's delete must not touch A's stored bytes"
    );
}

/// AC7: DELETE a session -> 204 + all photo rows and all their stored bytes
/// gone; a second DELETE -> 404.
#[sqlx::test(migrations = "../../migrations")]
async fn delete_session_removes_all_photos_and_bytes(pool: PgPool) {
    let (app, store, _dir) = build_app_with_store(pool.clone());
    let (_id, token) = register_and_token(&app, "delses@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;

    let mut keys = Vec::new();
    for (angle, ct, bytes) in [
        (Some("front"), "image/png", png_bytes()),
        (Some("back"), "image/jpeg", jpeg_bytes()),
    ] {
        let up = upload(&app, &token, &session_id, angle, ct, &bytes).await;
        let photo_id = body_json(up).await["id"].as_str().unwrap().to_string();
        keys.push(storage_key_of(&pool, &photo_id).await);
    }
    for key in &keys {
        assert!(store.get(key).await.is_ok(), "bytes present before delete");
    }

    let first = delete_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(first.status(), StatusCode::NO_CONTENT);
    assert_eq!(count(&pool, "photo_sessions").await, 0);
    assert_eq!(
        count(&pool, "photo_session_photos").await,
        0,
        "session delete must remove all photo rows"
    );
    for key in &keys {
        assert!(
            store.get(key).await.is_err(),
            "session delete must remove every photo's stored bytes"
        );
    }

    let second = delete_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(second.status(), StatusCode::NOT_FOUND);
}

/// AC7/AC9: deleting another user's session -> 404; it survives intact.
#[sqlx::test(migrations = "../../migrations")]
async fn delete_foreign_session_is_not_found_and_untouched(pool: PgPool) {
    let (app, store, _dir) = build_app_with_store(pool.clone());
    let (_id_a, token_a) = register_and_token(&app, "dsownerA@b.com", "8charsmin").await;
    let (_id_b, token_b) = register_and_token(&app, "dsintruderB@b.com", "8charsmin").await;

    let session_a = create_session(&app, &token_a).await;
    let up = upload(&app, &token_a, &session_a, None, "image/png", &png_bytes()).await;
    let photo_id = body_json(up).await["id"].as_str().unwrap().to_string();
    let key = storage_key_of(&pool, &photo_id).await;

    let resp = delete_with_auth(
        &app,
        &format!("/photo-sessions/{session_a}"),
        Some(&bearer(&token_b)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        count(&pool, "photo_sessions").await,
        1,
        "B's delete must not touch A's session"
    );
    assert!(
        store.get(&key).await.is_ok(),
        "B's delete must not touch A's stored bytes"
    );
}

/// AC7: DELETE photo with no token -> 401; DELETE session with no token -> 401.
#[sqlx::test(migrations = "../../migrations")]
async fn delete_without_token_is_unauthorized(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool);
    let (_id, token) = register_and_token(&app, "delunauth@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;
    let up = upload(&app, &token, &session_id, None, "image/png", &png_bytes()).await;
    let photo_id = body_json(up).await["id"].as_str().unwrap().to_string();

    let photo_del = delete_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/photos/{photo_id}"),
        None,
    )
    .await;
    assert_eq!(photo_del.status(), StatusCode::UNAUTHORIZED);

    let session_del = delete_with_auth(&app, &format!("/photo-sessions/{session_id}"), None).await;
    assert_eq!(session_del.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// SAC9 / AC9: cross-user isolation — A's list never returns B's sessions.
// ===========================================================================

/// AC9: two users each see only their own sessions in GET /photo-sessions.
#[sqlx::test(migrations = "../../migrations")]
async fn list_is_isolated_per_user(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool);
    let (id_a, token_a) = register_and_token(&app, "isoA@b.com", "8charsmin").await;
    let (id_b, token_b) = register_and_token(&app, "isoB@b.com", "8charsmin").await;
    assert_ne!(id_a, id_b);

    create_session(&app, &token_b).await;
    create_session(&app, &token_b).await;
    create_session(&app, &token_a).await;

    let list_a = get_with_auth(&app, "/photo-sessions", Some(&bearer(&token_a))).await;
    let a_arr = body_json(list_a).await;
    let a_arr = a_arr.as_array().unwrap();
    assert_eq!(a_arr.len(), 1, "A must see only its own session");
    assert_eq!(a_arr[0]["user_id"].as_str().unwrap(), id_a);

    let list_b = get_with_auth(&app, "/photo-sessions", Some(&bearer(&token_b))).await;
    let b_arr = body_json(list_b).await;
    let b_arr = b_arr.as_array().unwrap();
    assert_eq!(b_arr.len(), 2, "B must see only its own two sessions");
    for s in b_arr {
        assert_eq!(s["user_id"].as_str().unwrap(), id_b, "no cross-user leak");
    }
}

// ===========================================================================
// SAC10 / AC10: a download whose bytes were removed from the store out-of-band
// -> 500 with the opaque {"error":"internal"} body (no "bytes missing" leak).
// ===========================================================================

/// AC10: if the object disappears from the store under a still-recorded row, the
/// download surfaces an opaque 500 — never a leaky message, never a 404 (the row
/// exists and is owned). This pins the store-miss → `Internal` mapping
/// (SPEC-0006 §2.4) without needing a fault-injecting store: we delete the bytes
/// directly through the same `LocalObjectStore` the app holds.
#[sqlx::test(migrations = "../../migrations")]
async fn download_with_bytes_missing_from_store_is_opaque_500(pool: PgPool) {
    let (app, store, _dir) = build_app_with_store(pool.clone());
    let (_id, token) = register_and_token(&app, "storemiss@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;

    let up = upload(&app, &token, &session_id, None, "image/png", &png_bytes()).await;
    let photo_id = body_json(up).await["id"].as_str().unwrap().to_string();
    let key = storage_key_of(&pool, &photo_id).await;

    // Out-of-band: remove the bytes but leave the metadata row in place.
    store.delete(&key).await.expect("out-of-band delete");

    let resp = get_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/photos/{photo_id}"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "a store-miss on a recorded photo must be 500, not 404"
    );
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "internal" }),
        "the body must be the opaque internal error — no bytes-missing leak"
    );
}

// ===========================================================================
// SAC9 (download negative) — a cross-user download attempt that exists but is
// foreign must never serve bytes; re-asserted at the download path explicitly.
// ===========================================================================

/// AC9: the owner downloads their own bytes successfully (positive control for
/// the foreign-404 case above), confirming the same path serves bytes only to
/// the owner.
#[sqlx::test(migrations = "../../migrations")]
async fn owner_download_serves_their_own_bytes(pool: PgPool) {
    let (app, _store, _dir) = build_app_with_store(pool);
    let (_id, token) = register_and_token(&app, "ownerdl@b.com", "8charsmin").await;
    let session_id = create_session(&app, &token).await;

    let bytes = png_bytes();
    let up = upload(
        &app,
        &token,
        &session_id,
        Some("front"),
        "image/png",
        &bytes,
    )
    .await;
    let photo_id = body_json(up).await["id"].as_str().unwrap().to_string();

    let resp = get_with_auth(
        &app,
        &format!("/photo-sessions/{session_id}/photos/{photo_id}"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(content_type(&resp).as_deref(), Some("image/png"));
    assert_eq!(
        body_bytes(resp).await,
        bytes,
        "the owner must receive their exact bytes"
    );
}
