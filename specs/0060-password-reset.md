# SPEC-0060 — Forgotten password: reset by email

- **Status:** Accepted — architect review ACCEPT WITH CHANGES (2026-10-21), all required changes applied (§10)
- **Realizes:** R-0060
- **Author:** Claude (main session), with owner
- **Created:** 2026-10-21
- **Depends on:** SPEC-0002 (auth: `users`, argon2id, HS256 JWT, `AuthenticatedUser`), SPEC-0007 (mobile auth screens and `AuthInterceptor`)
- **Module(s):** `backend/crates/api/{auth,mail,db,error}`, `backend/migrations/`, `infra/cloudflare/` (env plumbing), `mobile/lib/src/auth/`, `mobile/lib/src/router/`, `mobile/lib/src/core/network/`

## 1. Motivation

Realizes [R-0060](../requirements/0060-password-reset.md). A user who forgot
their password asks for a link by email, opens it in the web app and sets a
new password. The flow follows the OWASP Forgot Password Cheat Sheet. Mail
goes out through Cloudflare Email Service's REST API (owner decisions,
2026-10-21: REST, sender `Goose Fitness Physics <no-reply@goosethropic.systems>`,
Google-only accounts get nothing).

The design has three parts, and two of them are seams the codebase already
uses:

1. **A token table and two endpoints** in `auth`, in the style of
   `register`/`login`.
2. **A `Mailer` trait** in a new `mail` module, with a Cloudflare
   implementation and a recording fake. This mirrors `GoogleIdTokenVerifier`
   (SPEC-0033): the production implementation talks to the network, tests
   construct the fake, and `AppState` carries it inside a `MailSettings`
   shaped like `GoogleAuthSettings`.
3. **Session invalidation** by comparing the JWT's `iat` against a new
   `users.password_changed_at`, inside the per-request lookup the extractor
   already does.

## 2. Design

### 2.1 Schema (migration `00011_password_reset.sql`)

```sql
-- R-0060 / SPEC-0060 §2.1 — password reset tokens, and the marker that ends
-- sessions issued before a reset (AC4).

ALTER TABLE users ADD COLUMN password_changed_at TIMESTAMPTZ;

CREATE TABLE password_reset_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  BYTEA NOT NULL UNIQUE,      -- SHA-256 of the token, never the token
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ NOT NULL,
    used_at     TIMESTAMPTZ
);
CREATE INDEX password_reset_tokens_user_id ON password_reset_tokens(user_id);

-- Rate-limit ledger (AC5). `key_hash` is SHA-256 of "email:<normalized>" or
-- "ip:<address>", so the ledger stores neither emails nor addresses. Rows
-- older than the one-hour window are purged by the handler on every insert.
CREATE TABLE password_reset_requests (
    key_hash     BYTEA NOT NULL,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX password_reset_requests_key ON password_reset_requests(key_hash, requested_at);
```

`password_changed_at` is nullable: NULL means "never changed", so every
existing token stays valid and the migration is additive (`run_migrations`'s
rollback note in `lib.rs` holds).

### 2.2 The token (AC2)

- 32 bytes from `OsRng`, reached as `argon2::password_hash::rand_core::OsRng`
  the way `password.rs:4` already does, hex-encoded by hand
  (`format!("{byte:02x}")`): 64 characters, URL-safe without padding, no
  encoding crate.
- Stored as its **SHA-256**. Lookup is by hash, so the store needs a fast,
  unsalted digest; argon2 would not allow a lookup. This declares `sha2` in
  `[workspace.dependencies]` like every other crate; it is already in
  `Cargo.lock` (0.10, transitive), so the addition costs nothing. The
  alternative rejected: reaching `ring::digest` through `jsonwebtoken`'s
  transitive dependency, which could vanish on an upgrade.
- TTL 30 minutes (R-0060 AC2). A constant `RESET_TOKEN_TTL` in `auth::reset`.
- Single use and superseded, by **one mechanism**: issuing a token deletes
  the user's unused tokens first, and consuming one is a single conditional
  `UPDATE … RETURNING` (§2.4). There is no second delete at consumption.

```rust
// crates/api/src/auth/reset.rs — pure helpers, unit-tested without a database
pub(crate) struct ResetToken(String);           // the plaintext, lives only in the email
pub(crate) struct ResetTokenHash(Vec<u8>);      // what the database stores

impl ResetToken {
    pub(crate) fn generate() -> Self;            // 32 bytes OsRng → 64 hex chars
    pub(crate) fn hash(&self) -> ResetTokenHash; // SHA-256
    pub(crate) fn parse(s: &str) -> Option<Self>;// exactly 64 lowercase hex chars, else None
}
```

`Debug` is implemented by hand on `ResetToken` and prints `ResetToken(..)`,
so no `tracing` line can leak it by accident (R-0002's no-plaintext rule,
extended).

### 2.3 The mailer seam (AC6, AC8, AC9)

```rust
// crates/api/src/mail/mod.rs
pub struct Message {
    pub to: String,
    pub subject: String,
    pub text: String,
    pub html: String,
}

#[derive(Debug, thiserror::Error)]
pub enum MailError {
    #[error("mail transport error")]      Transport,   // network, timeout, 5xx, 429
    #[error("mail rejected by provider")] Rejected,    // other 4xx
    #[error("mailer not configured")]     NotConfigured,
}

#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send(&self, message: Message) -> Result<(), MailError>;
}

pub struct CloudflareMailer { client: reqwest::Client, account_id: Arc<str>, token: Arc<str>, from: Arc<str> }
pub struct UnconfiguredMailer;                                  // every send → NotConfigured

/// Test double. `sent` is what was delivered; `wait_for(n, timeout)` resolves
/// once at least `n` messages are recorded, so a test never races the
/// spawned send (§2.4).
pub struct RecordingMailer { sent: Mutex<Vec<Message>>, notify: tokio::sync::Notify }

/// Carried by `AppState`, shaped like `GoogleAuthSettings`: a configured
/// mailer never exists without the URL its links point at.
#[derive(Clone)]
pub struct MailSettings { pub mailer: Arc<dyn Mailer>, pub public_web_url: Arc<str> }
impl Default for MailSettings { /* UnconfiguredMailer, "http://localhost:9500" */ }
```

`CloudflareMailer::send` does
`POST https://api.cloudflare.com/client/v4/accounts/{account_id}/email/sending/send`
with `Authorization: Bearer <token>` and the body
`{"from","to","subject","text","html"}`; a 2xx with `success: true` is `Ok`,
429 and 5xx and any timeout are `Transport`, other 4xx are `Rejected`. The
client is built with a **10-second timeout**, as `google.rs` builds its own
with 5 s: an unresponsive provider must not pin a task forever. The token is
held in an `Arc<str>` that is never formatted; `CloudflareMailer`'s `Debug`
prints only the account id.

Configuration, in `MailSettings::from_env()` following
`VoiceIntentSettings::from_env()`:

| Variable | Meaning | Kind | Default |
|---|---|---|---|
| `CF_ACCOUNT_ID` | Cloudflare account | var | none |
| `CF_EMAIL_API_TOKEN` | Email Service send token | **secret** | none |
| `MAIL_FROM` | sender | var | `Goose Fitness Physics <no-reply@goosethropic.systems>` |
| `PUBLIC_WEB_URL` | origin the reset link points at | var | none |

With `CF_ACCOUNT_ID`, `CF_EMAIL_API_TOKEN` and `PUBLIC_WEB_URL` all set,
the state gets a `CloudflareMailer`; otherwise `MailSettings::default()`,
and `main` logs one `warn` at boot saying password reset is disabled. The
forgot endpoint still answers 202 either way (AC1), and the send failure is
logged as `MailError::NotConfigured` without the recipient (AC6).

**Reaching the container.** The Worker in `infra/cloudflare/src/index.ts`
hands the container only the variables it enumerates in `envVars`. So this
spec also adds the four variables to that `Env` interface and `envVars`,
`CF_EMAIL_API_TOKEN` as a Worker secret (`wrangler secret put`, like
`JWT_SECRET`) and the other three as `vars` in `wrangler.jsonc`, and lists
them in `infra/cloudflare/README.md`. Without this step production would
run with `UnconfiguredMailer` and nobody would notice until a tester asked.
`.env.example` gets the four variables with placeholders.

**No local "print the link" mailer.** It would be a path where the token
reaches a log, the exact thing AC2 forbids, and a flag that keeps it out of
production is a flag that can be set wrong. Local testing uses the recording
fake through the test suite, or a real Cloudflare token in `backend/.env`.

### 2.4 Endpoints

Both are unauthenticated and live in `auth::handlers`, routed from
`auth::routes()`. The existing `body()` helper is generalized over
`T: DeserializeOwned + Validate` instead of growing a copy per request type.

#### `POST /auth/password/forgot` → `202 Accepted`, empty body (AC1, AC5)

```
{ "email": "..." }
```

1. Parse; `Email::parse` normalizes. A malformed body or email is
   `400 validation`. This reveals nothing about accounts.
2. **Rate limit (AC5).** One statement per key, a CTE that purges rows older
   than one hour, inserts this request's row, and returns the key's count
   within the hour:

   ```sql
   WITH purged AS (DELETE FROM password_reset_requests WHERE requested_at < now() - interval '1 hour'),
        added  AS (INSERT INTO password_reset_requests (key_hash) VALUES ($1))
   SELECT count(*) FROM password_reset_requests WHERE key_hash = $1 AND requested_at >= now() - interval '1 hour'
   ```

   Keys: `email:<normalized>` always, and `ip:<address>` when the
   `CF-Connecting-IP` header is present (Cloudflare's edge sets it and the
   Worker forwards the request unchanged; in tests and local runs it is
   absent, and only the email key applies). Insert-then-count is race-safe
   in the direction that matters: each request's own row is committed
   before its count, so concurrent requests can only count each other and
   over-block, never under-block. Over the limit (count `> 3` per email,
   `> 10` per address) the handler **returns 202 now** and does nothing
   else. Purging by age alone means keys that never recur do not
   accumulate; there is one window, the hour that is counted.
3. Look up the user by email. **Three cases, one outcome for the caller:**
   - no account → return 202;
   - Google-only account (`password_hash IS NULL`, OQ-2) → return 202;
   - account with a password → generate a token, delete the user's unused
     tokens, insert the new hash with `expires_at = now + 30 min`, then
     return 202.
4. **The email is sent after the response**, on a `tokio::spawn`ed task
   that holds the `Arc<dyn Mailer>` and the message and is
   `.instrument(tracing::Span::current())`ed so its log lines stay inside
   the request's span (and inside `tracing-test`'s capture, T12). A failure
   is logged at `error` as the `MailError` variant only: no address, no
   token.

   **Shutdown semantics, stated plainly.** `axum::serve(...)
   .with_graceful_shutdown` does not wait for detached tasks. A SIGTERM, or
   Cloudflare's `sleepAfter`, landing within the send's few hundred
   milliseconds drops that one email silently; the user's remedy is to ask
   again, and the token already stored stays valid. A panic inside the task
   is contained by its `JoinHandle` and does not touch the server. This is
   an **accepted loss for v1** (§8); draining a `TaskTracker` on shutdown
   is the upgrade if it ever shows up in practice.

Step 4 is what keeps the three cases' response times alike (AC1): the
network call, the only slow step, is outside the request. What remains in
the request is the ledger statement and one lookup for every case, plus,
for the password case, one `OsRng` read, one SHA-256 and two statements,
which is milliseconds. The timing test in §7 bounds the difference.

#### `POST /auth/password/reset` → `204 No Content` (AC3, AC4)

```
{ "token": "...", "password": "..." }
```

1. Parse; `password` passes the same `#[validate(length(min = 8))]` as
   registration, else `400 validation/password`. `ResetToken::parse` failing
   (wrong length or alphabet) is treated as an unknown token, not a 400, so
   the shape of a valid token is not confirmed by the error.
2. In one transaction, **consume** the token with a single conditional
   statement, atomic without `FOR UPDATE`:

   ```sql
   UPDATE password_reset_tokens SET used_at = now()
   WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()
   RETURNING user_id
   ```

   No row → roll back and **`401`** with the existing `unauthorized` body.
   One variant, one body, for every bad-token case: unknown, malformed,
   used, expired, superseded (AC3). `ApiError::Unauthorized` already
   documents itself as enumeration-safe, and a reset token is a credential,
   so no new error variant is added.
3. Otherwise, in the same transaction: `UPDATE users SET password_hash =
   $argon2, password_changed_at = $now WHERE id = $user`, where `$now` is
   the **app's** `Utc::now()` (§2.5); commit; `204`.

The new password is argon2id-hashed by the existing `password::hash`.

### 2.5 Sessions end on reset (AC4)

`AuthenticatedUser` already fetches the user on every request
(`find_user_by_id`, "AC5: confirm the user still exists"). That lookup now
also returns `password_changed_at`, and the extractor rejects the token when

```
password_changed_at is Some(t) && claims.iat <= t.timestamp()
```

No new claim, no counter, no extra query. The comparison **fails closed**: a
token minted in the same second as the reset is rejected too, because that
second is exactly where an attacker holding the old password would be. The
cost is that a login completed in the same second as a reset has to log in
again, which the UI flow cannot even produce. Both sides of the comparison
come from the app clock: `iat` is `Utc::now()` in `token.rs`, and the reset
handler writes `password_changed_at` from `Utc::now()` rather than
Postgres's `now()`, so clock skew between the two cannot widen or narrow the
window. Tokens issued after the reset have `iat > t.timestamp()` and pass.

`find_user_by_id` keeps returning `core::User`, which stays free of
persistence detail (SPEC-0002 §2.2). The extractor instead calls a new
`db::find_auth_row_by_id` that returns the row with `password_changed_at`,
and `find_user_by_id` remains for everyone else.

### 2.6 The email (AC6)

Subject: `Reset your Goose Fitness Physics password`. Body (text and a
minimal HTML twin), with the link
`{PUBLIC_WEB_URL}/#/reset-password?token=<token>`:

> Someone asked to reset the password for this account. If it was you, open
> this link within 30 minutes: `<link>`. If it wasn't you, ignore this
> message; your password has not changed.

The token sits in the URL **fragment**. The app uses Flutter web's default
hash URL strategy (no `usePathUrlStrategy` anywhere in `mobile/lib`), and
browsers do not send fragments to the server or in `Referer`, so the token
never reaches Cloudflare's or the Worker's request logs. The text is a
`const` in `auth::reset` with the link interpolated, no template engine.

### 2.7 Mobile (AC7)

Routes added to `app_router.dart`: `/forgot-password` and
`/reset-password` (reads `state.uri.queryParameters['token']`).

The redirect, arm by arm, because the first one is where the link would
otherwise die:

- `AuthUnknown` (cold start, which **every link opened from an email is**):
  today every location is sent to `/splash`, and the string `'/splash'`
  discards the query. The arm now passes `/reset-password` and
  `/forgot-password` through unchanged, with their query intact.
- `AuthUnauthenticated`: allows both alongside `/login` and `/register`.
- `AuthAuthenticated`: allows `/reset-password` too. A logged-in user who
  opens the link from another device's email should still be able to use
  it; `/forgot-password` redirects home like `/login` does.

`ResetPasswordScreen` reads the token once in `initState` and then calls
`context.replace('/reset-password')`, so the token leaves the browser's
history and the address bar.

`AuthInterceptor._exemptPaths` gains `/auth/password/reset`: its 401 means
"bad link", not "session expired", and must not log anyone out. The
exemption matches because `requestOptions.path` is the relative path.

`AuthApi` gains:

```dart
/// `POST /auth/password/forgot` → 202. The response is the same whether or
/// not the address has an account.
Future<void> forgotPassword(String email);

/// `POST /auth/password/reset` → 204. 401 → link no longer valid.
Future<void> resetPassword({required String token, required String password});
```

Screens, both built like `LoginScreen` (same `Scaffold`, `BrandHeader`,
420-px column):

- **`ForgotPasswordScreen`**: one email field, "Send reset link". After
  202 the form is replaced by: "If that address has an account, a reset
  link is on its way. It is valid for 30 minutes." and a "Back to log in"
  button. The text is the same for every address.
- **`ResetPasswordScreen`**: new password and confirmation, "Set new
  password". On 204: a snackbar "Password updated. Log in with it." and
  `context.go('/login')`. On 401: the form is replaced by "This link is no
  longer valid. Links expire after 30 minutes and work once." with a
  "Request a new link" button to `/forgot-password`. With no `token` in the
  URL the screen shows that same message straight away.
- **`LoginScreen`**: a `TextButton` "Forgot password?" under the form,
  next to "Create an account".

## 3. Code outline

```
backend/
├── Cargo.toml                     # + sha2 in [workspace.dependencies]
├── migrations/00011_password_reset.sql
└── crates/api/
    ├── Cargo.toml                 # + sha2.workspace
    └── src/
        ├── lib.rs                 # AppState + mail: MailSettings
        ├── main.rs                # MailSettings::from_env()
        ├── mail/mod.rs            # Message, MailError, Mailer, CloudflareMailer, RecordingMailer, UnconfiguredMailer, MailSettings
        ├── db.rs                  # find_auth_row_by_id, reset_request_count, replace_reset_token, consume_reset_token (tx)
        └── auth/
            ├── mod.rs             # + /auth/password/forgot, /auth/password/reset
            ├── handlers.rs        # forgot_password, reset_password; body<T>() generalized
            ├── reset.rs           # ResetToken, ResetTokenHash, RESET_TOKEN_TTL, email text, link builder
            └── extractor.rs       # iat <= password_changed_at → 401

infra/cloudflare/
├── src/index.ts                   # Env + envVars: CF_ACCOUNT_ID, CF_EMAIL_API_TOKEN, MAIL_FROM, PUBLIC_WEB_URL
├── wrangler.jsonc                 # vars: CF_ACCOUNT_ID, MAIL_FROM, PUBLIC_WEB_URL
└── README.md                      # secret list + CF_EMAIL_API_TOKEN

mobile/lib/src/
├── auth/
│   ├── data/auth_api.dart                     # forgotPassword, resetPassword
│   └── presentation/
│       ├── login_screen.dart                  # "Forgot password?"
│       ├── forgot_password_screen.dart
│       └── reset_password_screen.dart
├── core/network/dio_provider.dart             # exempt /auth/password/reset
└── router/app_router.dart                     # two routes, three redirect arms
```

Representative handler:

```rust
pub(crate) async fn forgot_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    req: Result<Json<ForgotRequest>, JsonRejection>,
) -> ApiResult<StatusCode> {
    let req = body(req)?;
    let email = Email::parse(&req.email).map_err(|_| ApiError::Validation { field: "email" })?;

    let client_ip = headers.get("cf-connecting-ip").and_then(|v| v.to_str().ok());
    if db::reset_request_over_limit(&state.pool, email.as_str(), client_ip).await? {
        return Ok(StatusCode::ACCEPTED);
    }

    let Some(row) = db::find_row_by_email(&state.pool, email.as_str()).await? else {
        return Ok(StatusCode::ACCEPTED);
    };
    if row.password_hash.is_none() {
        return Ok(StatusCode::ACCEPTED); // Google-only: nothing to reset (OQ-2)
    }

    let token = ResetToken::generate();
    db::replace_reset_token(&state.pool, UserId(row.id), &token.hash(), RESET_TOKEN_TTL).await?;

    let message = reset::message(&state.mail.public_web_url, &row.email, &token);
    let mailer = state.mail.mailer.clone();
    tokio::spawn(
        async move {
            if let Err(e) = mailer.send(message).await {
                tracing::error!(error = %e, "password reset email not sent");
            }
        }
        .instrument(tracing::Span::current()),
    );
    Ok(StatusCode::ACCEPTED)
}
```

## 4. Non-goals

- Changing the password while logged in.
- Setting a password on a Google-only account.
- Recovery when the email address itself is lost.
- Deep links into the installed mobile app; the link opens the web app.
- Any mail other than this one; the `Mailer` seam is general but only one
  message exists.
- Draining in-flight sends on shutdown (accepted loss, §2.4, §8).

## 5. Open questions

- **OQ-1.** `CF-Connecting-IP` reaching the container is expected (the
  Worker forwards the request unchanged) but not yet observed. Verified
  during implementation by one request against the live API that echoes
  nothing but logs the header's presence at `debug`; recorded in §8. If the
  header is absent in production, the limit is per email only, which the
  design tolerates.
- **OQ-2.** Whether T11 (timing) is stable in CI. If it flakes, it is kept
  as a benchmark run by hand, and the suite keeps the structural assertion
  that every path runs the same statements before answering.

## 6. Acceptance criteria

Mapped from R-0060; each is verified by the tests in §7.

| R-0060 | Realized by | Verified by |
|---|---|---|
| AC1 identical response | §2.4 forgot, steps 3–4 | T1, T2, T3, T11 |
| AC2 token | §2.2 | U1–U4, T4 |
| AC3 reset, generic refusal | §2.4 reset | T5–T8 |
| AC4 sessions end | §2.5 | T9 |
| AC5 rate limits | §2.4 step 2 | T10 |
| AC6 the email | §2.6, §2.3 | T4, T12 |
| AC7 the app | §2.7 | M1–M6 |
| AC8 secrets | §2.3, `infra/cloudflare` | review: `.env.example`, `wrangler.jsonc`, no literal in repo |
| AC9 tests | §7 | — |

## 7. Test plan

Backend unit (`auth/reset.rs`, no database):
- **U1** `generate()` yields 64 lowercase hex chars; two calls differ.
- **U2** `parse` rejects 63 and 65 chars, uppercase, and non-hex.
- **U3** `hash` is stable and equals an independently computed SHA-256 of
  the bytes.
- **U4** `format!("{:?}", token)` does not contain the token.

Backend integration (`tests/password_reset.rs`, `#[sqlx::test]`, app built
with a `RecordingMailer`; every assertion on `sent` is preceded by
`wait_for(n, 2 s)` so nothing races the spawned send):
- **T1** known email → 202, one message recorded, to that address.
- **T2** unknown email → 202, same (empty) body; `wait_for(1, 300 ms)`
  times out, nothing recorded.
- **T3** Google-only account → 202, nothing recorded, no token row.
- **T4** the recorded message contains `#/reset-password?token=` and a token
  whose SHA-256 matches the stored `token_hash`; the message contains no
  other user data.
- **T5** reset with that token and a new password → 204; login with the new
  password → 200; login with the old → 401.
- **T6** the same token a second time → 401.
- **T7** a token whose `expires_at` is moved into the past by SQL → 401.
- **T8** two forgot requests; the first token → 401 (superseded), the second
  → 204. A random 64-hex string → 401. A 10-char string → 401 (not 400).
- **T9** login → JWT; reset; the old JWT on `/auth/me` → 401; a fresh login
  → 200. No clock tricks: the comparison fails closed, so the old token is
  rejected even inside the same second.
- **T10** four forgot requests for one email within a minute → four 202s,
  **three** messages recorded.
- **T11** adversarial, timing: 20 **distinct** registered addresses and 20
  **distinct** unknown ones, one forgot request each, alternating, so no
  request takes the rate-limited early return; the medians differ by less
  than 50 ms. See OQ-2.
- **T12** `#[traced_test]`: with a mailer that always fails, the forgot path
  logs an error that contains neither the address nor the token (the send
  task is instrumented with the request span, so the capture sees it).
- **T13** reset-token rows cascade on user delete (schema check).
- **T14** the ledger: after a request, rows older than an hour for any key
  are gone.

Mobile widget tests (`test/auth/`):
- **M1** login screen shows "Forgot password?" and it navigates to
  `/forgot-password`.
- **M2** forgot screen: submit calls `forgotPassword` with the trimmed
  email and shows the neutral confirmation.
- **M3** reset screen with `?token=abc`: mismatched confirmation blocks
  submit; matching → `resetPassword(token: 'abc', …)`; success → `/login`.
- **M4** reset screen: a 401 from the fake API shows the "no longer valid"
  message and the "Request a new link" button.
- **M5** `AuthInterceptor`: a 401 from `/auth/password/reset` does not call
  `logout()`.
- **M6** cold start: with auth `AuthUnknown` and the initial location
  `/reset-password?token=abc`, the router lands on the reset screen with
  the token once auth resolves to unauthenticated; the same with
  `AuthAuthenticated`.

QA owns the e2e test for R-0060, which runs T1→T4→T5 against a booted
server with the recording mailer. Nothing in CI needs the network: the
recording mailer and `#[sqlx::test]`'s Postgres, as today.

## 8. Decision log

| Date | Decision | Rationale |
|---|---|---|
| 2026-10-21 | `sha2` declared as a workspace dependency | Lookup by hash needs an unsalted digest; already in the lockfile transitively; the alternative (`ring` via `jsonwebtoken`) could vanish. |
| 2026-10-21 | Sessions end by `iat <= password_changed_at`, both from the app clock | Reuses the lookup the extractor already does; no claim change. Fails closed on the same-second case (architect finding 5). |
| 2026-10-21 | Email sent after the response, instrumented with the request span | Keeps the known/unknown timing alike (AC1) without constant-time tricks; keeps its logs in the span. |
| 2026-10-21 | In-flight sends are not drained on shutdown — accepted loss for v1 (owner may veto) | A send takes a few hundred ms; a shutdown inside it loses one email, the token stays valid, the user asks again. `TaskTracker` is the upgrade if it ever shows up. |
| 2026-10-21 | Bad reset token is `401 unauthorized` | A reset token is a credential; the variant is already enumeration-safe; no new error variant. |
| 2026-10-21 | No "print the link" dev mailer | A token in a log is what AC2 forbids; a prod-guard flag can be set wrong. |
| 2026-10-21 | Rate-limit ledger stores hashes only, purged by age in the counting statement | The ledger must count unknown emails too (otherwise the limit itself leaks); hashing keeps addresses out; one window, no accumulation. |
| 2026-10-21 | Consume the token with one conditional `UPDATE … RETURNING` | Atomic without `FOR UPDATE`; one mechanism for single-use and superseded (architect finding 8). |
| 2026-10-21 | `MailSettings` bundles mailer and URL | A configured mailer never exists without its link origin (architect finding 9). |

## 9. Changelog

- 2026-10-21 — Draft for architect review.
- 2026-10-21 — Architect review applied (§10); status Accepted.

## 10. Architect review — ACCEPT WITH CHANGES (2026-10-21)

Twelve findings; every blocking and required change is applied above.

| # | Severity | Finding | Applied in |
|---|---|---|---|
| 1 | blocking | The cold-start `AuthUnknown` redirect to `/splash` discarded the reset link's query, so the emailed link never opened the screen | §2.7 redirect arms; M6 |
| 2 | blocking | Tests asserting on the recording mailer raced the spawned send; `tracing-test` capture did not see the task's logs | §2.3 `wait_for`; §2.4 `.instrument` |
| 3 | blocking | T11 compared rate-limited early returns, proving nothing | §7 T11 uses 40 distinct addresses |
| 4 | blocking | The Worker enumerates the container's env vars, so a secret set with `wrangler` alone never reached the process | §2.3 "Reaching the container"; §3 |
| 5 | required | `iat < t` failed open on the same-second case; app and DB clocks could drift | §2.5 `<=`, app clock on both sides; T9 |
| 6 | required | Ledger purge keyed by address accumulated never-recurring keys; two windows | §2.4 step 2 CTE, purge by age only |
| 7 | required | `reqwest::Client::new()` has no timeout; shutdown semantics unstated | §2.3 10 s timeout; §2.4 shutdown; §8 |
| 8 | required | The delete at consumption duplicated the delete at issue | §2.2, §2.4 single `UPDATE … RETURNING` |
| 9 | required | `public_web_url` as a separate `Option` next to the mailer | §2.3 `MailSettings` |
| 10 | suggestion | `sha2` already transitive; name the `OsRng` path | §2.2 |
| 11 | suggestion | Generalize `body()` instead of copying it | §2.4 |
| 12 | suggestion | Read the token once and `replace` the route | §2.7 |
