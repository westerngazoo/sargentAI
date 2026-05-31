# R-0003 — User profile CRUD

- **Status:** Accepted
- **Milestone:** M1
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-05-30
- **Depends on:** R-0002 (Done)
- **Realized by:** SPEC-0003 (to be written once this R is `Accepted`)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

An authenticated user can **create, read, and replace** their own fitness
profile. A profile holds the physiological and goal data the ML layer needs as
inputs: **date of birth** (age is derived), **height** (cm), **weight** (kg),
an **optional biological sex**, an **optional body-fat percentage**, and one or
more **training goals** drawn from a controlled set.

The profile is a **1:1 resource** keyed by the authenticated `user_id` from
R-0002. It is exposed at a single path, `/profile/me`, with `GET` (read) and
`PUT` (create-or-replace / upsert). Every route is protected by the R-0002
`Bearer` JWT middleware and is scoped to the caller — a user can only ever read
or write their own profile.

This requirement **introduces the `crates/core` profile domain types** that
SPEC-0001 §2.2 and R-0002 §4 deferred to R-0003: the `Profile` aggregate and
its validated value types (`Goal`, `Sex`, and metric measurement newtypes) live
in the pure `core` crate with no `sqlx`/`axum`/HTTP dependencies, mirroring the
`Email`/`UserId`/`User` layering established in R-0002.

## 2. Rationale

The ML response-inference engine (M5) and archetype matching (M4) consume
structured per-user attributes — age, sex, body metrics, and goals — as the
**prior** before enough logs exist to personalise. Without a profile there is
nothing to match a new user to a bodybuilder archetype, and no baseline against
which to measure body-composition change. R-0003 is the smallest slice that
captures those attributes behind authentication.

`PUT`-upsert (rather than separate `POST`/`DELETE`) is chosen because the
resource is 1:1 with the user: there is exactly zero or one profile per account,
so create and replace collapse into one idempotent operation. Account deletion
(which would also remove the profile) is an M8 privacy concern (R-0024), not
part of R-0003.

Storing **date of birth** rather than a literal age means the value never goes
stale; age is computed at read time. Persisting **canonical metric** units
keeps the model inputs uniform; any imperial display is a client concern or a
later R.

## 3. Acceptance criteria

Each criterion is observable from a checkout of the R-0003 branch with the
toolchain installed (Rust 1.95.0, Postgres 16 reachable, `JWT_SECRET` set), and
becomes one or more QA tests.

- **AC1.** A SQL migration in `backend/migrations/` creates a `user_profiles`
  table with: `user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE`,
  `date_of_birth DATE NOT NULL`, `height_cm` (whole centimetres) `NOT NULL`,
  `weight_kg` (one-decimal kilograms) `NOT NULL`, `sex TEXT NULL`,
  `body_fat_percentage` (one-decimal percent) `NULL`, `goals TEXT[] NOT NULL`,
  `created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`, and
  `updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`. Running migrations from a
  clean Postgres database succeeds. Deleting a `users` row cascades to its
  profile.

- **AC2.** `GET /profile/me` with a valid `Bearer` token:
  - returns `200 OK` with the caller's profile as JSON when one exists;
  - returns `404 Not Found` when the authenticated user has no profile yet;
  - returns `401 Unauthorized` for a missing/malformed/invalid/expired token
    (delegated to the R-0002 extractor).

- **AC3.** `PUT /profile/me` with a valid `Bearer` token and a JSON body
  `{ "date_of_birth", "height_cm", "weight_kg", "goals", "sex"?, "body_fat_percentage"? }`:
  - returns `201 Created` with the stored profile when the caller had no
    profile (first write);
  - returns `200 OK` with the stored profile when replacing an existing one,
    and bumps `updated_at`;
  - persists exactly one row in `user_profiles` keyed by the caller's `user_id`
    regardless of how many times it is called (upsert, never a duplicate);
  - returns `401 Unauthorized` for a missing/invalid token.

- **AC4.** The profile JSON returned by `GET` and `PUT` carries: `user_id`,
  `date_of_birth` (RFC 3339 date), a **derived** integer `age` in years,
  `height_cm`, `weight_kg`, `sex` (`"male"`/`"female"`/`null`),
  `body_fat_percentage` (or `null`), `goals` (array), `created_at`, and
  `updated_at` (RFC 3339 timestamps).

- **AC5.** Field validation returns `400 Bad Request` (and writes nothing) when:
  - `date_of_birth` is in the future, or implies an age `< 13` or `> 120`;
  - `height_cm` is outside `[50, 300]`;
  - `weight_kg` is outside `[20, 500]`;
  - `body_fat_percentage` is present and outside `[1, 75]`;
  - `goals` is empty, contains a value outside the controlled set, or contains
    duplicates;
  - `sex` is present and not one of `"male"`/`"female"`;
  - any required field (`date_of_birth`, `height_cm`, `weight_kg`, `goals`) is
    missing.

- **AC6.** The `goals` controlled set is exactly:
  `lose_fat`, `build_muscle`, `recomp`, `maintain`, `gain_strength`. It is
  defined once as a `core::Goal` enum and is the single source of truth for both
  request parsing and the JSON shape. (Multi-select: a body may list several,
  e.g. `["build_muscle", "lose_fat"]`.)

- **AC7.** Authorization scoping: a profile is only ever readable/writable by
  its owner. Two distinct authenticated users each operate on their own row;
  user A's `PUT` never mutates user B's profile, and A's `GET` never returns B's
  data. (No cross-user identifier is accepted in the path or body — the subject
  is always the token's `sub`.)

- **AC8.** The `core` crate gains the profile domain (`Profile` aggregate plus
  `Goal`, `Sex`, and metric measurement value types) with **unit tests** for
  every validation rule in AC5/AC6, and remains free of `sqlx`/`axum`/HTTP
  dependencies (the R-0002 purity boundary is preserved).

- **AC9.** At least **ten** integration tests (`#[sqlx::test]`, the R-0002
  harness pattern) cover the surface above, including: `GET` before any profile
  (404), `PUT` first-time (201), `PUT` replace (200 + `updated_at` bump),
  `GET` after write (200 + correct body incl. derived `age`), each `400`
  validation branch from AC5, unauthorized `GET`/`PUT` (401), and the
  cross-user isolation case from AC7.

## 4. Constraints & non-goals

**In scope (R-0003):**
- The `user_profiles` table, the `GET`/`PUT /profile/me` endpoints, the `core`
  profile domain types, and their validation, as described in §3.
- Reuse of the R-0002 `AuthenticatedUser` extractor for all profile routes
  (no new auth machinery).

**Out of scope (deferred):**
- **Tape/circumference measurements** (waist, chest, arm, thigh …) and
  time-series body-measurement history — these belong to the **logging core**
  (M2, R-0005+), not the static profile.
- **Progress photos / photo-derived body-comp proxy** — M6 (R-0010+).
- **Archetype matching / initial-program assignment** — M4; R-0003 only stores
  the inputs that matching will later consume.
- **Imperial units / unit-system conversion** — separate R if the product ever
  needs it; R-0003 stores and accepts canonical metric only.
- **Profile / account deletion** (and GDPR export) — M8 privacy work (R-0024).
  The `ON DELETE CASCADE` foreign key only ensures referential integrity when a
  user is removed by that future flow.
- **`PATCH` / partial update** — `PUT` is full-replace upsert; partial updates
  earn their own treatment only if a real need appears.
- **Multiple profiles per user** — the resource is strictly 1:1.

## 5. Open questions

None remaining — OQ1–OQ4 settled with the owner 2026-05-30 (below).

- **OQ1 — Units.** Metric-only (kg, cm), or accept a unit-system field and
  convert at the boundary? **Settled 2026-05-30: metric-only.** (§6)
- **OQ2 — Goals shape.** Single goal, multi-select, or free text?
  **Settled 2026-05-30: multi-select from a controlled enum.** (§6)
- **OQ3 — Biological sex.** Include (required), include (optional), or omit?
  **Settled 2026-05-30: include, optional/nullable.** (§6)
- **OQ4 — API shape.** `GET`+`PUT` upsert vs full REST?
  **Settled 2026-05-30: `GET` + `PUT` upsert on `/profile/me`.** (§6)

Implementation-level questions — exact SQL numeric types, `goals` as a Postgres
`TEXT[]` vs a join table, the validation crate/approach, and per-test DB
isolation (the R-0002 `#[sqlx::test]` pattern is the presumed default) — are
deferred to SPEC-0003, where they belong.

## 6. Decision log

Decisions made together (owner + Claude). Append-only.

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-05-30 | **Units: metric-only (kg, cm).** No unit-system field; imperial is a future client/R concern. | Target market (Mexico/LATAM) is metric; uniform canonical inputs keep ML features simple. Owner-approved (OQ1). |
| 2026-05-30 | **Goals: multi-select from a fixed enum** (`lose_fat`, `build_muscle`, `recomp`, `maintain`, `gain_strength`). | Structured values feed archetype matching / priors; multi-select reflects real users (e.g. recomp = build + cut). Owner-approved (OQ2). |
| 2026-05-30 | **Biological sex: included but optional/nullable** (`male`/`female`). | Physiological-response models and bodybuilder archetypes are sex-specific (improves priors), but users may decline; ML handles nulls. Health-data privacy applies at M8. Owner-approved (OQ3). |
| 2026-05-30 | **API: `GET` + `PUT` upsert on `/profile/me`.** No separate `POST`/`DELETE`. | Resource is 1:1 with the user; create and replace collapse into one idempotent op. Owner-approved (OQ4). |
| 2026-05-30 | **Store `date_of_birth`; derive `age` at read time.** | Age never goes stale; DOB is the durable fact. Claude-proposed engineering call. |
| 2026-05-30 | **`PUT` returns `201` on first create, `200` on replace.** | Observable distinction between create and update for clients and tests, without a separate `POST`. Claude-proposed. |
| 2026-05-30 | **Minimum age 13 (`date_of_birth` validation).** | Conservative floor aligned with common minor-data thresholds; avoids storing data on young children pre-legal-review (M8). Claude-proposed; revisit at M8. |
| 2026-05-30 | **"Body stats" scoped to optional `body_fat_percentage` in R-0003.** Tape/circumference measurements deferred to logging core (M2). | Keeps the static profile small; recurring measurements are time-series log data, not profile fields. Claude-proposed (§4). |
| 2026-05-30 | **Profile domain types live in `crates/core`** (`Profile`, `Goal`, `Sex`, metric newtypes), preserving the R-0002 purity boundary. | Realises the SPEC-0001 §2.2 / R-0002 §4 trigger; keeps validation pure and HTTP/DB-free. Claude-proposed. |

## Changelog

- _2026-05-30 — created (Draft); OQ1–OQ4 settled with owner the same day; five derived engineering decisions recorded._
- _2026-05-30 — owner acked acceptance criteria; status → Accepted. SPEC-0003 may begin._
