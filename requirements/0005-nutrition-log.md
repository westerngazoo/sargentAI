# R-0005 — Nutrition log

- **Status:** Accepted
- **Milestone:** M2
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-05-31
- **Depends on:** R-0002 (Met), R-0003 (Met)
- **Realized by:** [SPEC-0005](../specs/0005-nutrition-log.md) (Accepted)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

An authenticated user can **create, read, edit, and delete** their own daily
**nutrition logs**. A nutrition log is a single **per-day** record keyed by date
(`performed_on`) that captures the three macronutrients the model consumes —
**protein**, **carbs**, and **fat**, each in **grams** — for that day. Total
**calories are derived**, never stored: `calories = 4·protein + 4·carbs +
9·fat`, computed server-side and returned in every response. There is **at most
one** nutrition log per user per date.

Logs are owned by the authenticated `user_id` from R-0002 and scoped to the
caller: a user can only ever read or mutate their own logs. The collection is
exposed under `/nutrition` with the full lifecycle — `POST` (create), `GET`
(list own, newest first), `GET /{id}` (read one), `PUT /{id}` (full-replace
edit), and `DELETE /{id}` (remove). Every route is protected by the R-0002
`Bearer` JWT middleware.

This requirement **introduces the `crates/core` nutrition domain**: the
`NutritionLog` aggregate, its `NewNutritionLog` write model, the `Macros` value
group, and the `Grams` value type live in the pure `core` crate with no
`sqlx`/`axum`/HTTP dependencies, mirroring the profile (R-0003) and workout
(R-0004) domains. Macros are stored **verbatim** in grams; calories are the only
derived field and are computed from those grams by a single `core` authority.

## 2. Rationale

The nutrition log is, alongside the workout log (R-0004), one of the two primary
signals the ML response-inference engine (M5) consumes: daily macros and total
calories are listed directly among the model's input features (source brief, *ML
Model Design → Input Features*: "Macros: protein, carbs, fat, total calories")
and the data model (*NutritionLogs (date, protein, carbs, fat, calories)*). The
adjustment engine (R-0017) recommends changes to **diet macros**, which requires
a faithful per-day record to correlate against body-composition change.

A **flat, per-day** shape (one row per user per date) is chosen — over a
per-meal nested model — because that is exactly the grain the brief specifies and
the grain the model consumes; per-meal breakdown is a later refinement only if
the model demonstrably needs it. **Calories are derived, not stored**, because a
stored calorie field can drift out of agreement with the macros that determine
it; making macros the single source of truth and computing calories on demand
removes a whole class of inconsistency. The 4/4/9 kcal-per-gram convention is the
standard Atwater factor set; fiber and alcohol refinements are out of scope until
the model needs them.

**Full CRUD** (not just append) is in scope because users mis-log days and must be
able to correct or remove them; editing is a **full-replace** of the day's macros
(and optionally its date) via `PUT`, keeping mutation semantics unambiguous —
consistent with R-0004.

## 3. Acceptance criteria

Each criterion is observable from a checkout of the R-0005 branch with the
toolchain installed (Rust 1.95.0, Postgres 16 reachable, `JWT_SECRET` set), and
becomes one or more QA tests.

- **AC1.** A SQL migration `backend/migrations/00004_nutrition_logs.sql` creates
  one table `nutrition_logs`:
  - `id UUID PRIMARY KEY`,
  - `user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE`,
  - `performed_on DATE NOT NULL`,
  - `protein_g DOUBLE PRECISION NOT NULL`,
  - `carbs_g DOUBLE PRECISION NOT NULL`,
  - `fat_g DOUBLE PRECISION NOT NULL`,
  - `created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`,
  - `updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`,
  - `UNIQUE (user_id, performed_on)`.

  No `calories` column exists — calories are derived. Running migrations from a
  clean Postgres database succeeds. Deleting a `users` row cascades to its
  nutrition logs.

- **AC2.** `POST /nutrition` with a valid `Bearer` token and a JSON body
  (`performed_on`, `protein_g`, `carbs_g`, `fat_g`):
  - returns `201 Created` with the stored log as JSON, including the
    server-generated `id`, the **derived `calories`**, `created_at`, and
    `updated_at`;
  - persists the row owned by the caller's `user_id`;
  - returns `409 Conflict` (writing nothing) when the caller already has a log
    for that `performed_on` (the unique `(user_id, performed_on)` constraint —
    edit the existing log via `PUT` instead);
  - returns `401 Unauthorized` for a missing/malformed/invalid/expired token
    (delegated to the R-0002 extractor).

- **AC3.** `GET /nutrition` with a valid `Bearer` token returns `200 OK` with a
  JSON array of **only the caller's** logs, ordered by `performed_on` descending
  (newest first), each carrying its derived `calories`; an empty array when the
  caller has logged none; `401` for a missing/invalid token.

- **AC4.** `GET /nutrition/{id}` with a valid `Bearer` token returns `200 OK`
  with the log when it exists **and is owned by the caller**; returns `404 Not
  Found` when the id does not exist **or belongs to another user** (ownership is
  never leaked via a distinct status); `401` for a missing/invalid token.

- **AC5.** `PUT /nutrition/{id}` with a valid `Bearer` token and a full body
  performs a **full-replace**: `protein_g`, `carbs_g`, `fat_g`, and
  `performed_on` are overwritten and `updated_at` is bumped; returns `200 OK`
  with the stored log (recomputed `calories`). Returns `404` when the id is
  missing or owned by another user, `409` when the new `performed_on` collides
  with another existing log of the caller, `400` on a validation failure
  (writing nothing), and `401` for a missing/invalid token.

- **AC6.** `DELETE /nutrition/{id}` with a valid `Bearer` token deletes the
  caller's log and returns `204 No Content`; returns `404` when the id is
  missing or owned by another user; `401` for a missing/invalid token. A second
  `DELETE` of the same id returns `404`.

- **AC7.** The log JSON returned by `POST`/`GET`/`PUT` carries: `id`, `user_id`,
  `performed_on` (RFC 3339 date), `protein_g`, `carbs_g`, `fat_g` (grams, as
  numbers), `calories` (derived number), `created_at`, and `updated_at` (RFC
  3339 timestamps).

- **AC8.** Field validation returns `400 Bad Request` (and writes nothing) when:
  - `performed_on` is in the future;
  - any of `protein_g`, `carbs_g`, `fat_g` is missing;
  - any of `protein_g`, `carbs_g`, `fat_g` is negative (`< 0`) or exceeds
    `2000` grams.

- **AC9.** Calorie derivation is exactly `calories = 4·protein_g + 4·carbs_g +
  9·fat_g`, defined once as a method on the `core` nutrition aggregate and used
  as the single source of truth for the `calories` field in every response.
  No calories value is ever read from the request or the database.

- **AC10.** Authorization scoping: a log is only ever readable/mutable by its
  owner. Two distinct authenticated users each operate on their own logs; user A
  can never read, edit, or delete user B's log (each cross-user attempt yields
  `404`), and A's `GET /nutrition` never returns B's logs. No cross-user
  identifier is accepted — the owner is always the token's `sub`.

- **AC11.** The `core` crate gains the nutrition domain (`NutritionLog`
  aggregate, `NewNutritionLog` write model, `Macros` group, `Grams` value type,
  and the `calories()` derivation) with **unit tests** for every validation rule
  in AC8 and for the AC9 calorie formula, and remains free of `sqlx`/`axum`/HTTP
  dependencies (the R-0002/R-0003/R-0004 purity boundary is preserved).

- **AC12.** At least **twelve** integration tests (`#[sqlx::test]`, the
  R-0004 harness pattern) cover the surface above, including: `POST` create
  (201 + derived calories), `POST` duplicate-date (409), `GET` list (newest-first
  ordering; empty array), `GET /{id}` (200 owned; 404 missing; 404 other-user),
  `PUT /{id}` (200 full-replace + `updated_at` bump; 404 other-user; 409
  date-collision), `DELETE /{id}` (204 then 404), each `400` validation branch
  from AC8, unauthorized `POST`/`GET`/`PUT`/`DELETE` (401), and the cross-user
  isolation cases from AC10.

## 4. Constraints & non-goals

**In scope (R-0005):**
- The single `nutrition_logs` table, the full `/nutrition` CRUD surface, the
  `core` nutrition domain types and their validation, and server-side calorie
  derivation, as described in §3.
- Reuse of the R-0002 `AuthenticatedUser` extractor for all routes (no new auth
  machinery).

**Out of scope (deferred):**
- **Barcode scan / food database lookup** — the brief lists barcode scan as a
  nutrition-logger feature, but it requires an external food database and is
  explicitly deferred; R-0005 is **manual entry only**.
- **Per-meal breakdown** (breakfast/lunch/dinner/snack entries, per-meal
  timestamps) — R-0005 is one aggregate row per day; per-meal grain is a later
  refinement only if the model needs it.
- **Micronutrients, fiber, sugar, sodium, water, alcohol** — additional logged
  dimensions deferred until the model demonstrably needs them; fiber/alcohol
  also refine the calorie formula, which stays 4/4/9 for now.
- **Stored calories / label-value calories** — calories are always derived from
  macros; a user-supplied calorie field is intentionally rejected from the data
  model to prevent macro/calorie drift.
- **Macro targets / goals and adherence scoring** — comparing logged macros to a
  prescribed target belongs to the adjustment engine (M5, R-0017+).
- **Partial update (`PATCH`)** — editing is full-replace via `PUT`.
- **Pagination / rich filtering** of the list endpoint (date ranges, cursors) —
  `GET /nutrition` returns the caller's logs newest-first; filtering is a later
  refinement if the dataset warrants it.
- **Imperial units / calorie-unit choices** — grams and kilocalories only,
  consistent with the project's metric convention.

## 5. Open questions

None remaining — the two design forks were settled with the owner 2026-05-31
(below). Implementation-level questions — exact SQL ordering/tie-break columns,
the precise newtype representation of `Grams`, whether the duplicate-date check
is enforced purely by the unique constraint or also pre-checked, and per-test DB
isolation (the R-0004 `#[sqlx::test]` pattern is the presumed default) — are
deferred to SPEC-0005, where they belong.

## 6. Decision log

Decisions made together (owner + Claude). Append-only.

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-05-31 | **Granularity: per-day — one `nutrition_logs` row per `(user_id, performed_on)`**, enforced by a unique constraint. | Matches the brief's flat `NutritionLogs (date, protein, carbs, fat, calories)` and the exact grain the M5 model consumes; per-meal breakdown is a later refinement. Owner-approved (Q1). |
| 2026-05-31 | **Calories are derived, never stored** — `4·protein + 4·carbs + 9·fat`, computed server-side, returned in every response; no `calories` column or request field. | A stored calorie value can drift from the macros that determine it; making macros the single source of truth removes that inconsistency class. 4/4/9 is the standard Atwater set. Owner-approved (Q2). |
| 2026-05-31 | **REST surface: full CRUD** — `POST` + `GET` list + `GET /{id}` + `PUT /{id}` + `DELETE /{id}`, mirroring R-0004. | Users mis-log days and must correct/remove them; append-only would be insufficient. Claude-proposed within the brief's logging mandate; consistent with R-0004. |
| 2026-05-31 | **Duplicate-date `POST` returns `409 Conflict`; editing an existing day is `PUT /{id}`.** | Per-day uniqueness makes create-on-existing-date ambiguous; `409` keeps `POST` create-only and unambiguous, with `PUT` as the explicit edit path. Claude-proposed engineering call. |
| 2026-05-31 | **Cross-user access returns `404`, never `403`.** | Mirrors R-0004 / R-0003 / R-0002 enumeration-safety: ownership existence is never leaked through a distinct status. Claude-proposed. |
| 2026-05-31 | **Macros required and bounded: `protein_g`, `carbs_g`, `fat_g` each ∈ [0, 2000] grams; a `Grams` newtype.** | All three are model inputs, so none may default silently; `0` is valid (e.g. zero-fat day) but negatives and absurd values are garbage. The generous 2000 g cap rejects only nonsense. Claude-proposed. |
| 2026-05-31 | **Nutrition domain types live in `crates/core`** (`NutritionLog`, `NewNutritionLog`, `Macros`, `Grams`, plus the `calories()` derivation), preserving the purity boundary. | Continues the R-0002/R-0003/R-0004 layering; keeps validation and the calorie formula pure and HTTP/DB-free. Claude-proposed. |

## Changelog

- _2026-05-31 — created (Draft); two design forks settled with owner the same day (per-day single-row grain with a unique `(user_id, performed_on)` constraint; calories derived 4/4/9 and never stored); five derived engineering decisions recorded._
- _2026-05-31 — owner acked the twelve acceptance criteria (AC1–AC12); status → Accepted. SPEC-0005 may begin._
