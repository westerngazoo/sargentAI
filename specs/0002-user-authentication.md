# SPEC-0002 — User authentication

- **Status:** Draft
- **Realizes:** R-0002
- **Author:** Claude (main session), with owner
- **Created:** 2026-05-29
- **Depends on:** SPEC-0001 (Implemented) — extends the same workspace and CI
- **Module(s):** `backend/crates/core` (new), `backend/crates/api/{auth,db,error}` (new), `backend/migrations/` (new), `backend/docker-compose.yml` (new), `.github/workflows/ci.yml` (extended)

## 1. Motivation

Realizes [R-0002](../requirements/0002-user-authentication.md): users can
register, log in, and present a bearer token that an axum extractor validates.
A `users` row in Postgres backs every account; argon2id hashes every password;
HS256 JWTs (24h) authenticate every protected request.

R-0002 is the spec where this project gains a **database**, a **typed error
hierarchy**, an **auth primitive**, and its **first domain type** — every later
spec rests on these. R-0002's AC1–AC8 map 1:1 to §6.

## 2. Design

### 2.1 Repository layout (additions)

```
fitAI/
├── .github/workflows/ci.yml    # extended: Postgres service + DATABASE_URL env
├── backend/
│   ├── docker-compose.yml      # NEW — local Postgres 16
│   ├── .env.example            # NEW — DATABASE_URL + JWT_SECRET docs
│   ├── migrations/
│   │   └── 00001_users.sql     # NEW — users table
│   ├── scripts/dev/
│   │   └── db.sh               # NEW — up|down|reset|migrate helper
│   └── crates/
│       ├── core/               # NEW (SPEC-0001 §2.2 trigger fired here)
│       │   ├── Cargo.toml
│       │   └── src/
│       │       ├── lib.rs
│       │       └── user.rs     # User, UserId(Uuid), Email newtype
│       └── api/
│           ├── Cargo.toml      # adds sqlx, argon2, jsonwebtoken, …, depends on fitai-core
│           └── src/
│               ├── lib.rs      # AppState; pub fn app(state) -> Router
│               ├── main.rs     # build pool + state, run migrations, serve
│               ├── error.rs    # NEW — ApiError + IntoResponse
│               ├── db.rs       # NEW — PgPool builder, UserRow ↔ User mapping
│               ├── health.rs   # unchanged
│               └── auth/       # NEW
│                   ├── mod.rs       # routes() -> Router<AppState>
│                   ├── handlers.rs  # register, login, me
│                   ├── password.rs  # argon2id hash + verify
│                   ├── token.rs     # JWT encode/decode + Claims
│                   └── extractor.rs # AuthenticatedUser : FromRequestParts
```

### 2.2 `crates/core/` is introduced now

SPEC-0001 §2.2 named R-0003 as the trigger for splitting `crates/core/` from
`crates/api/`. The real trigger is "first domain type", and R-0002's `User`
is the first domain type. SPEC-0001 §7 carries the 2026-05-29 addendum;
SPEC-0001 §2.2 carries the inline correction. R-0003 will extend `core::User`
rather than extracting it.

**`core` stays pure.** No `sqlx`, no axum, no http types. It carries only the
domain model — `User`, `UserId(Uuid)`, `Email(String)` newtype with a validating
constructor — and the units that surround it (`Result`, error types tied to
domain invariants like `EmailParseError`). Persistence lives in `api::db`.

This keeps `password_hash` out of `core::User`: that field is a *persistence
detail* (how we authenticate), not a *domain fact* (the user has a password,
yes, but `core` doesn't need to know its representation). `api::db::UserRow`
carries `password_hash`; the `From<UserRow>` impl strips it when producing
`core::User`. Smaller code-paths see the hash → smaller AC6 (no-plaintext-in-
logs) audit surface.

### 2.3 Application state, error type, extractor

```rust
// crates/api/src/lib.rs
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: Arc<[u8]>,     // bytes, never logged
    pub jwt_ttl: Duration,         // injected for testability (24h prod; tunable in tests)
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .merge(health::router())
        .merge(auth::routes())
        .with_state(state)
}
```

`ApiError` (in `crates/api/src/error.rs`) is a `thiserror` enum implementing
`IntoResponse`:

| Variant | Status | Body |
|---------|--------|------|
| `Database(sqlx::Error)` | 500 | `{"error":"internal"}` (sqlx error logged at error-level; user-visible body is generic) |
| `AlreadyExists` | 409 | `{"error":"already_exists"}` |
| `Validation { field: &'static str }` | 400 | `{"error":"validation","field":<f>}` |
| `Unauthorized` | 401 | `{"error":"unauthorized"}` (*identical for all auth failures — enumeration-safe*) |
| `Internal(eyre::Report)` | 500 | `{"error":"internal"}` |

`AuthenticatedUser` (in `crates/api/src/auth/extractor.rs`) implements
`FromRequestParts<AppState>`. The pipeline is:

1. Read `Authorization` header → `Err → ApiError::Unauthorized`.
2. Strip `Bearer ` prefix → on mismatch, `Unauthorized`.
3. `auth::token::decode(token, &state.jwt_secret)` → on any error, `Unauthorized`.
4. Parse `claims.sub` as `Uuid` → on failure, `Unauthorized`.
5. `db::find_user_by_id(&state.pool, user_id)` → `None` ⇒ `Unauthorized`; `Some(user)` ⇒ `Ok(AuthenticatedUser { user_id })`.

Every failure path returns the **same** `ApiError::Unauthorized` — no
distinguishing body or header. AC5 verifies all five branches.

### 2.4 Auth flow details

- **Register.** Validate `RegisterRequest { email, password }` with `validator`'s `#[validate(email)]` + `length(min = 8)` on password. Hash the password (`auth::password::hash`). `INSERT INTO users (id, email, password_hash) VALUES (gen_random_uuid(), $1, $2) RETURNING id` — on `unique_violation` map to `ApiError::AlreadyExists`. Return `201 { user_id }`.
- **Login.** Look up by email. If `None`, hash the supplied password anyway (constant-time defence against email enumeration timing leaks) then return `Unauthorized`. If `Some(row)`, `auth::password::verify` against `row.password_hash`. On match, `auth::token::encode(claims)` and return `200 { token, user_id, expires_at }`. On mismatch, return `Unauthorized`.
- **Me.** Extractor → handler returns `Json({ user_id })`. Eight lines including imports.

### 2.5 Database lifecycle

- **Migrations:** `sqlx::migrate!("../../migrations").run(&pool).await?` called early in `main.rs`, before `axum::serve`. Atomic per `sqlx` (advisory-locked + tx-wrapped).
- **Pool:** `PgPoolOptions::new().max_connections(8).acquire_timeout(Duration::from_secs(3)).connect(&db_url).await?`. `DATABASE_URL` env. Pool is `Clone`-able and goes into `AppState`.
- **Test isolation:** `#[sqlx::test(migrations = "../../migrations")]` on every DB-touching test. sqlx creates a fresh per-test DB, applies migrations, hands a connected pool to the test. Slower than transaction rollback (~1.5 s/test) but trivially isolated; for ~12 tests we're well under 30 s total wall time.

### 2.6 CI Postgres service

The `rust` job in `.github/workflows/ci.yml` gains a `services.postgres`
declaration (Postgres 16, password `postgres`, port 5432) and a `DATABASE_URL`
env scoped to the job. No other job (`mobile`, `docker`) changes.

### 2.7 Local developer flow

`backend/docker-compose.yml` defines a single service `postgres` (Postgres 16,
named volume, healthcheck, port `5432`). `backend/scripts/dev/db.sh` wraps
the common operations:

| Command | What it does |
|---------|--------------|
| `db.sh up` | `docker compose up -d postgres` + wait for healthy + `sqlx migrate run` |
| `db.sh down` | `docker compose down` (volume preserved) |
| `db.sh reset` | `docker compose down -v` (drops volume) + `up` |
| `db.sh migrate` | `sqlx migrate run` only |

`backend/.env.example` documents the two env vars devs need: `DATABASE_URL`
and `JWT_SECRET`. The real `.env` is gitignored (already covered).

### 2.8 New dependencies

Added to `backend/Cargo.toml` `[workspace.dependencies]`:

| Crate | Version | Why |
|-------|---------|-----|
| `sqlx` | `0.8` | Postgres client + migrations; features `runtime-tokio`, `postgres`, `macros`, `migrate`, `uuid`, `chrono` |
| `argon2` | `0.5` | Password hashing (argon2id, modern OWASP default) |
| `jsonwebtoken` | `9` | JWT encode / decode (HS256 chosen for R-0002) |
| `uuid` | `1` | `UserId` newtype; features `v4`, `serde` |
| `chrono` | `0.4` | timestamps; features `clock`, `serde`; `default-features = false` to drop the `oldtime` ballast |
| `serde` | `1` | DTO derives; features `derive` |
| `serde_json` | `1` | response bodies |
| `thiserror` | `2` | `ApiError` derive |
| `validator` | `0.20` | `#[validate(email)]` + `length(min)` |
| `eyre` | `0.6` | `ApiError::Internal(eyre::Report)` |
| `tracing-test` | `0.2` (dev) | log-capture assertions for AC6 (no plaintext passwords in tracing output) |

Exact versions pinned at implementation time per the same §2.8 convention as
SPEC-0001 (run `cargo add` against the host's current stables; record in the
changelog).

## 3. Code outline

The files below are the agreed implementation shape per `CLAUDE.md` §4.4.
Tests are NOT included here — they are authored by the `qa` agent during
step 3, scoped to R-0002, against the AC list in §6.

### 3.1 `backend/migrations/00001_users.sql`

```sql
-- R-0002 / SPEC-0002 — users table.
-- Holds exactly what authentication needs; profile fields live elsewhere
-- (R-0003 adds them to the same table or to a sibling, decided in SPEC-0003).

CREATE EXTENSION IF NOT EXISTS "pgcrypto";  -- for gen_random_uuid()

CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX users_email_idx ON users (email);  -- redundant with UNIQUE constraint? UNIQUE creates an index, so this is dropped — note kept for review.
```

> The trailing `CREATE INDEX` is redundant with the `UNIQUE` constraint (which
> creates an index implicitly). It will be removed before commit; left here so
> the architect review can confirm. *Decision recorded in §7.*

### 3.2 `backend/Cargo.toml` — `[workspace.dependencies]` additions

```toml
# (existing deps unchanged)

sqlx = { version = "0.8", default-features = false, features = ["runtime-tokio", "postgres", "macros", "migrate", "uuid", "chrono"] }
argon2 = "0.5"
jsonwebtoken = "9"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", default-features = false, features = ["clock", "serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
validator = { version = "0.20", features = ["derive"] }
eyre = "0.6"

# also add the new member:
# [workspace]
# members = ["crates/api", "crates/core"]
```

The `[workspace]` `members` array gains `"crates/core"`.

### 3.3 `backend/crates/core/Cargo.toml`

```toml
[package]
name = "fitai-core"
version = "0.1.0"
edition.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
uuid.workspace = true
chrono.workspace = true
serde.workspace = true
thiserror.workspace = true
```

No sqlx, no axum, no http — `core` is pure domain.

### 3.4 `backend/crates/core/src/lib.rs`

```rust
//! fitAI domain types. Pure: no DB, no HTTP, no I/O.
//!
//! Persistence and presentation live in the `fitai-api` crate.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod user;

pub use user::{Email, EmailParseError, User, UserId};
```

### 3.5 `backend/crates/core/src/user.rs`

```rust
//! `User` and its identifier / email value-object.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// User identifier — newtype around `Uuid` so it can't be mixed with other
/// id types in handler signatures.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(pub Uuid);

impl UserId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Email newtype with a validating constructor. The format is checked by
/// the `validator` crate at the handler boundary; this type guarantees
/// "well-formed at construction" so downstream code can rely on the
/// invariant without re-validating.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Email(String);

#[derive(Debug, Error)]
#[error("invalid email format")]
pub struct EmailParseError;

impl Email {
    /// Construct from a `&str`. Returns `EmailParseError` on a malformed
    /// input. Trusts only basic shape: presence of `@` with non-empty
    /// local + domain parts. Stricter checking is the `validator` crate's
    /// job at the handler boundary.
    pub fn parse(raw: &str) -> Result<Self, EmailParseError> {
        let trimmed = raw.trim();
        let (local, domain) = trimmed.split_once('@').ok_or(EmailParseError)?;
        if local.is_empty() || domain.is_empty() || !domain.contains('.') {
            return Err(EmailParseError);
        }
        Ok(Self(trimmed.to_ascii_lowercase()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Domain `User`. Note: no `password_hash` field — that's a persistence
/// detail kept in `fitai_api::db::UserRow`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: Email,
    pub created_at: DateTime<Utc>,
}
```

### 3.6 `backend/crates/api/Cargo.toml` — additions

```toml
[dependencies]
# (existing — axum, tokio, tracing, tracing-subscriber)
fitai-core = { path = "../core" }
sqlx.workspace = true
argon2 = { workspace = true }
jsonwebtoken.workspace = true
uuid.workspace = true
chrono.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
validator.workspace = true
eyre.workspace = true

[dev-dependencies]
# (existing — tower, http-body-util, reqwest)
tracing-test = "0.2"
```

### 3.7 `backend/crates/api/src/lib.rs`

```rust
//! fitai-api library entry. Hosts the `AppState`, the router builder, and
//! re-exports for tests / integration code.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod auth;
pub mod db;
pub mod error;
mod health;

use std::{sync::Arc, time::Duration};

use axum::Router;
use sqlx::PgPool;

/// Application state shared across handlers via `Router::with_state`.
///
/// `Clone` is cheap: `PgPool` is `Arc`-internal, `jwt_secret` is `Arc<[u8]>`,
/// `Duration` is `Copy`.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: Arc<[u8]>,
    pub jwt_ttl: Duration,
}

/// Build the application router with all routes mounted.
pub fn app(state: AppState) -> Router {
    Router::new()
        .merge(health::router())
        .merge(auth::routes())
        .with_state(state)
}
```

### 3.8 `backend/crates/api/src/main.rs`

```rust
//! fitai-api binary: load config, build pool, run migrations, serve.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use sqlx::postgres::PgPoolOptions;
use tokio::signal::ctrl_c;

use fitai_api::{app, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let db_url = std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL must be set")?;
    let jwt_secret = std::env::var("JWT_SECRET")
        .map_err(|_| "JWT_SECRET must be set")?;

    let pool = PgPoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_url)
        .await?;

    sqlx::migrate!("../../migrations").run(&pool).await?;
    tracing::info!("migrations up to date");

    let state = AppState {
        pool,
        jwt_secret: Arc::from(jwt_secret.into_bytes().into_boxed_slice()),
        jwt_ttl: Duration::from_secs(60 * 60 * 24),  // 24 h
    };

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "fitai-api listening");

    let shutdown = build_shutdown()?;

    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown)
        .await?;

    Ok(())
}

// build_shutdown, log_ctrl_c_error, init_tracing — unchanged from SPEC-0001 §3.6.
// (Reuse verbatim; not re-listed here.)
```

### 3.9 `backend/crates/api/src/error.rs`

```rust
//! Typed application error → HTTP response.
//!
//! Every variant maps to exactly one status code and one stable error body
//! shape. `Unauthorized` is enumeration-safe: identical body across all
//! auth-failure causes (missing header, malformed token, expired, bad sig,
//! unknown sub).

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("database error")]
    Database(#[from] sqlx::Error),

    #[error("already exists")]
    AlreadyExists,

    #[error("validation error in field `{field}`")]
    Validation { field: &'static str },

    #[error("unauthorized")]
    Unauthorized,

    #[error("internal error")]
    Internal(#[from] eyre::Report),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            ApiError::Database(e) => {
                // Postgres unique-violation surfaces here when callers
                // didn't pre-check; map it to AlreadyExists.
                if let sqlx::Error::Database(db_err) = e {
                    if db_err.is_unique_violation() {
                        return ApiError::AlreadyExists.into_response();
                    }
                }
                tracing::error!(error = %e, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": "internal"}))
            }
            ApiError::AlreadyExists => {
                (StatusCode::CONFLICT, json!({"error": "already_exists"}))
            }
            ApiError::Validation { field } => (
                StatusCode::BAD_REQUEST,
                json!({"error": "validation", "field": field}),
            ),
            ApiError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, json!({"error": "unauthorized"}))
            }
            ApiError::Internal(e) => {
                tracing::error!(error = %e, "internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": "internal"}))
            }
        };
        (status, Json(body)).into_response()
    }
}
```

### 3.10 `backend/crates/api/src/db.rs`

```rust
//! Postgres-side types and queries. Maps row shapes to `fitai_core` types
//! at the seam so callers never see `password_hash`.

use chrono::{DateTime, Utc};
use sqlx::{prelude::FromRow, PgPool};
use uuid::Uuid;

use fitai_core::{Email, User, UserId};

use crate::error::{ApiError, ApiResult};

#[derive(Debug, FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,  // *never* leaves this module by accident
    pub created_at: DateTime<Utc>,
}

impl UserRow {
    pub fn into_user(self) -> User {
        // Email::parse cannot fail on data we wrote — DB CHECK and
        // application-side validation already enforced format. Falling
        // back via `expect` would violate the lints; use a fresh Email
        // constructed bypass: we know the invariant holds because we
        // wrote it.
        User {
            id: UserId(self.id),
            email: Email::parse(&self.email).unwrap_or_else(|_| {
                // unreachable in practice; if it happens, we've corrupted
                // the DB and the rest of the system is in trouble too.
                // Log and recover with a safe placeholder.
                tracing::error!(email = %self.email, "row email failed core::Email::parse");
                Email::parse("invalid@invalid.invalid").expect("hardcoded valid")
            }),
            created_at: self.created_at,
        }
    }
}

pub async fn find_user_by_id(pool: &PgPool, id: UserId) -> ApiResult<Option<User>> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, password_hash, created_at FROM users WHERE id = $1",
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(UserRow::into_user))
}

pub async fn find_row_by_email(pool: &PgPool, email: &str) -> ApiResult<Option<UserRow>> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, password_hash, created_at FROM users WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn insert_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
) -> ApiResult<UserId> {
    let id = Uuid::new_v4();
    let result = sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(email)
        .bind(password_hash)
        .execute(pool)
        .await;

    match result {
        Ok(_) => Ok(UserId(id)),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(ApiError::AlreadyExists)
        }
        Err(e) => Err(ApiError::Database(e)),
    }
}
```

> The `into_user` fallback path on email parse failure is awkward; the
> architect should weigh in on whether the DB schema should add a `CHECK`
> constraint (so this path truly is unreachable) and the parse can `expect`.
> Recorded in §7 as an open architect question.

### 3.11 `backend/crates/api/src/auth/mod.rs`

```rust
//! Auth surface: register, login, /auth/me.

mod extractor;
mod handlers;
mod password;
mod token;

pub use extractor::AuthenticatedUser;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(handlers::register))
        .route("/auth/login", post(handlers::login))
        .route("/auth/me", get(handlers::me))
}
```

### 3.12 `backend/crates/api/src/auth/handlers.rs`

```rust
//! HTTP handlers for the auth endpoints.

use axum::{extract::State, Json};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use fitai_core::UserId;

use crate::{
    auth::{password, token, AuthenticatedUser},
    db,
    error::{ApiError, ApiResult},
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct AuthRequest {
    #[validate(email)]
    email: String,
    #[validate(length(min = 8))]
    password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    user_id: UserId,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    token: String,
    user_id: UserId,
    expires_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    user_id: UserId,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<AuthRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<RegisterResponse>)> {
    req.validate().map_err(|_| ApiError::Validation { field: "email" })?;

    let hash = password::hash(&req.password).map_err(ApiError::Internal)?;
    let user_id = db::insert_user(&state.pool, &req.email.to_ascii_lowercase(), &hash).await?;

    Ok((axum::http::StatusCode::CREATED, Json(RegisterResponse { user_id })))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<AuthRequest>,
) -> ApiResult<Json<LoginResponse>> {
    req.validate().map_err(|_| ApiError::Validation { field: "email" })?;

    let lookup = db::find_row_by_email(&state.pool, &req.email.to_ascii_lowercase()).await?;

    // Constant-time defence: hash the input password even when the email
    // doesn't exist, so timing doesn't leak existence.
    match lookup {
        Some(row) => {
            if password::verify(&req.password, &row.password_hash).is_ok() {
                let user_id = UserId(row.id);
                let expires_at = Utc::now()
                    + Duration::from_std(state.jwt_ttl).expect("ttl fits chrono::Duration");
                let token = token::encode(user_id, state.jwt_ttl, &state.jwt_secret)
                    .map_err(ApiError::Internal)?;
                Ok(Json(LoginResponse {
                    token,
                    user_id,
                    expires_at,
                }))
            } else {
                Err(ApiError::Unauthorized)
            }
        }
        None => {
            // Burn comparable time so register-vs-login timing matches.
            let _ = password::hash(&req.password);
            Err(ApiError::Unauthorized)
        }
    }
}

pub async fn me(user: AuthenticatedUser) -> Json<MeResponse> {
    Json(MeResponse { user_id: user.user_id })
}
```

### 3.13 `backend/crates/api/src/auth/password.rs`

```rust
//! argon2id password hashing.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use eyre::WrapErr;

/// Hash a plaintext password using argon2id with default parameters and a
/// fresh per-password salt. Returns the PHC string (`$argon2id$v=19$…`).
pub fn hash(plain: &str) -> eyre::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon = Argon2::default();
    let hash = argon
        .hash_password(plain.as_bytes(), &salt)
        .wrap_err("argon2 hash")?
        .to_string();
    Ok(hash)
}

/// Verify a plaintext password against a stored PHC string.
/// Returns `Ok(())` on match, `Err` on mismatch or malformed hash.
pub fn verify(plain: &str, phc: &str) -> eyre::Result<()> {
    let parsed = PasswordHash::new(phc).wrap_err("parse PHC hash")?;
    Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .wrap_err("argon2 verify")?;
    Ok(())
}
```

### 3.14 `backend/crates/api/src/auth/token.rs`

```rust
//! HS256 JWT encode + decode for fitai-api.

use std::time::Duration;

use chrono::Utc;
use jsonwebtoken::{decode, encode as jwt_encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use fitai_core::UserId;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// User id as a string (Uuid).
    pub sub: String,
    /// Issued-at, seconds since epoch.
    pub iat: i64,
    /// Expiry, seconds since epoch.
    pub exp: i64,
}

pub fn encode(user_id: UserId, ttl: Duration, secret: &[u8]) -> eyre::Result<String> {
    let iat = Utc::now().timestamp();
    let exp = iat + i64::try_from(ttl.as_secs()).unwrap_or(i64::MAX);
    let claims = Claims {
        sub: user_id.0.to_string(),
        iat,
        exp,
    };
    let token = jwt_encode(&Header::default(), &claims, &EncodingKey::from_secret(secret))?;
    Ok(token)
}

pub fn decode_token(token: &str, secret: &[u8]) -> eyre::Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),  // HS256 + exp check
    )?;
    Ok(data.claims)
}
```

### 3.15 `backend/crates/api/src/auth/extractor.rs`

```rust
//! `AuthenticatedUser` extractor — turns a Bearer header into a user id.

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use uuid::Uuid;

use fitai_core::UserId;

use crate::{
    auth::token,
    db,
    error::ApiError,
    AppState,
};

pub struct AuthenticatedUser {
    pub user_id: UserId,
}

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;

        let token_str = header.strip_prefix("Bearer ").ok_or(ApiError::Unauthorized)?;
        let claims = token::decode_token(token_str, &state.jwt_secret)
            .map_err(|_| ApiError::Unauthorized)?;

        let uuid = Uuid::parse_str(&claims.sub).map_err(|_| ApiError::Unauthorized)?;
        let user_id = UserId(uuid);

        // AC5: confirm the user still exists.
        let user = db::find_user_by_id(&state.pool, user_id)
            .await
            .map_err(|_| ApiError::Unauthorized)?
            .ok_or(ApiError::Unauthorized)?;

        Ok(AuthenticatedUser { user_id: user.id })
    }
}
```

### 3.16 `backend/docker-compose.yml`

```yaml
services:
  postgres:
    image: postgres:16
    container_name: fitai-postgres
    environment:
      POSTGRES_USER: fitai
      POSTGRES_PASSWORD: dev
      POSTGRES_DB: fitai
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD", "pg_isready", "-U", "fitai", "-d", "fitai"]
      interval: 5s
      timeout: 5s
      retries: 10

volumes:
  postgres_data:
```

### 3.17 `backend/.env.example`

```dotenv
# fitai-api configuration. Copy to `backend/.env` for local dev.
# Real .env is gitignored.

# Postgres connection string. Matches `backend/docker-compose.yml`.
DATABASE_URL=postgres://fitai:dev@localhost:5432/fitai

# JWT signing secret for HS256. ANY non-empty string works for local dev.
# In production this comes from a secret manager (R-0026).
JWT_SECRET=dev-only-secret-replace-in-production

# Optional: HTTP listen port (default 8080).
# PORT=8080

# Optional: tracing filter. Defaults to "info".
# RUST_LOG=fitai_api=debug,info
```

### 3.18 `backend/scripts/dev/db.sh`

```bash
#!/usr/bin/env bash
# Local Postgres lifecycle helper for fitai-api dev. Wraps docker-compose
# and sqlx-cli. Requires: docker (colima), sqlx-cli (`cargo install sqlx-cli`).

set -euo pipefail

cmd="${1:-help}"
backend_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$backend_dir"

wait_healthy() {
    echo "[db.sh] waiting for postgres to be healthy..."
    for _ in $(seq 1 30); do
        if [[ "$(docker inspect -f '{{.State.Health.Status}}' fitai-postgres 2>/dev/null)" == "healthy" ]]; then
            echo "[db.sh] postgres healthy."
            return 0
        fi
        sleep 1
    done
    echo "[db.sh] FAIL: postgres did not become healthy in 30 s"
    return 1
}

case "$cmd" in
    up)
        docker compose up -d postgres
        wait_healthy
        DATABASE_URL="${DATABASE_URL:-postgres://fitai:dev@localhost:5432/fitai}" \
            sqlx migrate run
        ;;
    down)
        docker compose down
        ;;
    reset)
        docker compose down -v
        docker compose up -d postgres
        wait_healthy
        DATABASE_URL="${DATABASE_URL:-postgres://fitai:dev@localhost:5432/fitai}" \
            sqlx migrate run
        ;;
    migrate)
        DATABASE_URL="${DATABASE_URL:-postgres://fitai:dev@localhost:5432/fitai}" \
            sqlx migrate run
        ;;
    help|*)
        cat <<USAGE
db.sh — local Postgres lifecycle.

Usage:  scripts/dev/db.sh <command>

Commands:
  up        Bring up Postgres (creates volume on first run), wait healthy, run migrations.
  down      Stop and remove the container (volume preserved).
  reset     down -v + up (drops the volume; clears all data).
  migrate   Run pending sqlx migrations against the running DB.
  help      This message.
USAGE
        ;;
esac
```

### 3.19 `.github/workflows/ci.yml` — `rust` job additions

```yaml
  rust:
    name: rust (fmt, clippy, test, build)
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: backend
    services:                                  # NEW
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: fitai_ci
        ports:
          - 5432:5432
        options: >-
          --health-cmd "pg_isready -U postgres"
          --health-interval 5s --health-timeout 5s --health-retries 10
    env:                                       # NEW
      DATABASE_URL: postgres://postgres:postgres@localhost:5432/fitai_ci
      JWT_SECRET: ci-only-test-secret
    steps:
      - uses: actions/checkout@v4
      - name: install toolchain
        run: rustup show
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: backend
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
      - run: cargo test --workspace --all-features --locked
      - run: cargo build --workspace --all-targets --locked
```

The `mobile` and `docker` jobs are unchanged.

### 3.20 R-0001's qa scripts

`scripts/qa/r-0001-*` continue to work as-is — the AC verification commands
are the same. Note: AC1/AC2/AC3 will now require `DATABASE_URL` and
`JWT_SECRET` to be exported (because the test build links the new deps).
R-0001's `run-all.sh` will be extended in R-0002's own qa step 3 to set up
the local DB before running R-0002's auth-specific tests. R-0001's ACs
remain green.

## 4. Non-goals

- OAuth2 / social login flows (separate R).
- Refresh tokens, token rotation, token blacklisting (separate R).
- Password reset, account recovery, email verification (separate R).
- Two-factor authentication (separate R).
- Rate limiting on the auth endpoints (separate R; deserves its own middleware spec).
- Account deletion / GDPR export (R-0024 / later).
- Production secret management (R-0026).
- User profile fields — height, weight, goals, training history (**R-0003**, which extends `core::User`).
- Admin role / RBAC (separate R when product needs it).
- Bulk-import of users (out of MVP scope entirely).
- A web UI for auth — the Flutter app is the only client (R-0007 onward).
- A `sqlx prepare` (`.sqlx/` offline-mode cache) step in CI. CI has a live DB at compile time, so live-checked queries are fine. Revisit if a build-without-DB workflow becomes a need.

## 5. Open questions

For the **architect** review to settle:

- **OQ-A1.** `UserRow::into_user`'s fallback path on `Email::parse` failure: should the DB schema add a `CHECK` constraint that enforces format (so the path becomes truly unreachable and we can `expect`), or is the recover-and-log behaviour preferred? Currently the spec keeps the recover-and-log; a `CHECK` constraint would be cleaner but adds a migration concern. *Architect call.*
- **OQ-A2.** Workspace lint setup for the new `fitai-core` crate — should we explicitly forbid `tokio` / `axum` / `sqlx` dependencies in `core` (perhaps via `cargo-deny`)? `cargo-deny` isn't in the project yet; the discipline is enforced socially today.
- **OQ-A3.** The `users_email_idx` line in §3.1 is redundant with the `UNIQUE` constraint; planned to be removed before commit. Architect should confirm.

For the **owner** to settle (none — all R-0002 OQ1–OQ4 already locked at requirement-discussion). Implementer mechanical choices (exact crate versions) are recorded at implementation time per §2.8.

## 6. Acceptance criteria

Each SAC maps back to an R-0002 AC; each becomes one or more `qa` agent tests.

- [ ] **SAC1 → AC1.** `00001_users.sql` exists at `backend/migrations/00001_users.sql`. Running migrations against a fresh Postgres DB succeeds. A SELECT against `information_schema.columns WHERE table_name = 'users'` returns rows for `id`, `email`, `password_hash`, `created_at` with the expected types and constraints (UNIQUE on email, NOT NULL on the others).
- [ ] **SAC2 → AC2.** `POST /auth/register` with `{ "email": "a@b.com", "password": "8charsmin" }`:
  - on a fresh DB returns 201 + `{ "user_id": "<uuid>" }`, and a `SELECT` after the call finds exactly one row whose `password_hash` starts with `$argon2id$`;
  - a second identical call returns 409 + `{ "error": "already_exists" }`;
  - a call with `{ "email": "not-an-email", "password": "8charsmin" }` returns 400 + `{ "error": "validation", "field": "email" }`;
  - a call with `{ "email": "a@b.com" }` (no password) returns 400.
- [ ] **SAC3 → AC3.** After AC2 leaves a user in the DB:
  - `POST /auth/login` with the correct credentials returns 200 + JSON with a non-empty `token`, the matching `user_id`, and an `expires_at` that's 24h ± 5 s in the future;
  - same email + wrong password returns 401 + `{ "error": "unauthorized" }`;
  - unknown email returns 401 with the **identical** body (byte-for-byte).
- [ ] **SAC4 → AC4.** Decoding the token from SAC3 with the test `JWT_SECRET` succeeds and yields `sub = user_id.to_string()`, `iat` and `exp` are i64s with `exp - iat` between 86 395 and 86 405 (24h ± 5 s slack). Decoding with a different secret fails with `jsonwebtoken::errors::ErrorKind::InvalidSignature`.
- [ ] **SAC5 → AC5.** `GET /auth/me` with `Authorization: Bearer <valid token>` returns 200 + `{ "user_id": "<uuid>" }`. Five separate test cases each return 401 + `{ "error": "unauthorized" }`: (a) missing `Authorization`; (b) `Authorization: Token <jwt>` (wrong scheme); (c) the token from SAC3 with one character of the signature flipped; (d) a token issued with `jwt_ttl = 0` so it's already expired (or `exp` rewound); (e) a structurally valid token whose `sub` UUID is not in the `users` table.
- [ ] **SAC6 → AC6.** A `#[traced_test]` integration test exercises the register path with a recognisable password (`"recognisable-plaintext-pw"`) and asserts that the captured tracing output **does not contain** the substring — both before and after enabling DEBUG-level logs. The `argon2`-hashed PHC string and the user_id may appear.
- [ ] **SAC7 → AC7.** At least **ten** `#[sqlx::test(migrations = "../../migrations")]` integration tests pass, covering each branch in SAC2/SAC3/SAC5. Exact list authored by qa during step 3.
- [ ] **SAC8 → AC8.** The `rust` CI job spins up Postgres 16 as a service, exports `DATABASE_URL` / `JWT_SECRET`, and runs `cargo test --workspace --all-features --locked` against it; all auth integration tests pass. Locally, `bash backend/scripts/dev/db.sh up` followed by `cd backend && cargo test` produces the same result. R-0001's eight ACs remain green (no regression).

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-05-29 | **`crates/core/` extracted at R-0002.** SPEC-0001 §2.2 trigger fired one R earlier than written; `User` is the first domain type. SPEC-0001 §2.2 + §7 updated with a 2026-05-29 addendum. | Avoids a future refactor at R-0003. Honors the spirit of the trigger (first domain type), not the letter (R name). Owner-approved (design discussion). |
| 2026-05-29 | **`core` is pure: no `sqlx`, no `axum`, no http types.** | Domain ↔ persistence separation; lets future targets (e.g. CLI tools, tests) consume `core` without dragging a DB driver. |
| 2026-05-29 | **`password_hash` lives on `db::UserRow`, not `core::User`.** Conversion strips it at the seam. | Shrinks AC6's no-plaintext-in-logs audit surface. The hash exists in exactly one struct definition. |
| 2026-05-29 | **JWT signing algorithm: HS256.** Single secret, `JWT_SECRET` env. | Owner-approved (R-0002 OQ1). Switch to RS256/EdDSA if federation arises. |
| 2026-05-29 | **Access-token lifetime: 24h.** No refresh tokens in R-0002. | Owner-approved (R-0002 OQ2). Refresh tokens earn their own R when product needs them. |
| 2026-05-29 | **Local Postgres via `backend/docker-compose.yml` (Postgres 16).** Helper script `backend/scripts/dev/db.sh up|down|reset|migrate`. | Owner-approved (R-0002 OQ3). Same image as CI; no version skew. |
| 2026-05-29 | **Migration tool: `sqlx::migrate!` macro, files in `backend/migrations/`, run from `main.rs` at startup.** | Owner-approved (R-0002 OQ4 + design Q3). Atomic + advisory-locked; devs get migrations on `cargo run`. |
| 2026-05-29 | **Test isolation: `#[sqlx::test(migrations = "../../migrations")]`** — fresh per-test DB. | Owner-approved (design Q2). Trivially isolated; ~1.5 s/test overhead acceptable for ~12 tests. |
| 2026-05-29 | **Email validation: `validator` crate's `#[validate(email)]` at the handler boundary**, plus `core::Email::parse` invariant at the domain boundary. | Owner-approved (design Q4). Two-layer defence: framework-grade format check at ingress, type-level guarantee inside the domain. |
| 2026-05-29 | **Password hashing: argon2id (`argon2` crate), default parameters.** | OWASP modern recommendation (2025+). Default params tuned for current hardware. |
| 2026-05-29 | **Login error: enumeration-safe.** Wrong-password and unknown-email return identical body + status. Login also burns a `password::hash` call on the unknown-email path to keep response timing comparable. | Standard practice; both privacy + timing-leak mitigations. |
| 2026-05-29 | **`AppState.jwt_ttl: Duration`** is **injected**, not a const. | Tests can issue already-expired tokens by passing `Duration::ZERO`; production uses 24h. Configurability without env-var sprawl. |
| 2026-05-29 | **CI `JWT_SECRET = "ci-only-test-secret"`** committed in the workflow. Production `JWT_SECRET` is a secret-manager concern in R-0026. | Repo-public secret is harmless because the CI db is also ephemeral; tests need a stable secret to assert on token decoding. |
| 2026-05-29 | **Architect questions OQ-A1 / OQ-A2 / OQ-A3 in §5 carried into the architect review** rather than pre-resolved by me. | These have real trade-offs and the architect's perspective on long-term invariants is exactly the point of the review. |
| 2026-05-29 | **Lockstep snippet policy from SPEC-0001 §7 (architect finding #1) remains in force.** Any clippy-pedantic failures during implementation get patched in spec + impl together. | Same discipline as R-0001; the gate is project-wide, not per-spec. |

## Changelog

- _2026-05-29 — created (Draft); decisions OQ1–OQ4 + 10 derived choices recorded. Pending `architect` agent review._
