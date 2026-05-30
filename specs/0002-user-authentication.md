# SPEC-0002 — User authentication

- **Status:** Accepted
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
carries `password_hash`; the fallible `UserRow::into_user` conversion strips it
when producing `core::User`. Smaller code-paths see the hash → smaller AC6
(no-plaintext-in-logs) audit surface.

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
3. `auth::token::decode_token(token, &state.jwt_secret)` → on any error, `Unauthorized`.
4. Parse `claims.sub` as `Uuid` → on failure, `Unauthorized`.
5. `db::find_user_by_id(&state.pool, user_id)` → `None` ⇒ `Unauthorized`; `Some(user)` ⇒ `Ok(AuthenticatedUser { user_id })`.

Every failure path returns the **same** `ApiError::Unauthorized` — no
distinguishing body or header. AC5 verifies all five branches.

### 2.4 Auth flow details

- **Body extraction.** Both `register` and `login` extract the body as `Result<Json<AuthRequest>, JsonRejection>`; a serde deserialization failure (missing/empty `email` or `password`, malformed JSON) maps to `ApiError::Validation { field: "body" }` → 400. This is what makes the "no password → 400" case (SAC2) actually reach a 400 rather than axum's default extractor rejection.
- **Register.** Validate `AuthRequest { email, password }` with `validator`'s `length(min = 8)` on the **password only** (mapped to `ApiError::Validation { field: "password" }` → 400). The email is deliberately *not* gated by `#[validate(email)]`: `core::Email::parse` is the single email validation+normalization authority, and a raw `#[validate(email)]` would reject a padded/mixed-case address before normalization, breaking case-insensitive duplicate detection (SAC2). Normalize through the domain authority: `let email = Email::parse(&req.email).map_err(|_| ApiError::Validation { field: "email" })?;` (a malformed address still yields 400 + `field: "email"`) and persist `email.as_str()` — `core::Email` is the *single* normalization point (trim + lowercase), so handler and DB never disagree. Hash the password (`auth::password::hash`). `INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)` — on `unique_violation` map to `ApiError::AlreadyExists`. Return `201 { user_id }`.
- **Login.** Normalize the lookup email via `Email::parse` (same authority), look up by `email.as_str()`. If `None`, hash the supplied password anyway (best-effort timing-equalization against email-enumeration timing leaks — *not* a hard constant-time guarantee; rate-limiting is the real defence and is deferred) then return `Unauthorized`. If `Some(row)`, `auth::password::verify` against `row.password_hash`. On match, `auth::token::encode` returns `(token, exp)`; report that exact `exp` as `expires_at` and return `200 { token, user_id, expires_at }`. On mismatch, return `Unauthorized`.
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
| `validator` | `0.20` | `length(min)` on password (email is validated by `core::Email::parse`) |
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
```

> No separate index on `email`: the `UNIQUE` constraint already creates a
> backing B-tree index, so a `CREATE INDEX users_email_idx` would be a pure
> duplicate (write amplification, zero read benefit). Architect-confirmed
> (OQ-A3). *Decision recorded in §7.*

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

/// Email newtype with a validating constructor. `parse` is the single email
/// validation+normalization authority on both the write and lookup paths; it
/// trims, lowercases, and checks basic shape, so downstream code can rely on
/// the "well-formed and normalized at construction" invariant without
/// re-validating.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Email(String);

#[derive(Debug, Error)]
#[error("invalid email format")]
pub struct EmailParseError;

impl Email {
    /// Construct from a `&str`. Returns `EmailParseError` on a malformed
    /// input. Trims surrounding whitespace and lowercases, then checks basic
    /// shape: presence of `@` with non-empty local + domain parts and a dot in
    /// the domain. This is the only email gate — the handler does not also run
    /// `#[validate(email)]`, which would reject a padded address pre-normalization.
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
/// detail kept in `fitai_api::db::UserRow`. No `Deserialize`: `User` is only
/// ever produced from a `UserRow` (DB), never parsed from the wire — and
/// `Email` intentionally has no `Deserialize` (it must go through `parse`).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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
        jwt_ttl: Duration::from_hours(24),
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
    pub password_hash: String,  // only crosses the seam via find_row_by_email → login (verify needs it); into_user strips it everywhere else
    pub created_at: DateTime<Utc>,
}

impl UserRow {
    /// Convert a persisted row into the domain `User`, stripping
    /// `password_hash`. Fallible: a stored `email` that fails
    /// `core::Email::parse` is data corruption (we only ever write
    /// parsed-and-normalized emails), so surface it loudly as a 500 rather
    /// than fabricating a placeholder identity. §6 (CLAUDE.md): aborting /
    /// recovering-with-a-lie is wrong; a typed error is right.
    pub fn into_user(self) -> ApiResult<User> {
        let email = Email::parse(&self.email).map_err(|_| {
            tracing::error!(user_id = %self.id, "stored email failed core::Email::parse — data corruption");
            ApiError::Internal(eyre::eyre!("stored email failed domain validation"))
        })?;
        Ok(User {
            id: UserId(self.id),
            email,
            created_at: self.created_at,
        })
    }
}

pub async fn find_user_by_id(pool: &PgPool, id: UserId) -> ApiResult<Option<User>> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, password_hash, created_at FROM users WHERE id = $1",
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?;
    row.map(UserRow::into_user).transpose()
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

> `into_user` is fallible by design (OQ-A1, architect-resolved): a row whose
> `email` fails `core::Email::parse` is corruption and becomes a logged 500,
> never a fabricated identity. No DB `CHECK` constraint — that would duplicate
> validation in a drift-prone Postgres regex. *Recorded in §7.*

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

use axum::{extract::rejection::JsonRejection, extract::State, http::StatusCode, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use fitai_core::{Email, UserId};

use crate::{
    auth::{password, token, AuthenticatedUser},
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
fn body(req: Result<Json<AuthRequest>, JsonRejection>) -> ApiResult<AuthRequest> {
    let Json(req) = req.map_err(|_| ApiError::Validation { field: "body" })?;
    req.validate().map_err(|_| ApiError::Validation { field: "password" })?;
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
}

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
        // the real defence (deferred, see §4).
        let _ = password::hash(&req.password);
        return Err(ApiError::Unauthorized);
    };

    if password::verify(&req.password, &row.password_hash).is_err() {
        return Err(ApiError::Unauthorized);
    }

    let user_id = UserId(row.id);
    let (token, expires_at) =
        token::encode(user_id, state.jwt_ttl, &state.jwt_secret).map_err(ApiError::Internal)?;
    Ok(Json(LoginResponse { token, user_id, expires_at }))
}

pub(crate) async fn me(user: AuthenticatedUser) -> Json<MeResponse> {
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

// `argon2::password_hash::Error` does not implement `std::error::Error`, so it
// can't flow through `eyre`'s `wrap_err`/`?`; we map it to an `eyre::Report`
// via its `Display`.

/// Hash a plaintext password using argon2id with default parameters and a
/// fresh per-password salt. Returns the PHC string (`$argon2id$v=19$…`).
pub(crate) fn hash(plain: &str) -> eyre::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon = Argon2::default();
    let hash = argon
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| eyre::eyre!("argon2 hash: {e}"))?
        .to_string();
    Ok(hash)
}

/// Verify a plaintext password against a stored PHC string.
/// Returns `Ok(())` on match, `Err` on mismatch or malformed hash.
pub(crate) fn verify(plain: &str, phc: &str) -> eyre::Result<()> {
    let parsed = PasswordHash::new(phc).map_err(|e| eyre::eyre!("parse PHC hash: {e}"))?;
    Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .map_err(|e| eyre::eyre!("argon2 verify: {e}"))?;
    Ok(())
}
```

### 3.14 `backend/crates/api/src/auth/token.rs`

```rust
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
    let token = jwt_encode(&Header::default(), &claims, &EncodingKey::from_secret(secret))?;
    let expires_at = Utc
        .timestamp_opt(exp, 0)
        .single()
        .ok_or_else(|| eyre::eyre!("exp timestamp out of range"))?;
    Ok((token, expires_at))
}

/// Decode and validate an HS256 JWT. The signature is checked by
/// `jsonwebtoken`; the `exp` claim is enforced **here** as `exp <= now`
/// (expired) rather than leaning on `jsonwebtoken`'s `exp < now`. The design
/// treats a token as dead the instant it reaches its expiry second, so a
/// `Duration::ZERO` token (`exp == iat == now`) is already expired on arrival
/// (SAC5(d)) — `jsonwebtoken`'s strict `<` would accept it for the rest of that
/// whole-second tick.
pub(crate) fn decode_token(token: &str, secret: &[u8]) -> eyre::Result<Claims> {
    let mut validation = Validation::default();
    validation.validate_exp = false; // enforced explicitly below for `<=` semantics
    let data = decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)?;
    if data.claims.exp <= Utc::now().timestamp() {
        return Err(eyre::eyre!("token expired"));
    }
    Ok(data.claims)
}
```

### 3.15 `backend/crates/api/src/auth/extractor.rs`

```rust
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

// axum 0.7's `FromRequestParts` is still an `#[async_trait]` trait (native
// `async fn` in traits is an axum 0.8 change), so the impl must carry the
// attribute or it fails to satisfy the trait's lifetime bounds.
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

All three architect questions were settled in the 2026-05-30 architect review
(resolutions in §7):

- **OQ-A1 — *resolved*.** `UserRow::into_user` is made **fallible** (`-> ApiResult<User>`); a stored email that fails `core::Email::parse` is data corruption and becomes a logged `ApiError::Internal` (500), never a fabricated placeholder identity. **No** DB `CHECK` constraint — a Postgres regex would duplicate validation and drift from `validator` + `Email::parse`. (§3.10)
- **OQ-A2 — *resolved*.** Do **not** add `cargo-deny` now (premature for one pure crate). Instead record a *trigger*: introduce `cargo-deny` with a `bans` section excluding async/web/db crates from `core` when a second `core`-consuming crate lands, or when `core` gains its first non-trivial dependency surface — whichever comes first. Until then the inward-dependency rule is architect-enforced.
- **OQ-A3 — *resolved*, confirmed.** The redundant `users_email_idx` is removed from §3.1; the `UNIQUE` constraint already provides the backing index.

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
| 2026-05-30 | **OQ-A1 resolved: `UserRow::into_user` is fallible (`-> ApiResult<User>`); corrupt stored email → logged `ApiError::Internal` (500). No DB `CHECK` constraint.** | Architect review. A fabricated placeholder identity violates §6 (surface errors, never recover-with-a-lie); a Postgres email regex would drift from `validator`/`Email::parse`. `core::Email` stays the single normalization authority. |
| 2026-05-30 | **OQ-A2 resolved: no `cargo-deny` yet; record a trigger** — add it (with a `bans` section excluding async/web/db crates from `core`) when a second `core`-consuming crate lands or `core` gains its first non-trivial dependency. | Architect review. §2 "no premature anything": the guard is unneeded for one pure, architect-reviewed crate. |
| 2026-05-30 | **OQ-A3 resolved: `users_email_idx` removed.** | Architect review. `UNIQUE` already creates the backing B-tree index; a separate index is pure write-amplification with no read benefit. |
| 2026-05-30 | **Body extraction via `Result<Json<AuthRequest>, JsonRejection>` mapped to 400.** Email normalized through `core::Email::parse` on both write and lookup paths; DB persists `email.as_str()`. | Architect review (blocking finding 1 + major finding 3). The default `Json` extractor would reject a missing-`password` body before the handler runs, returning the wrong status for SAC2; routing all body rejections through `ApiError::Validation` fixes it. Single normalization authority prevents handler/DB email divergence. |
| 2026-05-30 | **`token::encode` returns `(token, exp)`; handler reports the token's actual `exp` as `expires_at`.** Function renamed in prose to `decode_token` everywhere (was `decode` in §2.3). `User` drops `Deserialize` (it is never wire-parsed; `Email` has none). | Architect review (findings 2, 5, 7). Removes a sub-second `expires_at`/`exp` skew, fixes a prose/code name mismatch, and removes a non-compiling derive. |
| 2026-05-30 | **Timing-defence wording softened to "best-effort timing-equalization", not "constant-time".** | Architect review (finding 6). `hash` and `verify` have different timings; rate-limiting (deferred) is the real defence. Honest wording avoids future misreading of the guarantee. |
| 2026-05-30 | **Supersedes the 2026-05-29 "two-layer email defence": `#[validate(email)]` is dropped; `core::Email::parse` is the sole email validation+normalization gate.** Password-only `validate()` failures now map to `field: "password"`. | Step-7 CI surfaced a real conflict: the ingress `#[validate(email)]` rejected the padded/mixed-case `"  case@b.COM  "` with 400 *before* `Email::parse` could trim+lowercase it, so SAC2's case-insensitive-duplicate test got 400 instead of 409. The two layers disagreed; the single-authority direction (finding 4) wins. A malformed address still returns 400 + `field: "email"` via `Email::parse`. |
| 2026-05-30 | **Expiry boundary is `exp <= now` (expired), enforced in `decode_token` (not `jsonwebtoken`'s `exp < now`).** `decode_token` sets `validation.validate_exp = false` and checks `exp <= Utc::now().timestamp()` itself. | Step-7 CI: the `leeway = 0` approach still let a `Duration::ZERO` token (`exp == iat == now`) authenticate, because `jsonwebtoken` treats `exp == now` as valid for the rest of that whole-second tick — so SAC5(d)'s expired-token test got 200 instead of 401. Honoring the documented "`Duration::ZERO` ⇒ already expired" contract (decision 2026-05-29) requires owning the `<=` comparison. |

## Changelog

- _2026-05-29 — created (Draft); decisions OQ1–OQ4 + 10 derived choices recorded. Pending `architect` agent review._
- _2026-05-30 — revised per architect review (REQUEST CHANGES). Applied blocking fixes 1/2/7 (JsonRejection→400 body extraction, `decode_token` rename, `User` drops `Deserialize`), major fixes 3/4 (`core::Email` single normalization authority; fallible `into_user`), minor fixes 5/6 (`encode` returns `(token, exp)`; timing wording). Settled OQ-A1/A2/A3 in §5 + §7. Awaiting owner ratification to flip Accepted._
- _2026-05-30 — step-5 implementation lockstep: §3 snippets patched to match the merged code under the pinned toolchain (Rust/clippy 1.95.0). auth internals dropped from `pub` to `pub(crate)` (`unreachable_pub`); `login` rewritten as a `let-else` guard (`clippy::single_match_else`); `password` maps the non-`std::error::Error` argon2 error via `eyre::eyre!` instead of `wrap_err`; `decode_token` sets `validation.leeway = 0`; `extractor` impl carries `#[async_trait]` (axum 0.7); `jwt_ttl` uses `Duration::from_hours(24)` (`duration_suboptimal_units`). No change to the §2 contract._
- _2026-05-30 — step-7 CI fix: dropped `#[validate(email)]` so `core::Email::parse` is the sole email gate (the ingress validator rejected padded/mixed-case input before normalization, failing SAC2's case-insensitive duplicate test with 400 instead of 409). Password `validate()` failures now map to `field: "password"`. Updated §2 register prose, the `Email` type docs, the dependency table, and the decision log (supersedes the 2026-05-29 two-layer decision)._
- _2026-05-30 — step-7 CI fix: `decode_token` enforces `exp <= now` itself (`validation.validate_exp = false` + explicit check) instead of `validation.leeway = 0`. `jsonwebtoken`'s strict `exp < now` accepted a `Duration::ZERO` token within the same whole-second tick, failing SAC5(d) (200 instead of 401). §3.14 snippet patched in lockstep._
