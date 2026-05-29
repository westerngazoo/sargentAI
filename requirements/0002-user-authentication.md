# R-0002 — User authentication

- **Status:** Draft
- **Milestone:** M1
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-05-28
- **Depends on:** R-0001 (Done)
- **Realized by:** SPEC-0002 (to be written once this R is `Accepted`)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

A user can **register** with an email + password and **log in** to receive a
short-lived JWT. A bearer-token middleware validates JWTs on protected routes
and extracts the authenticated `user_id`. User records persist in Postgres;
passwords hash with **argon2id**.

OAuth2 / social login is **explicitly out of scope** for R-0002 — it will land
as its own R when (and if) the product team commits to it. See R-0001 §6's
open question on social login.

This requirement establishes three new project primitives that every later R
will reuse:

1. **Postgres connection + migrations** via `sqlx` (introduces the `backend/migrations/` directory).
2. **The `auth` layer** — password hashing (argon2id), JWT issuance and verification (HS256), an axum extractor that turns a `Bearer` header into a typed `AuthenticatedUser`.
3. **Database lifecycle in CI and dev** — Postgres 16 service container in CI; `backend/docker-compose.yml` locally.

## 2. Rationale

R-0003 (user profile) and **every subsequent R that touches user data** needs
an authenticated user to scope queries to. R-0002 is the smallest possible
slice that delivers that, with one new endpoint (`GET /auth/me`) that exists
only to prove the middleware works end-to-end.

Choosing JWT (rather than session cookies) matches the mobile-first
architecture: the Flutter client holds the token in secure storage and sends
it as a `Bearer` header; the backend remains stateless on auth.

## 3. Acceptance criteria

Each criterion is observable from a checkout of the R-0002 branch with the
toolchain installed (Rust 1.95.0, Postgres 16 reachable, `JWT_SECRET` set).

- **AC1.** A SQL migration in `backend/migrations/` creates a `users` table
  with columns: `id UUID PRIMARY KEY`, `email TEXT UNIQUE NOT NULL`,
  `password_hash TEXT NOT NULL`, `created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`.
  Running migrations from a clean Postgres database succeeds.
- **AC2.** `POST /auth/register` with a JSON body `{ "email": ..., "password": ... }`:
  - returns `201 Created` and JSON `{ "user_id": "<uuid>" }` on first-time valid input;
  - returns `409 Conflict` when the email already exists;
  - returns `400 Bad Request` when the email is malformed or either field is missing/empty;
  - persists exactly one new row in `users` on success, with `password_hash` in argon2id format (`$argon2id$…`).
- **AC3.** `POST /auth/login` with a JSON body `{ "email": ..., "password": ... }`:
  - returns `200 OK` and JSON `{ "token": "<jwt>", "user_id": "<uuid>", "expires_at": "<rfc3339>" }` on valid credentials;
  - returns `401 Unauthorized` on wrong password or unknown email (response body identical between the two cases to avoid email enumeration).
- **AC4.** The issued JWT is HS256, signed with the `JWT_SECRET` env var, and
  carries claims `sub` (the user_id as a string), `exp` (now + 24h), `iat`
  (now). Decoding succeeds with the same secret and fails (signature error)
  with any other.
- **AC5.** `GET /auth/me` is wired through an axum extractor that validates
  the `Authorization: Bearer <jwt>` header:
  - returns `200 OK` and `{ "user_id": "<uuid>" }` for a valid, unexpired token whose `sub` resolves to a known user;
  - returns `401 Unauthorized` for: missing header, malformed header, invalid signature, expired token, or `sub` not in the `users` table.
- **AC6.** Passwords are hashed with argon2id using the `argon2` crate's
  default parameters. Plaintext passwords never appear in logs (verified by
  the test that exercises register + a tracing-capture).
- **AC7.** At least **ten** integration tests cover the surface above,
  including: register success, register dup-email 409, register bad-format 400,
  register missing-field 400, login success, login wrong-password 401,
  login unknown-email 401, `/auth/me` success, `/auth/me` expired 401,
  `/auth/me` invalid-signature 401, `/auth/me` missing-header 401. Tests
  use an isolated database (per-test schema *or* per-suite truncate-and-seed
  — to be settled in SPEC-0002).
- **AC8.** Database lifecycle:
  - **In CI:** the `rust` job spins up a Postgres 16 service container, exports `DATABASE_URL`, runs migrations, runs all tests (including the new auth integration tests), then tears down.
  - **Locally:** `backend/docker-compose.yml` brings up Postgres 16 on a fixed port; a `scripts/dev/db.sh up|down|reset` helper (or `make` target — settled in SPEC-0002) wraps `docker compose` and `sqlx migrate run`.

## 4. Constraints & non-goals

**In scope (R-0002):**
- The endpoints, table, middleware, and DB lifecycle described in §3.
- The new `Cargo.toml` deps: `sqlx` (with `runtime-tokio` + `postgres` + `uuid` + `chrono` features), `argon2`, `jsonwebtoken`, `serde`, `uuid`, `chrono`, plus a small `validator`-style crate for email format (TBD in SPEC-0002).
- `JWT_SECRET` env var and its default-for-dev value (committed in `.env.example`, never the real value).

**Out of scope (deferred):**
- **OAuth2 / social login** — separate R; flagged from R-0001 §6.
- **Refresh tokens / token rotation** — separate R; 24h access tokens only.
- **Password reset / forgot-password flow** — separate R.
- **Email verification (double opt-in)** — separate R; productionization concern.
- **Rate limiting on `/auth/register` and `/auth/login`** — separate R; depends on choice of rate-limit middleware.
- **Account deletion / GDPR export** — separate R; depends on M8 privacy work (R-0024).
- **Production secret management** (real `JWT_SECRET`, rotation) — R-0026.
- **User profile fields** (height, weight, goals, training history) — **R-0003**, which extracts a `crates/core/` per SPEC-0001 §2.2's trigger and owns the profile domain types.
- Token revocation / blacklist (JWT is intentionally stateless).
- Two-factor authentication.

## 5. Open questions

None remaining. OQ1–OQ4 settled in chat 2026-05-28:

- **OQ1 — JWT algorithm:** HS256. Recorded in §6.
- **OQ2 — Access-token lifetime:** 24h, no refresh tokens (refresh tokens become their own R when needed). Recorded in §6.
- **OQ3 — Local Postgres:** `backend/docker-compose.yml` with Postgres 16. Recorded in §6.
- **OQ4 — Migration tool:** `sqlx::migrate!` macro + `backend/migrations/` SQL files. Recorded in §6.

Implementation-level questions (e.g. exact crate versions, per-test vs per-suite DB isolation, `make` vs shell script for the dev helper, exact email validator crate) are deferred to SPEC-0002, where they belong.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-05-28 | **JWT signing algorithm: HS256** (shared `JWT_SECRET`). | Single-API-server topology; one secret to manage; simplest path. Switch to RS256/EdDSA if/when federation arises. Owner-approved (OQ1). |
| 2026-05-28 | **Access-token lifetime: 24h.** No refresh tokens in R-0002. | Pragmatic for pre-launch mobile. Refresh tokens add an entire token-rotation surface; they earn their own R when product needs them. Owner-approved (OQ2). |
| 2026-05-28 | **Local Postgres: `backend/docker-compose.yml` (Postgres 16).** Dev runs `docker compose up -d` (colima is already installed from R-0001). | Same image CI uses → no version skew. Avoids requiring devs to install Postgres natively. Owner-approved (OQ3). |
| 2026-05-28 | **Migration tool: `sqlx::migrate!` macro** with SQL files in `backend/migrations/`. | Compile-time discovery, no extra binary, runs at app start. Conventional with the rest of sqlx. Owner-approved (OQ4). |
| 2026-05-28 | **Password hashing: argon2id (`argon2` crate, default parameters).** | Modern OWASP recommendation; supersedes bcrypt/scrypt for new projects. Default parameters are tuned for 2025+ hardware. |
| 2026-05-28 | **Login error messages don't distinguish "unknown email" from "wrong password".** Both return 401 with identical body. | Avoids email-enumeration. Standard practice. |
| 2026-05-28 | **OAuth2 / social login deferred to a separate R.** | Keeps R-0002's surface tight; OAuth2 introduces provider-specific flows and consent screens that earn their own loop. Carried forward from R-0001 §6. |

## Changelog

- _2026-05-28 — created (Draft); decisions OQ1–OQ4 + three derived choices recorded._
