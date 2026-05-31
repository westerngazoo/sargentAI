# R-0004 — Workout log

- **Status:** Accepted
- **Milestone:** M2
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-05-31
- **Depends on:** R-0002 (Met), R-0003 (Met)
- **Realized by:** SPEC-0004 (to be written once this R is `Accepted`)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

An authenticated user can **create, read, edit, and delete** their own
**workout sessions**. A session is dated (`performed_on`) and contains one or
more **exercises**, each carrying a free-text name and an optional controlled
**muscle group**; each exercise contains one or more **sets**, and each set
records **reps** (required), an optional **weight** (kg), and an optional
**RPE** (rate of perceived exertion). The structure is a 1:N:N hierarchy —
one user has many sessions, a session has many exercises, an exercise has many
sets.

Sessions are owned by the authenticated `user_id` from R-0002 and scoped to the
caller: a user can only ever read or mutate their own sessions. The collection
is exposed under `/workouts` with the full lifecycle — `POST` (create),
`GET` (list own, newest first), `GET /{id}` (read one), `PUT /{id}`
(full-replace edit), and `DELETE /{id}` (remove). Every route is protected by
the R-0002 `Bearer` JWT middleware.

This requirement **introduces the `crates/core` workout domain types**: the
`WorkoutSession` aggregate and its validated value types (`MuscleGroup`, `Rpe`,
`Reps`, `LoadKg`, `ExerciseName`) and the `NewWorkoutSession` write model live
in the pure `core` crate with no `sqlx`/`axum`/HTTP dependencies, mirroring the
profile domain established in R-0003. Logged values are stored **verbatim** in
canonical metric units; no value is derived at read time (volume, `%1RM`, and
per-muscle-group aggregation are ML-layer concerns deferred to M5).

## 2. Rationale

The workout log is the **primary training signal** the ML response-inference
engine (M5) consumes: training volume (sets × reps × weight per muscle group),
frequency (sessions/week per muscle group), and intensity (average RPE) are all
computed from these rows (source brief, *ML Model Design → Input Features*).
Without a faithful, queryable record of what the user actually did in the gym,
there is nothing to correlate against body-composition change, and the
adjustment engine (R-0017) has no input.

A **normalized three-table** shape (`workout_sessions` → `workout_exercises` →
`workout_sets`) is chosen over a JSONB document because the ML layer aggregates
*across* the hierarchy — per muscle group, per time window — and that is SQL's
job, not the application's. Storing an **optional muscle group** on each
exercise captures the grouping signal the model needs now, while leaving a full
curated exercise catalog (with canonical names and muscle mappings) to its own
later requirement. **Full CRUD** (not just append) is in scope because users
mis-log sessions and must be able to correct or remove them; editing is a
**transactional full-replace** of the session's nested rows rather than a
partial patch, keeping the mutation semantics unambiguous.

Storing logged values **verbatim** (no read-time derivation) keeps R-0004 a
pure capture layer; `%1RM` requires a formula choice and volume requires a
muscle-group rollup, both of which belong with the model that consumes them.

## 3. Acceptance criteria

Each criterion is observable from a checkout of the R-0004 branch with the
toolchain installed (Rust 1.95.0, Postgres 16 reachable, `JWT_SECRET` set), and
becomes one or more QA tests.

- **AC1.** A SQL migration in `backend/migrations/` creates three tables:
  - `workout_sessions`: `id UUID PRIMARY KEY`,
    `user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE`,
    `performed_on DATE NOT NULL`, `created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`,
    `updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`;
  - `workout_exercises`: `id UUID PRIMARY KEY`,
    `session_id UUID NOT NULL REFERENCES workout_sessions(id) ON DELETE CASCADE`,
    `position INT NOT NULL`, `name TEXT NOT NULL`, `muscle_group TEXT NULL`;
  - `workout_sets`: `id UUID PRIMARY KEY`,
    `exercise_id UUID NOT NULL REFERENCES workout_exercises(id) ON DELETE CASCADE`,
    `position INT NOT NULL`, `reps INT NOT NULL`,
    `weight_kg DOUBLE PRECISION NULL`, `rpe DOUBLE PRECISION NULL`.

  Running migrations from a clean Postgres database succeeds. Deleting a `users`
  row cascades to its sessions, their exercises, and their sets; deleting a
  session cascades to its exercises and sets.

- **AC2.** `POST /workouts` with a valid `Bearer` token and a JSON body
  describing a session (`performed_on`, `exercises[]` each with `name`,
  optional `muscle_group`, and `sets[]` each with `reps`, optional `weight_kg`,
  optional `rpe`):
  - returns `201 Created` with the stored session as JSON, including the
    server-generated `id`s (session, each exercise, each set), `created_at`,
    and `updated_at`;
  - persists the full hierarchy in the three tables, owned by the caller's
    `user_id`;
  - returns `401 Unauthorized` for a missing/malformed/invalid/expired token
    (delegated to the R-0002 extractor).

- **AC3.** `GET /workouts` with a valid `Bearer` token returns `200 OK` with a
  JSON array of **only the caller's** sessions, ordered by `performed_on`
  descending (newest first), each session carrying its full nested
  exercises/sets; an empty array when the caller has logged none; `401` for a
  missing/invalid token.

- **AC4.** `GET /workouts/{id}` with a valid `Bearer` token returns `200 OK`
  with the full nested session when it exists **and is owned by the caller**;
  returns `404 Not Found` when the id does not exist **or belongs to another
  user** (ownership is never leaked via a distinct status); `401` for a
  missing/invalid token.

- **AC5.** `PUT /workouts/{id}` with a valid `Bearer` token and a full session
  body performs a **transactional full-replace**: the session's exercises and
  sets are atomically replaced by those in the body, `updated_at` is bumped, and
  `performed_on` is updated; returns `200 OK` with the stored session (new
  exercise/set `id`s reflecting the replacement). Returns `404` when the id is
  missing or owned by another user, `400` on a validation failure (writing
  nothing), and `401` for a missing/invalid token.

- **AC6.** `DELETE /workouts/{id}` with a valid `Bearer` token deletes the
  caller's session (cascading to its exercises and sets) and returns
  `204 No Content`; returns `404` when the id is missing or owned by another
  user; `401` for a missing/invalid token. A second `DELETE` of the same id
  returns `404`.

- **AC7.** The session JSON returned by `POST`/`GET`/`PUT` carries: `id`,
  `user_id`, `performed_on` (RFC 3339 date), `created_at`, `updated_at`
  (RFC 3339 timestamps), and `exercises` — an ordered array where each element
  has `id`, `position`, `name`, `muscle_group` (one of the controlled set or
  `null`), and `sets` — an ordered array where each element has `id`,
  `position`, `reps`, `weight_kg` (or `null`), and `rpe` (or `null`).

- **AC8.** Field validation returns `400 Bad Request` (and writes nothing) when:
  - `performed_on` is in the future;
  - `exercises` is empty, or any exercise has an empty `sets` array;
  - an exercise `name` is blank/whitespace-only or longer than 100 characters;
  - a `muscle_group` is present and outside the controlled set;
  - a set `reps` is `< 1` or `> 10000`;
  - a set `weight_kg` is present and `<= 0` or `> 1000`;
  - a set `rpe` is present and outside `[6.0, 10.0]` or not a multiple of `0.5`;
  - any required field (`performed_on`, `exercises`, per-exercise `name` and
    `sets`, per-set `reps`) is missing.

- **AC9.** The `muscle_group` controlled set is exactly:
  `chest`, `back`, `shoulders`, `arms`, `legs`, `core`. It is defined once as a
  `core::MuscleGroup` enum and is the single source of truth for both request
  parsing and the JSON shape.

- **AC10.** Authorization scoping: a session is only ever readable/mutable by
  its owner. Two distinct authenticated users each operate on their own
  sessions; user A can never read, edit, or delete user B's session (each
  cross-user attempt yields `404`), and A's `GET /workouts` never returns B's
  sessions. No cross-user identifier is accepted — the owner is always the
  token's `sub`.

- **AC11.** The `core` crate gains the workout domain (`WorkoutSession`
  aggregate, `NewWorkoutSession` write model, and the `MuscleGroup`, `Rpe`,
  `Reps`, `LoadKg`, `ExerciseName` value types) with **unit tests** for every
  validation rule in AC8/AC9, and remains free of `sqlx`/`axum`/HTTP
  dependencies (the R-0002/R-0003 purity boundary is preserved).

- **AC12.** At least **fourteen** integration tests (`#[sqlx::test]`, the
  R-0002/R-0003 harness pattern) cover the surface above, including: `POST`
  create (201 + nested ids), `GET` list (newest-first ordering; empty array),
  `GET /{id}` (200 owned; 404 missing; 404 other-user), `PUT /{id}`
  (200 full-replace + `updated_at` bump; 404 other-user), `DELETE /{id}`
  (204 then 404), each `400` validation branch from AC8, unauthorized
  `POST`/`GET`/`PUT`/`DELETE` (401), and the cross-user isolation cases from
  AC10.

## 4. Constraints & non-goals

**In scope (R-0004):**
- The `workout_sessions` / `workout_exercises` / `workout_sets` tables, the
  full `/workouts` CRUD surface, the `core` workout domain types, and their
  validation, as described in §3.
- Reuse of the R-0002 `AuthenticatedUser` extractor for all routes (no new auth
  machinery).

**Out of scope (deferred):**
- **Curated exercise catalog** (canonical exercise names, per-exercise muscle
  mappings, equipment, aliases) — its own later requirement; R-0004 uses
  free-text names plus an optional `muscle_group` enum.
- **Derived training metrics** — training volume (sets × reps × weight per
  muscle group), frequency, `%1RM` estimates, and any per-time-window rollup —
  belong to the ML aggregation layer (M5, R-0015+). R-0004 stores raw values
  only.
- **Partial update (`PATCH`)** — editing is full-replace via `PUT`; a partial
  patch earns its own treatment only if a real need appears.
- **Pagination / rich filtering** of the list endpoint (date ranges, muscle
  group, cursors) — `GET /workouts` returns the caller's sessions newest-first;
  filtering is a later refinement if the dataset warrants it.
- **Tempo, rest-between-sets, set type (warmup/working/drop), supersets, and
  per-session notes** — additional logged dimensions deferred until the model
  demonstrably needs them.
- **Nutrition and photo logs** — sibling M2 requirements (R-0005, R-0006).
- **Imperial units** — canonical metric (kg) only, consistent with R-0003.

## 5. Open questions

None remaining — the four design decisions were settled with the owner
2026-05-31 (below). Implementation-level questions — exact SQL ordering columns,
whether `position` is client-supplied or server-assigned, the precise newtype
representation of `Rpe`'s half-point constraint, and per-test DB isolation (the
R-0002/R-0003 `#[sqlx::test]` pattern is the presumed default) — are deferred to
SPEC-0004, where they belong.

## 6. Decision log

Decisions made together (owner + Claude). Append-only.

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-05-31 | **Schema: normalized three tables** (`workout_sessions` → `workout_exercises` → `workout_sets`, FK `ON DELETE CASCADE`). | The ML layer aggregates across the hierarchy (volume/RPE per muscle group, per window); relational shape makes that SQL's job. Owner-approved (OQ1). |
| 2026-05-31 | **Exercise identity: free-text `name` + optional `muscle_group` enum.** | Captures the per-muscle-group grouping signal the model needs now; a full curated exercise catalog is its own later R. Owner-approved (OQ2). |
| 2026-05-31 | **REST surface: full CRUD** — `POST` + `GET` list + `GET /{id}` + `PUT /{id}` + `DELETE /{id}`. | Users mis-log and must correct/remove sessions; append-only would be insufficient. Owner-approved (OQ3). |
| 2026-05-31 | **Per-set fields: `reps` required; `weight_kg` & `rpe` optional; RPE on the 6.0–10.0 half-point scale.** | Matches how lifters actually log (bodyweight = no weight; RPE is subjective/optional); 6–10 step-0.5 is the standard RPE scale and the brief's "avg RPE" input. Owner-approved (OQ4). |
| 2026-05-31 | **Editing is a transactional full-replace via `PUT /{id}`; no `PATCH`.** | A nested aggregate has ambiguous partial-update semantics; atomically replacing the session's exercises/sets keeps mutation unambiguous and the diff reviewable. Claude-proposed engineering call within the owner's "full CRUD incl. edit" choice. |
| 2026-05-31 | **Cross-user access returns `404`, never `403`.** | Mirrors R-0003 AC7 / R-0002 enumeration-safety: ownership existence is never leaked through a distinct status. Claude-proposed. |
| 2026-05-31 | **No read-time derivation; logged values stored verbatim in metric (kg).** | `%1RM` needs a formula choice and volume needs a muscle-group rollup — both belong with the M5 model that consumes them, not the capture layer. Claude-proposed (§4). |
| 2026-05-31 | **`muscle_group` controlled set: `chest`, `back`, `shoulders`, `arms`, `legs`, `core`.** Coarse, single `core::MuscleGroup` authority. | A coarse split is enough to group volume for the prior; finer granularity (e.g. quads vs hamstrings) can extend the enum later without schema change. Claude-proposed; revisit if the model needs finer groups. |
| 2026-05-31 | **Set bounds: `reps` ∈ [1, 10000]; `weight_kg` ∈ (0, 1000]; a workout-specific `LoadKg` newtype distinct from the profile `WeightKg`.** | Lifted load has different semantics/range from body weight (a 2.5 kg dumbbell is valid; profile weight starts at 20 kg), so the profile newtype is not reused. Generous caps reject only garbage. Claude-proposed. |
| 2026-05-31 | **Workout domain types live in `crates/core`** (`WorkoutSession`, `NewWorkoutSession`, `MuscleGroup`, `Rpe`, `Reps`, `LoadKg`, `ExerciseName`), preserving the purity boundary. | Continues the R-0002/R-0003 layering; keeps validation pure and HTTP/DB-free. Claude-proposed. |

## Changelog

- _2026-05-31 — created (Draft); OQ1–OQ4 settled with owner the same day (normalized 3-table schema; free-text name + optional muscle_group enum; full CRUD with full-replace PUT edit; reps required / weight & RPE optional, RPE 6–10 step 0.5); five derived engineering decisions recorded._
- _2026-05-31 — owner acked the twelve acceptance criteria (AC1–AC12); status → Accepted. SPEC-0004 may begin._
