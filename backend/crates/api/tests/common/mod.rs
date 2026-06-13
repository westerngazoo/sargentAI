//! Shared test harness for the R-0002 auth integration suite.
//!
//! Authored by the qa agent during R-0002 step 3. These helpers build an
//! `AppState` around the per-test `PgPool` that `#[sqlx::test]` hands in, with
//! a known `JWT_SECRET` and a caller-chosen token TTL (so a test can mint an
//! already-expired token by passing `Duration::ZERO`).

#![allow(
    dead_code,
    unreachable_pub,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::{sync::Arc, time::Duration};

use axum::{
    body::Body,
    http::{Request, Response},
    Router,
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tempfile::TempDir;
use tower::ServiceExt;

use fitai_api::{
    app,
    storage::{LocalObjectStore, ObjectStore},
    AppState,
};

/// Stable secret the whole suite signs/decodes with. SAC4 asserts that a
/// *different* secret fails signature verification.
pub const TEST_SECRET: &[u8] = b"qa-test-secret-r0002";

/// 24h, the production TTL — used by every test except the expiry case.
pub const TTL_24H: Duration = Duration::from_hours(24);

/// Build an `AppState` over the supplied pool with the canonical test secret, a
/// chosen TTL, and a `LocalObjectStore` rooted in a fresh `TempDir`.
///
/// R-0006 (SPEC-0006 §3.4) added `store: Arc<dyn ObjectStore>` to `AppState`;
/// every test app now constructs a per-test local object store so the photo
/// upload/download/delete handlers have somewhere to put bytes — and so the
/// whole suite runs cloud-free (AC8). The `TempDir` is returned alongside the
/// router so the caller can keep it alive for the test's duration; it is
/// removed from disk when dropped.
fn state_with_ttl(pool: PgPool, ttl: Duration) -> (AppState, Arc<LocalObjectStore>, TempDir) {
    let dir = tempfile::tempdir().expect("a temp dir for the object store must be creatable");
    let store = Arc::new(LocalObjectStore::new(dir.path()));
    let state = AppState {
        pool,
        jwt_secret: Arc::from(TEST_SECRET.to_vec().into_boxed_slice()),
        jwt_ttl: ttl,
        store: store.clone(),
    };
    (state, store, dir)
}

/// Build a router over the supplied pool with the canonical test secret and a
/// chosen TTL. The per-test object-store `TempDir` is leaked (via
/// [`TempDir::keep`]) so the directory outlives this function for suites that do
/// not need to inspect the store handle.
pub fn app_with_ttl(pool: PgPool, ttl: Duration) -> Router {
    let (state, _store, dir) = state_with_ttl(pool, ttl);
    // Persist the directory for the life of the test; suites using this helper
    // never assert on stored bytes, so leaking the path is acceptable here.
    let _ = dir.keep();
    app(state)
}

/// Build a router with the production 24h TTL (object-store `TempDir` leaked).
pub fn build_app(pool: PgPool) -> Router {
    app_with_ttl(pool, TTL_24H)
}

/// Build a router together with a handle to its backing `LocalObjectStore` and
/// the owning `TempDir`. The photo suite uses this to assert that uploaded bytes
/// land in the store and that deletes remove them. The caller MUST hold the
/// returned `TempDir` for the test's duration — dropping it deletes the backing
/// directory.
pub fn build_app_with_store(pool: PgPool) -> (Router, Arc<LocalObjectStore>, TempDir) {
    let (state, store, dir) = state_with_ttl(pool, TTL_24H);
    (app(state), store, dir)
}

/// Build a router over `pool` wired to a caller-supplied object store.
///
/// The R-0006 compensation tests pass a fault-injecting stub here to drive the
/// upload handler's "bytes-first, row-second, compensate-on-insert-failure"
/// branch (SPEC-0006 §2.3, AC10) — failure paths the real `LocalObjectStore`
/// cannot reach on demand. Everything else (secret, TTL) matches the production
/// 24h app the rest of the suite builds.
pub fn build_app_with_object_store(pool: PgPool, store: Arc<dyn ObjectStore>) -> Router {
    let state = AppState {
        pool,
        jwt_secret: Arc::from(TEST_SECRET.to_vec().into_boxed_slice()),
        jwt_ttl: TTL_24H,
        store,
    };
    app(state)
}

/// POST a JSON body to `path` and return the raw response.
pub async fn post_json(app: &Router, path: &str, body: Value) -> Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// GET `path` with an optional raw `Authorization` header value.
pub async fn get_with_auth(app: &Router, path: &str, auth: Option<&str>) -> Response<Body> {
    let mut builder = Request::builder().method("GET").uri(path);
    if let Some(value) = auth {
        builder = builder.header("authorization", value);
    }
    app.clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// POST a JSON body to `path` with an optional raw `Authorization` header value.
/// Mirrors `put_json_with_auth`; used by the R-0004 `/workouts` create tests
/// (the existing `post_json` is auth-free, for `/auth/register` and `/login`).
pub async fn post_json_with_auth(
    app: &Router,
    path: &str,
    auth: Option<&str>,
    body: Value,
) -> Response<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json");
    if let Some(value) = auth {
        builder = builder.header("authorization", value);
    }
    app.clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

/// DELETE `path` with an optional raw `Authorization` header value.
pub async fn delete_with_auth(app: &Router, path: &str, auth: Option<&str>) -> Response<Body> {
    let mut builder = Request::builder().method("DELETE").uri(path);
    if let Some(value) = auth {
        builder = builder.header("authorization", value);
    }
    app.clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// PUT a JSON body to `path` with an optional raw `Authorization` header value.
pub async fn put_json_with_auth(
    app: &Router,
    path: &str,
    auth: Option<&str>,
    body: Value,
) -> Response<Body> {
    let mut builder = Request::builder()
        .method("PUT")
        .uri(path)
        .header("content-type", "application/json");
    if let Some(value) = auth {
        builder = builder.header("authorization", value);
    }
    app.clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

/// Drain a response body into raw bytes.
pub async fn body_bytes(resp: Response<Body>) -> Vec<u8> {
    resp.into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec()
}

/// Drain a response body and parse it as JSON.
pub async fn body_json(resp: Response<Body>) -> Value {
    let bytes = body_bytes(resp).await;
    serde_json::from_slice(&bytes).expect("response body must be valid JSON")
}

/// The raw value of a response's `Content-Type` header, if present. The photo
/// download path returns image bytes with the stored content type rather than
/// JSON (SPEC-0006 §2.6), so the suite asserts on this directly.
pub fn content_type(resp: &Response<Body>) -> Option<String> {
    resp.headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

/// A boundary that will not collide with the small binary payloads the photo
/// tests use (PNG/JPEG magic bytes never contain this ASCII run).
const MULTIPART_BOUNDARY: &str = "----fitaiQaBoundary7M2zX";

/// Encode a `multipart/form-data` body with an optional `angle` text part and a
/// single `file` part carrying `content_type` + `bytes`. Returns the encoded
/// body and the matching `Content-Type: multipart/form-data; boundary=...`
/// header value, ready to drive through [`post_multipart_with_auth`].
///
/// This mirrors what a Flutter/HTTP client sends to
/// `POST /photo-sessions/:id/photos` (SPEC-0006 §2.3): a tiny hand-rolled
/// encoder keeps the wire bytes exact and inspectable, so the size/content-type
/// assertions are unambiguous.
pub fn multipart_body(
    angle: Option<&str>,
    file_content_type: &str,
    bytes: &[u8],
) -> (Vec<u8>, String) {
    let dashes = format!("--{MULTIPART_BOUNDARY}");
    let mut body: Vec<u8> = Vec::new();

    if let Some(angle) = angle {
        body.extend_from_slice(format!("{dashes}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"angle\"\r\n\r\n");
        body.extend_from_slice(angle.as_bytes());
        body.extend_from_slice(b"\r\n");
    }

    body.extend_from_slice(format!("{dashes}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"photo.bin\"\r\n",
    );
    body.extend_from_slice(format!("Content-Type: {file_content_type}\r\n\r\n").as_bytes());
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(format!("{dashes}--\r\n").as_bytes());

    let header = format!("multipart/form-data; boundary={MULTIPART_BOUNDARY}");
    (body, header)
}

/// A `multipart/form-data` body that has NO `file` part at all (only an `angle`
/// text part) — exercises the AC3 "missing file → 400" branch.
pub fn multipart_body_no_file(angle: Option<&str>) -> (Vec<u8>, String) {
    let dashes = format!("--{MULTIPART_BOUNDARY}");
    let mut body: Vec<u8> = Vec::new();
    if let Some(angle) = angle {
        body.extend_from_slice(format!("{dashes}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"angle\"\r\n\r\n");
        body.extend_from_slice(angle.as_bytes());
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("{dashes}--\r\n").as_bytes());
    let header = format!("multipart/form-data; boundary={MULTIPART_BOUNDARY}");
    (body, header)
}

/// POST a pre-encoded `multipart/form-data` body to `path` with the matching
/// content-type header and an optional raw `Authorization` header value.
pub async fn post_multipart_with_auth(
    app: &Router,
    path: &str,
    auth: Option<&str>,
    body: Vec<u8>,
    content_type_header: &str,
) -> Response<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", content_type_header);
    if let Some(value) = auth {
        builder = builder.header("authorization", value);
    }
    app.clone()
        .oneshot(builder.body(Body::from(body)).unwrap())
        .await
        .unwrap()
}

/// Register a user and return its `user_id` string. Convenience for tests that
/// need a seeded account before exercising login / `/auth/me`.
pub async fn register_user(app: &Router, email: &str, password: &str) -> String {
    let resp = post_json(
        app,
        "/auth/register",
        serde_json::json!({ "email": email, "password": password }),
    )
    .await;
    assert_eq!(
        resp.status(),
        axum::http::StatusCode::CREATED,
        "seed register expected 201"
    );
    body_json(resp).await["user_id"]
        .as_str()
        .expect("register response must carry a string user_id")
        .to_string()
}

/// Register a user and log in, returning `(user_id, bearer_token)`. Convenience
/// for tests that need an authenticated caller — including the cross-user
/// isolation case, which mints two distinct callers.
pub async fn register_and_token(app: &Router, email: &str, password: &str) -> (String, String) {
    let user_id = register_user(app, email, password).await;
    let login = post_json(
        app,
        "/auth/login",
        serde_json::json!({ "email": email, "password": password }),
    )
    .await;
    assert_eq!(
        login.status(),
        axum::http::StatusCode::OK,
        "seed login expected 200"
    );
    let token = body_json(login).await["token"]
        .as_str()
        .expect("login response must carry a string token")
        .to_string();
    (user_id, token)
}
