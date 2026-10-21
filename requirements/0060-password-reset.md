# R-0060 — Forgotten password: reset by email

- **Status:** Draft
- **Milestone:** M1 (auth). Pulled forward by the beta (owner, 2026-10-21).
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-10-21
- **Depends on:** R-0002 (email/password auth, Done), which deferred this
  flow to its own requirement (R-0002 §4)
- **Realized by:** SPEC-0060 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

A user who forgot their password can ask for a reset link by email, open
it, and set a new password. The link is single-use and short-lived. The flow
never reveals whether an email has an account. Email is sent through
Cloudflare Email Service.

## 2. Rationale

- **The beta started on 2026-10-21, and today a forgotten password is a dead
  end.** No path exists, so the only way out is a new account under another
  address. The owner hit this on the first day.
- **This is a known pattern.** The flow follows the OWASP Forgot Password
  Cheat Sheet rather than an invented one.
- **Cloudflare Email Service** (owner decision, 2026-10-21). The domain, DNS
  and both deployments already live on Cloudflare. Sending is in public beta
  on Workers Paid, the plan the API container already needs, with 3,000
  messages a month included. It is reachable by a REST endpoint, so the
  backend needs no new crate (`reqwest` is already a dependency).

## 3. Acceptance criteria

- **AC1. Request a reset.**
  - An unauthenticated endpoint takes an email address.
  - Whether or not an account exists, the response is identical in status
    code, body, and order of magnitude of time.
  - If an account exists, a reset email is sent.
- **AC2. The token.**
  - At least 256 bits from a CSPRNG, sent only inside the email link.
  - Stored only as a hash, never in plain text, and never logged.
  - Expires 30 minutes after issue, unless SPEC-0060 justifies another
    value.
  - Single use. A newer request invalidates any earlier unused token for the
    same account.
- **AC3. Set the new password.**
  - An unauthenticated endpoint takes the token and a new password.
  - The password passes the same validation as registration (today a
    minimum of 8 characters, `auth/handlers.rs`) and is hashed with argon2id
    like every other password.
  - A token that is unknown, expired, already used or superseded is refused
    with one generic error. The response does not say which case it was.
- **AC4. Existing sessions end.** After a reset, JWTs issued before it are
  no longer accepted. Sessions are stateless 24-hour JWTs today (R-0002), so
  this needs a per-user marker the token check reads. SPEC-0060 designs it.
- **AC5. Rate limits.** Reset requests are limited per email address and per
  client address. Going over the limit is answered like an ordinary request,
  so the limit does not reveal that an account exists.
- **AC6. The email.**
  - Sent as **Goose Fitness Physics** `<no-reply@goosethropic.systems>`,
    a verified sender.
  - Contains the link, the expiry, and a line saying the message can be
    ignored if the reader did not ask for it.
  - Contains no other account data.
  - A send failure is logged without the token or the recipient's address,
    and is not reported to the requester (AC1).
- **AC7. The app.** The login screen gets a "Forgot password?" link. The app
  has:
  - a screen to request the email, which shows a neutral confirmation;
  - a screen the emailed link opens, where the new password is set;
  - a clear message for a link that is no longer valid, with a way to
    request a new one.

  The link works in the web build, the beta's surface. Deep-linking into the
  installed mobile app is a non-goal for v1.
- **AC8. Secrets.** The Email Service credential is a Worker or container
  secret set out of band, like `DATABASE_URL` and `JWT_SECRET`. It is never
  in the repository, CI logs or `.env` files; `.env.example` gets a
  placeholder.
- **AC9. Tests.**
  - Unit tests: token generation, hashing, expiry and single use.
  - Integration tests:
    - an existing and a missing email give the same response;
    - expired, reused and superseded tokens are refused;
    - old JWTs are refused after a reset;
    - the rate limit applies.
  - One adversarial test per trust boundary, at minimum:
    - token guessing is refused;
    - enumeration by timing stays within a stated bound.
  - Email sending is behind a seam with a fake in tests. CI never sends real
    mail.

## 4. Constraints & non-goals

- **No password change from inside the app.** Changing the password while
  logged in is a separate, smaller flow.
- **No SMS and no security questions.**
- **No account recovery when the email is lost.** That is manual support
  for the beta.
- **Accounts made with Google sign-in** are out of scope until OQ-2 is
  answered.
- **Privacy:** reset tokens and send logs join R-0024's data inventory
  before launch.

## 5. Open questions

- **OQ-1. How the backend reaches Email Service.** *Resolved 2026-10-21
  (owner): the REST API, with a scoped API token held as a container secret
  (AC8).* The rejected alternative was the Worker's `EMAIL` binding, called
  through the fronting Worker: no credential, but a new internal hop between
  the container and the Worker.
- **OQ-2. A Google-only account asks for a reset.** It has no password.
  Should it get a "sign in with Google" email or nothing at all? Either way
  the response to the requester stays identical (AC1).
- **OQ-3. Sender address.** *Resolved 2026-10-21 (owner):* **Goose
  Fitness Physics** `<no-reply@goosethropic.systems>`. Verifying the domain
  is an owner step in the Cloudflare dashboard.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-21 | Reset by emailed single-use link, following OWASP's Forgot Password Cheat Sheet | A known pattern for a security boundary, not an invented one. |
| 2026-10-21 | Cloudflare Email Service sends the mail | Owner. Infra already on Cloudflare; covered by the Workers Paid plan the API needs; reachable by REST with no new crate. |
| 2026-10-21 | The sender's display name is "Goose Fitness Physics" | Owner: the app's official name going forward. |
| 2026-10-21 | Email Service is reached by REST; the sender is `no-reply@goosethropic.systems` | Owner. REST keeps the container independent of the Worker; the token is a secret set out of band (AC8). |
