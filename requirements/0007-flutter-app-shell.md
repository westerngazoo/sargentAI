# R-0007 — Flutter app architecture & auth shell

- **Status:** Accepted
- **Milestone:** M3
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-06-02
- **Depends on:** R-0001 (Done — Flutter scaffold under `/mobile`), R-0002 (Done — `/auth/register`, `/auth/login`, `/auth/me`)
- **Realized by:** [SPEC-0007](../specs/0007-flutter-app-shell.md) (Accepted)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

A person can **register, log in, and reach an authenticated home shell** from the
fitAI Flutter app, and the app carries the **cross-cutting architecture every
later screen depends on**. Today `/mobile` is a bare hello-world (one screen, no
dependencies) — this requirement turns it into a real, authenticated client
skeleton: Riverpod state management, a typed HTTP client pointed at a
configurable API base URL, the JWT held in platform secure storage, a router
that gates authenticated vs. unauthenticated routes, and one uniform
loading/error pattern.

This is the project's **first mobile feature work** and is deliberately scoped to
the shell only. It builds **no feature logger UI** — onboarding, the workout and
nutrition loggers, and the dashboard are separate requirements (R-0008+) that
stand on this foundation. The value is that the architecture is decided and
reviewed **once**, cleanly, so each later screen is a thin, fast loop on top of
it — mirroring how R-0001 built the backend skeleton before any backend feature.

## 2. Rationale

There is currently **no way for a human to put data into fitAI** except raw HTTP
calls. The Flutter client is the answer, but every logger screen needs the same
plumbing — authentication, token storage, an HTTP layer, navigation, error
handling. Building that plumbing inside the first feature screen would couple
unrelated concerns, bloat that PR, and force the architect to review the app's
foundational architecture and a feature at the same time. Carving the shell into
its own requirement keeps each piece reviewable and sets the idiom (state, HTTP,
routing) that the four following M3 screens inherit.

## 3. Acceptance criteria

- **AC1.** From the app, a new user can **register** (calls `POST /auth/register`);
  on success the app proceeds to an authenticated session (auto-login via a
  follow-up `POST /auth/login`, since `register` returns no token). Server
  validation failures (`400` bad email / weak password, `409` duplicate email)
  surface as readable, field-aware messages; nothing crashes.
- **AC2.** A user can **log in** (calls `POST /auth/login`); the returned JWT is
  persisted to **platform secure storage** (iOS Keychain / Android Keystore via
  `flutter_secure_storage`), and the app routes to the **authenticated home
  shell**. Wrong credentials (`401`) show a non-enumerating "invalid email or
  password" message.
- **AC3.** On **cold start**, if a token is present in secure storage the app
  opens directly to the home shell; otherwise it opens to the login screen.
- **AC4.** Every authenticated API call attaches `Authorization: Bearer <token>`.
  The home shell shows the **current user** fetched from `GET /auth/me`.
- **AC5.** Any authenticated request that returns **`401`** clears the stored
  token and routes the user back to login (**re-login on 401** — there is no
  refresh-token flow; the backend has no refresh endpoint).
- **AC6.** A **logout** action clears the stored token and returns to login; a
  subsequent cold start opens to login (AC3).
- **AC7.** The **API base URL is configurable at build time**
  (`--dart-define=API_BASE_URL=…`) with a development default
  (`http://localhost:8080`); no production URL is hard-coded.
- **AC8.** Application state is managed with **Riverpod**, and the
  authenticated-session state is a **single source of truth** that the router's
  redirect gate reads (no duplicated "am I logged in?" logic).
- **AC9.** Loading and error states are represented **uniformly**: no raw
  exception or stack trace reaches the user; network/timeout failures show a
  retryable message; in-flight auth calls show a busy indicator and disable
  double-submit.
- **AC10.** **Tests:** widget tests cover the login screen, the register screen,
  and the auth redirect gate; at least one `integration_test` drives
  **login → home** end-to-end against a mocked HTTP layer. The mobile gates are
  green: `flutter analyze`, `dart format --set-exit-if-changed .`, `flutter test`.
- **AC11.** **No feature logger UI** (workout, nutrition, photo, dashboard) ships
  in this requirement — the authenticated shell (a placeholder home showing the
  user + logout) is the only screen beyond register/login. Those features are
  R-0008+.

## 4. Constraints & non-goals

- **No feature loggers / dashboard** — onboarding (R-0008), workout logger
  (R-0009), nutrition logger (R-0010), dashboard (R-0011), progress-photo capture
  (re-homed; gated on the photo backend R-0006). The shell only.
- **No token refresh / silent session extension** — re-login on `401` (owner
  decision); a refresh-token flow would be a separate backend + mobile R.
- **No OAuth2 / social login** — email + password only (deferred, per R-0002).
- **No biometric unlock, no offline mode / local caching / sync queue** — online
  only for the MVP.
- **No push notifications, no deep links.**
- **No release-style signed builds / store config** — CI stays `flutter analyze`
  + `flutter test`; device-build platform folders and signing are R-0025/R-0026.
- **Thin client only** — no on-device inference or business logic beyond
  presentation and call orchestration (per `project-specifics.md`).

## 5. Open questions

Settled in the step-1 discussion (folded into §3/§6); none blocking `Accepted`:

- **OQ1 — Shell as its own requirement vs. folded into onboarding?**
  RESOLVED → its own requirement (this file), reviewed independently.
- **OQ2 — State management?** RESOLVED → **Riverpod** (AC8).
- **OQ3 — Token expiry strategy given no backend refresh endpoint?**
  RESOLVED → **re-login on 401**, JWT in secure storage (AC2/AC5).

Deferred to the SPEC-0007 design discussion (HOW, not WHAT — do not block this
requirement): HTTP package choice (`dio` vs `http`), router package (`go_router`
vs hand-rolled), whether to run a full `flutter create .` now to regenerate
`android/`/`ios/` platform folders for emulator builds, and the exact
mocked-vs-real backend approach for the `integration_test`.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-02 | **Carve the Flutter architecture + auth shell into its own requirement before any feature logger.** | Every M3 logger depends on the same plumbing (auth, token storage, HTTP, routing); reviewing it once keeps later screens thin and the architecture cleanly reviewable. Mirrors R-0001 (backend skeleton first). (OQ1) |
| 2026-06-02 | **Riverpod for state management.** | Modern, compile-safe, testable, ergonomic for async API state; sets the idiom for all five M3 screens. (OQ2) |
| 2026-06-02 | **Re-login on `401`; JWT in platform secure storage; no refresh flow.** | Backend issues 24h HS256 tokens with no refresh endpoint (R-0002); re-login is the simplest correct MVP behaviour and needs no backend change. (OQ3) |
| 2026-06-02 | **API base URL configurable via `--dart-define`, dev default `http://localhost:8080`.** | No hard-coded prod URL; flavors deferred to M8. (AC7) |
| 2026-06-02 | **Inserted as R-0007; M3 feature rows renumber (onboarding→R-0008, workout→R-0009, nutrition→R-0010, dashboard stays R-0011); progress-photo capture re-homed onto the photo-backend gate (R-0006).** | Avoids rotating 31 committed cross-references to R-0012+ that a full cascade would cause; SPEC-0002 already frames R-0007 as where the Flutter client begins. |

## Changelog

- _2026-06-02 — created (Draft). First M3 / first-mobile requirement: the Flutter app architecture + auth shell. Three step-1 decisions captured (own requirement; Riverpod; re-login on 401)._
- _2026-06-02 — **Accepted.** Owner accepted AC1–AC11 and the M3 renumber (onboarding→R-0008, workout→R-0009, nutrition→R-0010, dashboard stays R-0011; progress-photo capture re-homed onto the photo-backend gate R-0006). Next: step 2 — write SPEC-0007 and the architect design review._
