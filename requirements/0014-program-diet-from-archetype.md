# R-0014 — Program + diet generation from matched archetype

- **Status:** Accepted
- **Milestone:** M4 (Archetype prior) — the differentiator fast-track
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-06-18
- **Depends on:** R-0013 (Done — produces the ranked archetype list this reads),
  R-0012 (Done — provides the program/diet templates this instantiates),
  R-0007 (Done — Flutter auth shell)
- **Realized by:** SPEC-0014 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

After archetype matching (R-0013), the user is presented with the **top-3
closest archetypes** returned by that ranking. For each candidate, the backend
generates a **proposed training program and diet** instantiated from that
archetype's templates. The user **picks one**; the choice is persisted as an
active `UserProgram` in a new **`user_programs`** table. A Flutter screen drives
the presentation and selection flow.

This is the "the AI gives you a starting program" step of the differentiator:
it consumes R-0013's ranking, materialises each archetype's templates into
concrete week-1 plans, lets the user own the decision, and gives R-0027 (earbud
session driver) its first structured program to speak.

Generation is **template-instantiation**, not ML inference — the archetype's
`program_template` and `diet_template` (R-0012) are expanded into a
`UserProgram` value (concrete exercises, sets/reps/intensity, macros) using
lightweight parameterisation (user profile: body-weight for load priors, goal
for macro split). The statistical/ML loop is M5; this step lays the data the M5
model will later read and update.

## 2. Rationale

R-0013 ranks archetypes but deliberately stops before any persistence or program
generation (R-0013 AC9 scope guard). R-0014 is the next link: it turns a
ranking into a real choice the user owns, and a real row the system can act on.
Persisting the chosen program in `user_programs` creates the structured record
that:

- **R-0027** reads to drive earbud-guided sessions (exercise name, sets, weight,
  rest) — the immediate downstream dependent.
- **R-0015–R-0017** (M5) aggregate over time to learn per-user response and
  update the program — the longer-term downstream.

Presenting top-3 (not just top-1) keeps the user in the loop: the match is a
prior, not a verdict. If the top-1 archetype feels wrong, the user chooses
differently, and that signal is itself informative for future personalisation.

## 3. Acceptance criteria

- **AC1. Proposals endpoint.** `GET /photo-sessions/:session_id/program-proposals`
  (authenticated) returns the **top-3** archetypes from R-0013's ranking for
  that session, each expanded into a **proposed program and diet** derived from
  its templates. Response includes: archetype user-facing fields (R-0012 wire
  contract — no `internal_name`/`sources`), a `score`/`distance` from the match,
  and a `proposed_program` + `proposed_diet` object. The endpoint is read-only
  and idempotent; re-calling it re-derives from the stored photo session (no
  server-side state written).

- **AC2. Template instantiation.** Each proposal's `proposed_program` and
  `proposed_diet` is derived from the archetype's `program_template` /
  `diet_template` with at least **body-weight and goal** from the user profile
  applied as parameters (e.g. initial load as a percentage of body-weight for
  compound lifts; macro split shifted toward the user's stated goal). Derivation
  is deterministic and unit-tested from fixed archetype + profile inputs.

- **AC3. Choose endpoint.** `POST /programs` (authenticated, JSON body:
  `{photo_session_id, archetype_id}`) persists the user's choice as a new
  **`UserProgram`** row and returns it as `201`. The row stores: user id,
  archetype id, the full generated program + diet (JSONB), `chosen_at`
  timestamp, and an `active` flag. Choosing a new program **deactivates** any
  previously active program for that user (only one active at a time). `409` if
  the `(photo_session_id, archetype_id)` pair is not among the top-3 proposals
  for that session.

- **AC4. Current program endpoint.** `GET /programs/me/current` (authenticated)
  returns the user's current active `UserProgram` (`200`), or `404` if none
  exists yet.

- **AC5. Program history endpoint.** `GET /programs/me` (authenticated) returns
  the user's full program history (all rows, newest first), including inactive
  ones. Supports at minimum `limit`/`offset` pagination.

- **AC6. Ownership & isolation.** All endpoints are scoped to the token's `sub`.
  A user can only access proposals for **their own** photo sessions and can only
  choose an archetype from **their own** session's proposals — cross-user
  attempts return `404` (never `403`). `401` for unauthenticated requests.

- **AC7. Database migration.** A `sqlx` migration creates the `user_programs`
  table with appropriate columns, foreign-key constraints (`user_id` →
  `users`, `archetype_id` stored as the slug string), and index on
  `(user_id, active)`.

- **AC8. Flutter — proposals screen.** After a successful match (`POST
  /photo-sessions/:id/match`), the app navigates to a **Program Proposals
  screen** showing the top-3 cards. Each card displays: archetype name,
  match score (or a human-readable label — "closest", "close", "possible"),
  a summary of the proposed program (training days/week, representative
  exercises), and a summary of the proposed diet (approx calories,
  protein/carb/fat split). The user taps a card to expand the full proposal
  and a **"Choose this program"** button to confirm.

- **AC9. Flutter — program detail screen.** Confirming a choice calls `POST
  /programs`, then navigates to a **Program Detail screen** showing the full
  active program (all exercises, sets, reps, weight guidance) and diet plan.
  The screen is also reachable from a persistent entry point (e.g. home screen
  shortcut) via `GET /programs/me/current`.

- **AC10. Tests.** Backend: unit tests for template instantiation (AC2);
  integration tests for each endpoint — proposals `200`, choose `201` and
  deactivates previous, duplicate-choose `409`, cross-user `404`, `401`.
  Flutter: widget tests for the proposals screen (card rendering, tap-to-expand,
  confirm) and the program detail screen. All gates green (`cargo fmt`/`clippy`/
  `test`/`build`, `flutter analyze`/`test`/`dart format`).

- **AC11. Scope guard.** R-0014 is **template instantiation and user choice
  persistence**. No ML inference (M5), no program adjustment from logs (M5), no
  earbud session driving (R-0027), no nutrition-log UI (R-0010), no progress
  photo analytics (R-0018/R-0019). The famous-athlete data stays the prior; this
  requirement reads `library()` and the user's profile — it does not modify
  either.

## 4. Constraints & non-goals

- **No ML inference** — program generation is template-based parameterisation;
  the statistical learning loop is M5.
- **No on-device inference or local program computation** — the backend owns
  generation; the Flutter client is thin (display + one POST).
- **No archetype editing or creation** — R-0012's library is read-only here.
- **No billing/gating** — freemium features are M7 (R-0021/R-0022).
- **No earbud driving** — the `UserProgram` row is the seam R-0027 reads; R-0027
  itself is out of scope here.
- **Famous names never cross the wire** — the R-0012 wire-contract rule
  (`internal_name`/`sources` omitted) applies to all responses.

## 5. Open questions

Settled in this step-1 discussion (owner, 2026-06-18):

- **OQ1 — Top-N count?** RESOLVED → **top-3** closest archetypes (the owner
  confirmed "let the user pick"). (AC1/AC3)
- **OQ2 — Persistence?** RESOLVED → **new `user_programs` table**, one active
  row per user at a time. (AC3/AC7)
- **OQ3 — Scope?** RESOLVED → **both backend (Rust) and Flutter**. (AC8/AC9)

Deferred to SPEC-0014:

- **OQ-H1 — Template parameterisation detail:** exact formula for load priors
  (% of bodyweight per lift, or per goal), macro-split deltas by goal, and
  which profile fields are used. (AC2)
- **OQ-H2 — Proposals endpoint design:** does it re-run R-0013 matching inline
  or cache the ranking from the match call? (AC1)
- **OQ-H3 — `proposed_program` / `proposed_diet` wire shape:** concrete JSON
  schema (weekly split structure, sets/reps/intensity encoding, macro object).
  (AC1/AC3)
- **OQ-H4 — `409` vs. `422` semantics** for "archetype not in the top-3 for
  this session." (AC3)
- **OQ-H5 — Flutter navigation:** where exactly the proposals screen sits in the
  existing Riverpod router — whether it's a post-match push or a standalone
  route reachable from home. (AC8/AC9)

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-18 | **Top-3 closest archetypes, user picks.** | Keeps the user in the loop; match is a prior, not a verdict; top-3 surfaces meaningful variation given the 6-archetype library. (OQ1) |
| 2026-06-18 | **`user_programs` table, one active row per user.** | Durable record needed by R-0027 (session driver) and M5 (response model); single active program simplifies downstream reads. (OQ2) |
| 2026-06-18 | **Both backend and Flutter in one requirement.** | The backend API and the Flutter screen are tightly coupled (both are thin — a proposals screen + one POST); splitting would leave an unused API or a broken screen. (OQ3) |

## Changelog

- _2026-06-18 — created and **Accepted**. Step-1 discussion settled three forks
  (top-3 / user picks; `user_programs` table; backend + Flutter). Six HOW-level
  questions deferred to SPEC-0014 (template parameterisation, proposals endpoint
  caching, wire shape, 409 vs 422, Flutter navigation)._
