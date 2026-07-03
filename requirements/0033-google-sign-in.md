# R-0033 — Google Sign-In

- **Status:** Accepted
- **Milestone:** M3 (fast-track) / auth extension
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-07-02
- **Depends on:** R-0002 (Done — JWT auth this issues tokens through),
  R-0007 (Done — Flutter auth shell this plugs into)
- **Realized by:** SPEC-0033 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

Users can sign in with their Google account. "Continue with Google" on the
login screen obtains a Google ID token client-side; the backend verifies it
against Google's public keys, finds-or-creates the user by verified email, and
issues the same first-party JWT the password flow issues. Password login
remains available; both methods can attach to the same account (same email).

## 2. Rationale

The owner wants to sign in with Google for day-to-day use and testing instead
of managing a password. This is the "OAuth2 social login" extension explicitly
deferred from R-0002 — promoted now because personal MVP testing is active.
Debug builds already have a friction-free path (the DevLogin test-account
button); Google sign-in is the production answer.

## 3. Acceptance criteria

- **AC1.** `POST /auth/google` (unauthenticated) accepts `{ id_token: string }`,
  verifies signature/audience/expiry against Google's JWKS, and returns the
  standard `{ token, user_id, expires_at }` on success.
- **AC2.** Verification failures (bad signature, wrong `aud`, expired, email
  not verified) return 401 with a typed error — never a 5xx.
- **AC3.** First Google sign-in with an unknown verified email creates a user
  (no password hash); subsequent sign-ins map to the same user. A Google
  sign-in with an email that already has a password account logs into that
  account (email is the identity key).
- **AC4.** Users created via Google can not log in via the password flow
  (401, non-enumerating) until they set a password (out of scope here).
- **AC5.** Migration: `users.password_hash` becomes nullable (or an auth-method
  column is added — SPEC decides), with existing rows unaffected.
- **AC6.** Flutter: a "Continue with Google" button on the login screen using
  `google_sign_in`, shown only when a Google client id is configured
  (`GOOGLE_CLIENT_ID` dart-define); hidden otherwise — no dead UI.
- **AC7.** `GOOGLE_CLIENT_ID` (Flutter) and `GOOGLE_OAUTH_AUDIENCE` (backend
  env var) are configuration — never hardcoded.
- **AC8.** Tests — backend: JWKS verification against fixed test vectors
  (stubbed keys), find-or-create paths, 401 paths. Flutter: button hidden
  without client id, sign-in flow mocked behind a seam, token handoff to the
  existing `AuthController`.
- **AC9.** Scope guard: no other providers (Apple/Facebook), no account
  linking UI, no password-set flow, no refresh tokens.

## 4. Owner prerequisites (blocking step 5)

The implementation cannot start end-to-end testing until the owner creates,
in Google Cloud Console (any project):

1. An **OAuth consent screen** (External, testing mode is fine).
2. An **OAuth 2.0 Client ID** of type *Web application* with the dev origins
   (`http://localhost:9422`, later `https://fit.goosethropic.systems`).
3. Hands the client id to the repo as dart-define/env config (it is not a
   secret, but lives outside source per AC7).

Claude cannot create these (account/consent actions).

## 5. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-02 | Email is the identity key; find-or-create on verified email | Simplest correct account model; matches R-0002's unique-email constraint. |
| 2026-07-02 | Backend verifies the ID token itself (JWKS), no Google server SDK | Keeps the Rust surface small; standard JOSE verification. |
| 2026-07-02 | Button hidden without configured client id | No dead UI in builds that can't complete the flow. |

## Changelog

- _2026-07-02 — created and **Accepted** (owner request in session)._
