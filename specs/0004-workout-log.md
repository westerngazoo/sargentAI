# SPEC-0004 — Workout log

- **Status:** Accepted
- **Realizes:** R-0004
- **Author:** Claude (main session), with owner
- **Created:** 2026-05-31
- **Depends on:** SPEC-0002 (Implemented), SPEC-0003 (Implemented) — reuses `AppState`, the `AuthenticatedUser` extractor, `ApiError` (incl. `NotFound`), the `db` seam, the parse-don't-validate core layering, and the migration/CI/dev-DB machinery
- **Module(s):** `backend/crates/core/workout` (new), `backend/crates/api/workout` (new), `backend/crates/api/http` (new — the promoted `parse_body` helper, OQ-C5), `backend/crates/api/{db,lib}` (extended), `backend/crates/api/profile/handlers.rs` (edited — imports the promoted `parse_body`), `backend/migrations/` (new file)

## 1. Motivation

Realizes [R-0004](../requirements/0004-workout-log.md): an authenticated user
can create, read, edit, and delete their own **workout sessions** — the primary
training signal the ML response-inference engine (M5) consumes. A session is a
1:N:N hierarchy (session → exercises → sets) exposed as full CRUD under
`/workouts`, owned by the token's `sub`.

R-0004 is the spec where `crates/core` gains its **third domain** (the workout
aggregate and its value types), the api gains its **first multi-table,
transactional persistence** (three tables written atomically), and its **first
collection resource** with id-addressed sub-routes — all built on the
R-0002/R-0003 primitives, not new ones.

## 2. Design

### 2.1 Shape

Three normalised tables (owner OQ1), one row per logical entity, FK
`ON DELETE CASCADE` down the chain so deleting a user removes sessions →
exercises → sets, and deleting a session removes its exercises → sets:

```
users (R-0002)
  └── workout_sessions   (id, user_id→users, performed_on, created_at, updated_at)
        └── workout_exercises (id, session_id→sessions, position, name, muscle_group?)
              └── workout_sets    (id, exercise_id→exercises, position, reps, weight_kg?, rpe?)
```

The ML layer (M5) aggregates *across* this hierarchy (volume = Σ sets×reps×weight
per muscle group; avg RPE per window); a relational shape keeps that in SQL.
R-0004 itself derives nothing — values are stored and returned verbatim.

### 2.2 Layering (preserves the R-0002/R-0003 purity boundary)

- **`core::workout`** (pure — no `sqlx`/`axum`/HTTP):
  - value types: `MuscleGroup` (closed enum), `Reps`, `LoadKg`, `Rpe`
    (validated numeric newtypes), `ExerciseName` (validated string newtype);
  - **write models** (validated, no identity/timestamps): `NewSet`,
    `NewExercise`, `NewWorkoutSession` — built through `::new(..)` constructors
    that return `Result<_, WorkoutError>`; `NewWorkoutSession::new` takes
    `today` (injected) for the `performed_on`-not-future check;
  - **read aggregates** (reconstructed from rows, `Serialize`): `WorkoutSet`,
    `WorkoutExercise`, `WorkoutSession` — carry server-assigned `Uuid` ids,
    `position`, and (on the session) `user_id`/timestamps;
  - typed `WorkoutError` with a `.field()` method naming the offending request
    field (drives `ApiError::Validation { field }`).
- **`api::db`** (persistence seam): `SessionRow`/`ExerciseRow`/`SetRow`
  (`FromRow`) and the five queries (`insert_session`, `find_sessions_by_user`,
  `find_session_by_id`, `replace_session`, `delete_session`). Rows map back to
  core aggregates; a stored value that fails domain validation is data
  corruption → logged 500 (the `into_*` discipline from R-0003).
- **`api::workout`** (HTTP): request DTOs (`SessionRequest`/`ExerciseRequest`/
  `SetRequest`), the five handlers, and `routes()`. Validation is `core`'s job;
  handlers are thin.

### 2.3 Request parsing & validation

Request DTOs deserialize **raw** scalars plus the typed `Option<MuscleGroup>`
(so an unknown muscle group is serde-rejected to `400` with field `"body"`,
exactly as R-0003 handles `Goal`/`Sex`). The handler then calls
`NewWorkoutSession::new(req, today)`, which validates every rule in AC8 and
returns the first `WorkoutError`; the handler maps it to
`ApiError::Validation { field: e.field() }` (`400`, nothing written). A
malformed/again-missing body (missing `performed_on`, non-array `exercises`,
missing `reps`) is a `JsonRejection` → `400` field `"body"` (the field-label
asymmetry documented in SPEC-0003 §2.3: structural failures report `"body"`,
semantic failures report the leaf field).

`MuscleGroup` is the **single authority** for the controlled set (AC9): it is
the only place the six strings live, shared by serde (JSON) and `as_str`/`parse`
(SQL), pinned equal by an exhaustive unit test (the dual-encoding discipline
from SPEC-0003 §2.4).

### 2.4 Response shape

The `core` read aggregates derive `Serialize` with field names and transparent
newtypes that **already match AC7 exactly** (`weight_kg`, `rpe`, `muscle_group`
nullable; `id`/`position`/`user_id`/`performed_on`/`created_at`/`updated_at`),
and R-0004 has **no derived response field** (unlike R-0003's `age`). The
handlers therefore serialize the aggregate directly rather than maintaining a
parallel tree of response DTOs. **Resolved (architect, OQ-C1):** serialize the
aggregate; a parallel DTO tree would be pure restatement (CLAUDE.md §2 "no
premature abstraction"). Because this couples the domain type to the wire
contract, SAC7's test asserts the **literal JSON keys** (not a Rust-struct
round-trip) so the contract is pinned.

### 2.5 Persistence & transactions

`insert_session` and `replace_session` write three tables and **must be
atomic** — this is the codebase's first multi-statement transaction. Both take
`pool.begin().await?`, perform their writes against the `&mut *tx`, and
`tx.commit().await?` only on full success; any `?` bubbles an error and the
transaction drops → rollback.

- **Ownership scoping (AC4/AC5/AC6/AC10):** every id-addressed query carries
  `AND user_id = $caller`. `find_session_by_id` returns `None` (→ `404`) when
  the id is missing *or* owned by another user; `replace_session` and
  `delete_session` likewise no-op → `None`/`false` (→ `404`). Ownership is never
  leaked via a distinct status. No id from the path is ever trusted as an owner.
- **`replace_session` (AC5):** within one tx — `UPDATE workout_sessions SET
  performed_on = $1, updated_at = NOW() WHERE id = $2 AND user_id = $3 RETURNING
  created_at, updated_at`; if no row, the session is missing/foreign → return
  `None`. Otherwise `DELETE FROM workout_exercises WHERE session_id = $2` (sets
  cascade), then re-insert the new exercises/sets. Child ids are **not stable**
  across an edit (full-replace, not diff) — documented, acceptable. *OQ-C4.*
- **`delete_session` (AC6):** `DELETE FROM workout_sessions WHERE id = $1 AND
  user_id = $2`; `rows_affected() > 0` → `204`, else `404`.
- **List assembly (AC3):** `find_sessions_by_user` issues **three** queries —
  sessions for the user (`ORDER BY performed_on DESC, created_at DESC`),
  exercises `WHERE session_id = ANY($ids) ORDER BY session_id, position`, sets
  `WHERE exercise_id = ANY($ids) ORDER BY exercise_id, position` — and assembles
  in memory by parent id. This avoids both per-session N+1 and the row-explosion
  of a 3-way join. **Resolved (architect, OQ-C2):** two contract points — (a)
  children are grouped under their parent **in `position` order** (the
  `ORDER BY parent_id, position` clause makes the grouped push order correct, or
  each child vec is sorted after grouping); (b) when the user has zero sessions,
  the two child queries are **short-circuited** (return `[]` after query 1) so no
  degenerate `ANY('{}')` bind is issued. The same nested ordering guarantee
  applies to `find_session_by_id`'s single-session read.

### 2.6 `position`

`position` is **server-assigned** from the request array index (0-based `i32`),
so client array order is the source of truth and the stored/returned order is
deterministic. By construction it is **contiguous and 0-based per parent**
(exercises within a session, sets within an exercise), which SAC7 asserts. Reads
`ORDER BY position`. **Resolved (architect, OQ-C3):** server-assigned; keep the
checked `i32::try_from(index)` cast (§3.6), never `as i32`.

### 2.7 SQL types

`reps INTEGER`/`i32`; `weight_kg`/`rpe` `DOUBLE PRECISION`/`f64` (0.1 / 0.5
resolution; no `rust_decimal`, consistent with SPEC-0003 §2.7). `performed_on
DATE`/`NaiveDate`. Ids `UUID`/`Uuid` generated app-side (`Uuid::new_v4()`, as
`insert_user` does). Validation lives in `core`, never DB `CHECK`s (SPEC-0002
OQ-A1). FK-column indexes (`user_id`, `session_id`, `exercise_id`) support the
lookups and cascades.

## 3. Code outline

Snippets are representative (final form reconciled in step-5 lockstep with the
pinned 1.95.0 toolchain, SPEC-0001 §7 policy). Tests are authored by `qa` in
step 3 against §6.

### 3.1 `backend/migrations/00003_workout_logs.sql`

```sql
-- R-0004 / SPEC-0004 — workout log (sessions → exercises → sets).
-- Validation lives in crates/core (SPEC-0002 OQ-A1); the DB enforces
-- referential integrity and ordering support only.

CREATE TABLE workout_sessions (
    id           UUID PRIMARY KEY,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    performed_on DATE NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_workout_sessions_user_id ON workout_sessions (user_id);

CREATE TABLE workout_exercises (
    id           UUID PRIMARY KEY,
    session_id   UUID NOT NULL REFERENCES workout_sessions(id) ON DELETE CASCADE,
    position     INTEGER NOT NULL,
    name         TEXT NOT NULL,
    muscle_group TEXT
);
CREATE INDEX idx_workout_exercises_session_id ON workout_exercises (session_id);

CREATE TABLE workout_sets (
    id          UUID PRIMARY KEY,
    exercise_id UUID NOT NULL REFERENCES workout_exercises(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    reps        INTEGER NOT NULL,
    weight_kg   DOUBLE PRECISION,
    rpe         DOUBLE PRECISION
);
CREATE INDEX idx_workout_sets_exercise_id ON workout_sets (exercise_id);
```

### 3.2 `backend/crates/core/src/lib.rs` (extended)

```rust
pub mod profile;
pub mod user;
pub mod workout;

pub use workout::{
    ExerciseName, LoadKg, MuscleGroup, NewExercise, NewSet, NewWorkoutSession, Reps, Rpe,
    WorkoutError, WorkoutExercise, WorkoutSession, WorkoutSet,
};
// … existing profile / user re-exports unchanged …
```

### 3.3 `core/src/workout.rs` — `MuscleGroup`, newtypes, `WorkoutError`

```rust
//! Workout-log domain: the `WorkoutSession` aggregate and its value types.
//! Pure — no DB, no HTTP. Parse-don't-validate, as `profile`/`user`.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::UserId;

/// Coarse muscle grouping. Closed set; the single authority (AC9).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MuscleGroup {
    Chest,
    Back,
    Shoulders,
    Arms,
    Legs,
    Core,
}

impl MuscleGroup {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            MuscleGroup::Chest => "chest",
            MuscleGroup::Back => "back",
            MuscleGroup::Shoulders => "shoulders",
            MuscleGroup::Arms => "arms",
            MuscleGroup::Legs => "legs",
            MuscleGroup::Core => "core",
        }
    }

    /// Parse the canonical SQL string (inverse of [`MuscleGroup::as_str`]).
    ///
    /// # Errors
    /// [`WorkoutError::MuscleGroupUnknown`] for anything outside the set.
    pub fn parse(raw: &str) -> Result<Self, WorkoutError> {
        match raw {
            "chest" => Ok(Self::Chest),
            "back" => Ok(Self::Back),
            "shoulders" => Ok(Self::Shoulders),
            "arms" => Ok(Self::Arms),
            "legs" => Ok(Self::Legs),
            "core" => Ok(Self::Core),
            _ => Err(WorkoutError::MuscleGroupUnknown),
        }
    }
}

/// Repetitions in a set, range [1, 10000].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Reps(i32);

impl Reps {
    pub const MIN: i32 = 1;
    pub const MAX: i32 = 10_000;

    /// # Errors
    /// [`WorkoutError::RepsOutOfRange`] when outside `[1, 10000]`.
    pub fn try_new(reps: i32) -> Result<Self, WorkoutError> {
        if (Self::MIN..=Self::MAX).contains(&reps) {
            Ok(Self(reps))
        } else {
            Err(WorkoutError::RepsOutOfRange)
        }
    }

    #[must_use]
    pub fn get(self) -> i32 {
        self.0
    }
}

/// Lifted load in kilograms, range (0, 1000]. Distinct from the profile
/// `WeightKg` (different semantics/range — a 2.5 kg dumbbell is valid).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct LoadKg(f64);

impl LoadKg {
    pub const MAX: f64 = 1000.0;

    /// # Errors
    /// [`WorkoutError::WeightOutOfRange`] when not finite, `<= 0`, or `> 1000`.
    pub fn try_new(kg: f64) -> Result<Self, WorkoutError> {
        if kg.is_finite() && kg > 0.0 && kg <= Self::MAX {
            Ok(Self(kg))
        } else {
            Err(WorkoutError::WeightOutOfRange)
        }
    }

    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// Rate of perceived exertion: [6.0, 10.0] in 0.5 steps.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Rpe(f64);

impl Rpe {
    pub const MIN: f64 = 6.0;
    pub const MAX: f64 = 10.0;

    /// # Errors
    /// [`WorkoutError::RpeInvalid`] when not finite, outside `[6, 10]`, or not
    /// a multiple of `0.5`.
    pub fn try_new(rpe: f64) -> Result<Self, WorkoutError> {
        let half_step = (rpe * 2.0).fract() == 0.0;
        if rpe.is_finite() && (Self::MIN..=Self::MAX).contains(&rpe) && half_step {
            Ok(Self(rpe))
        } else {
            Err(WorkoutError::RpeInvalid)
        }
    }

    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// A non-blank exercise name, trimmed, ≤ 100 characters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ExerciseName(String);

impl ExerciseName {
    pub const MAX_CHARS: usize = 100;

    /// # Errors
    /// [`WorkoutError::NameBlank`] if empty after trimming;
    /// [`WorkoutError::NameTooLong`] if over 100 characters.
    pub fn try_new(raw: &str) -> Result<Self, WorkoutError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(WorkoutError::NameBlank);
        }
        if trimmed.chars().count() > Self::MAX_CHARS {
            return Err(WorkoutError::NameTooLong);
        }
        Ok(Self(trimmed.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkoutError {
    #[error("performed_on is in the future")]
    PerformedOnInFuture,
    #[error("a session must have at least one exercise")]
    ExercisesEmpty,
    #[error("an exercise must have at least one set")]
    SetsEmpty,
    #[error("exercise name is blank")]
    NameBlank,
    #[error("exercise name is too long")]
    NameTooLong,
    #[error("reps is outside the allowed range")]
    RepsOutOfRange,
    #[error("weight_kg is outside the allowed range")]
    WeightOutOfRange,
    #[error("rpe is invalid")]
    RpeInvalid,
    #[error("unknown muscle group")]
    MuscleGroupUnknown,
}

impl WorkoutError {
    /// The request field this error concerns — drives `ApiError::Validation`.
    #[must_use]
    pub fn field(&self) -> &'static str {
        match self {
            WorkoutError::PerformedOnInFuture => "performed_on",
            WorkoutError::ExercisesEmpty => "exercises",
            WorkoutError::SetsEmpty => "sets",
            WorkoutError::NameBlank | WorkoutError::NameTooLong => "name",
            WorkoutError::RepsOutOfRange => "reps",
            WorkoutError::WeightOutOfRange => "weight_kg",
            WorkoutError::RpeInvalid => "rpe",
            WorkoutError::MuscleGroupUnknown => "muscle_group",
        }
    }
}
```

### 3.4 `core/src/workout.rs` — write models

```rust
/// A validated set (no identity).
#[derive(Clone, Debug, PartialEq)]
pub struct NewSet {
    pub reps: Reps,
    pub weight_kg: Option<LoadKg>,
    pub rpe: Option<Rpe>,
}

impl NewSet {
    /// # Errors
    /// First [`WorkoutError`] among reps / weight / rpe validation.
    pub fn new(reps: i32, weight_kg: Option<f64>, rpe: Option<f64>) -> Result<Self, WorkoutError> {
        Ok(Self {
            reps: Reps::try_new(reps)?,
            weight_kg: weight_kg.map(LoadKg::try_new).transpose()?,
            rpe: rpe.map(Rpe::try_new).transpose()?,
        })
    }
}

/// A validated exercise with at least one set (no identity).
#[derive(Clone, Debug, PartialEq)]
pub struct NewExercise {
    pub name: ExerciseName,
    pub muscle_group: Option<MuscleGroup>,
    pub sets: Vec<NewSet>,
}

impl NewExercise {
    /// # Errors
    /// [`WorkoutError::SetsEmpty`] if no sets, or the first set/name error.
    pub fn new(
        name: &str,
        muscle_group: Option<MuscleGroup>,
        sets: Vec<NewSet>,
    ) -> Result<Self, WorkoutError> {
        if sets.is_empty() {
            return Err(WorkoutError::SetsEmpty);
        }
        Ok(Self {
            name: ExerciseName::try_new(name)?,
            muscle_group,
            sets,
        })
    }
}

/// A validated session with at least one exercise (no identity/timestamps).
#[derive(Clone, Debug, PartialEq)]
pub struct NewWorkoutSession {
    pub performed_on: NaiveDate,
    pub exercises: Vec<NewExercise>,
}

impl NewWorkoutSession {
    /// `today` is injected for a deterministic future-date check.
    ///
    /// # Errors
    /// [`WorkoutError::PerformedOnInFuture`], [`WorkoutError::ExercisesEmpty`],
    /// or the first nested exercise/set error.
    pub fn new(
        performed_on: NaiveDate,
        exercises: Vec<NewExercise>,
        today: NaiveDate,
    ) -> Result<Self, WorkoutError> {
        if performed_on > today {
            return Err(WorkoutError::PerformedOnInFuture);
        }
        if exercises.is_empty() {
            return Err(WorkoutError::ExercisesEmpty);
        }
        Ok(Self {
            performed_on,
            exercises,
        })
    }
}
```

> The handler builds the `Vec<NewExercise>`/`Vec<NewSet>` from the request DTOs
> via the `::new` constructors (innermost first), so the first validation error
> surfaces with its `field()`. Building bottom-up keeps each constructor's
> responsibility single.

### 3.5 `core/src/workout.rs` — read aggregates

```rust
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WorkoutSet {
    pub id: Uuid,
    pub position: i32,
    pub reps: Reps,
    pub weight_kg: Option<LoadKg>,
    pub rpe: Option<Rpe>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WorkoutExercise {
    pub id: Uuid,
    pub position: i32,
    pub name: ExerciseName,
    pub muscle_group: Option<MuscleGroup>,
    pub sets: Vec<WorkoutSet>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WorkoutSession {
    pub id: Uuid,
    pub user_id: UserId,
    pub performed_on: NaiveDate,
    pub exercises: Vec<WorkoutExercise>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### 3.6 `backend/crates/api/src/db.rs` — rows + transactional queries (extended)

```rust
use fitai_core::{
    ExerciseName, LoadKg, MuscleGroup, NewWorkoutSession, Reps, Rpe, WorkoutExercise,
    WorkoutSession, WorkoutSet,
};

#[derive(Debug, FromRow)]
pub struct SessionRow { /* id, user_id, performed_on, created_at, updated_at */ }
#[derive(Debug, FromRow)]
pub struct ExerciseRow { /* id, session_id, position, name, muscle_group: Option<String> */ }
#[derive(Debug, FromRow)]
pub struct SetRow { /* id, exercise_id, position, reps: i32, weight_kg/rpe: Option<f64> */ }

// Row → aggregate. A stored value that fails domain validation is corruption
// → logged 500 (the into_profile / into_user discipline). E.g. for a set:
fn set_from_row(r: SetRow) -> ApiResult<WorkoutSet> {
    let corrupt = |what: &'static str| { /* tracing::error! + ApiError::Internal */ };
    Ok(WorkoutSet {
        id: r.id,
        position: r.position,
        reps: Reps::try_new(r.reps).map_err(|_| corrupt("reps"))?,
        weight_kg: r.weight_kg.map(LoadKg::try_new).transpose().map_err(|_| corrupt("weight_kg"))?,
        rpe: r.rpe.map(Rpe::try_new).transpose().map_err(|_| corrupt("rpe"))?,
    })
}

// Likewise the exercise mapper runs the stored muscle_group back through
// MuscleGroup::parse with the same corruption discipline (mirrors into_profile's
// Sex/Goal handling): a stored string outside the AC9 set is data corruption.
fn exercise_from_row(r: ExerciseRow, sets: Vec<WorkoutSet>) -> ApiResult<WorkoutExercise> {
    Ok(WorkoutExercise {
        id: r.id,
        position: r.position,
        name: ExerciseName::try_new(&r.name).map_err(|_| corrupt("name"))?,
        muscle_group: r.muscle_group.map(|g| MuscleGroup::parse(&g))
            .transpose().map_err(|_| corrupt("muscle_group"))?,
        sets,
    })
}

/// Insert a full session atomically; returns the stored aggregate.
///
/// # Errors
/// [`ApiError::Database`] on any query failure (transaction rolls back).
pub async fn insert_session(
    pool: &PgPool,
    user_id: UserId,
    new: &NewWorkoutSession,
) -> ApiResult<WorkoutSession> {
    let mut tx = pool.begin().await?;
    let session_id = Uuid::new_v4();
    let row: SessionRow = sqlx::query_as(
        "INSERT INTO workout_sessions (id, user_id, performed_on) VALUES ($1, $2, $3) \
         RETURNING id, user_id, performed_on, created_at, updated_at",
    )
    .bind(session_id).bind(user_id.0).bind(new.performed_on)
    .fetch_one(&mut *tx).await?;

    let mut exercises = Vec::with_capacity(new.exercises.len());
    for (ei, ex) in new.exercises.iter().enumerate() {
        let exercise_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workout_exercises (id, session_id, position, name, muscle_group) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(exercise_id).bind(session_id).bind(i32::try_from(ei)?)
        .bind(ex.name.as_str()).bind(ex.muscle_group.map(MuscleGroup::as_str))
        .execute(&mut *tx).await?;

        let mut sets = Vec::with_capacity(ex.sets.len());
        for (si, st) in ex.sets.iter().enumerate() {
            let set_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO workout_sets (id, exercise_id, position, reps, weight_kg, rpe) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(set_id).bind(exercise_id).bind(i32::try_from(si)?)
            .bind(st.reps.get()).bind(st.weight_kg.map(LoadKg::get)).bind(st.rpe.map(Rpe::get))
            .execute(&mut *tx).await?;
            sets.push(WorkoutSet { id: set_id, position: i32::try_from(si)?, reps: st.reps,
                weight_kg: st.weight_kg, rpe: st.rpe });
        }
        exercises.push(WorkoutExercise { id: exercise_id, position: i32::try_from(ei)?,
            name: ex.name.clone(), muscle_group: ex.muscle_group, sets });
    }
    tx.commit().await?;
    Ok(WorkoutSession { id: row.id, user_id, performed_on: row.performed_on,
        exercises, created_at: row.created_at, updated_at: row.updated_at })
}

/// All of the caller's sessions, newest `performed_on` first, fully nested.
/// Three queries (sessions; exercises by session ids; sets by exercise ids)
/// assembled in memory — no N+1, no join row-explosion.
pub async fn find_sessions_by_user(pool: &PgPool, user_id: UserId)
    -> ApiResult<Vec<WorkoutSession>> { /* … see §2.5 … */ }

/// One session if it exists and is owned by the caller, else `None` (→ 404).
pub async fn find_session_by_id(pool: &PgPool, user_id: UserId, id: Uuid)
    -> ApiResult<Option<WorkoutSession>> { /* WHERE id = $1 AND user_id = $2 */ }

/// Full-replace edit within a transaction; `None` when missing/foreign (→ 404).
pub async fn replace_session(pool: &PgPool, user_id: UserId, id: Uuid, new: &NewWorkoutSession)
    -> ApiResult<Option<WorkoutSession>> { /* UPDATE…RETURNING; if none → None; DELETE children; re-insert */ }

/// Delete the caller's session (children cascade); `false` when missing/foreign.
pub async fn delete_session(pool: &PgPool, user_id: UserId, id: Uuid) -> ApiResult<bool> {
    let res = sqlx::query("DELETE FROM workout_sessions WHERE id = $1 AND user_id = $2")
        .bind(id).bind(user_id.0).execute(pool).await?;
    Ok(res.rows_affected() > 0)
}
```

> `i32::try_from(usize)` cannot realistically fail (array length), but is
> surfaced as `ApiError::Internal` via `#[from] std::num::TryFromIntError`
> rather than a panic — honouring the no-unchecked-failure rule (CLAUDE.md §6).
> *This `#[from]` is the one new error-plumbing line; confirm in §3.7-equivalent.*

### 3.7 `backend/crates/api/src/workout/{mod,handlers}.rs`

```rust
// mod.rs — routes (axum 0.7 path syntax `:id`).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/workouts", post(handlers::create).get(handlers::list))
        .route(
            "/workouts/:id",
            get(handlers::get_one).put(handlers::replace).delete(handlers::delete),
        )
}
```

```rust
// handlers.rs — DTOs deserialize raw scalars + typed Option<MuscleGroup>.
#[derive(Debug, Deserialize)]
pub(crate) struct SetRequest { reps: i32, #[serde(default)] weight_kg: Option<f64>,
    #[serde(default)] rpe: Option<f64> }
#[derive(Debug, Deserialize)]
pub(crate) struct ExerciseRequest { name: String, #[serde(default)] muscle_group: Option<MuscleGroup>,
    sets: Vec<SetRequest> }
#[derive(Debug, Deserialize)]
pub(crate) struct SessionRequest { performed_on: NaiveDate, exercises: Vec<ExerciseRequest> }

impl SessionRequest {
    /// Build the validated write model (bottom-up so the first error has the
    /// right `field()`).
    fn into_new(self, today: NaiveDate) -> ApiResult<NewWorkoutSession> {
        let mut exercises = Vec::with_capacity(self.exercises.len());
        for ex in self.exercises {
            let mut sets = Vec::with_capacity(ex.sets.len());
            for s in ex.sets {
                sets.push(NewSet::new(s.reps, s.weight_kg, s.rpe)
                    .map_err(|e| ApiError::Validation { field: e.field() })?);
            }
            exercises.push(NewExercise::new(&ex.name, ex.muscle_group, sets)
                .map_err(|e| ApiError::Validation { field: e.field() })?);
        }
        NewWorkoutSession::new(self.performed_on, exercises, today)
            .map_err(|e| ApiError::Validation { field: e.field() })
    }
}

pub(crate) async fn create(State(s): State<AppState>, user: AuthenticatedUser,
    req: Result<Json<SessionRequest>, JsonRejection>)
    -> ApiResult<(StatusCode, Json<WorkoutSession>)> {
    let new = parse_body(req)?.into_new(Utc::now().date_naive())?;
    let session = db::insert_session(&s.pool, user.user_id, &new).await?;
    Ok((StatusCode::CREATED, Json(session)))
}

pub(crate) async fn list(State(s): State<AppState>, user: AuthenticatedUser)
    -> ApiResult<Json<Vec<WorkoutSession>>> {
    Ok(Json(db::find_sessions_by_user(&s.pool, user.user_id).await?))
}

pub(crate) async fn get_one(State(s): State<AppState>, user: AuthenticatedUser,
    Path(id): Path<Uuid>) -> ApiResult<Json<WorkoutSession>> {
    db::find_session_by_id(&s.pool, user.user_id, id).await?
        .map(Json).ok_or(ApiError::NotFound)
}

pub(crate) async fn replace(State(s): State<AppState>, user: AuthenticatedUser,
    Path(id): Path<Uuid>, req: Result<Json<SessionRequest>, JsonRejection>)
    -> ApiResult<Json<WorkoutSession>> {
    let new = parse_body(req)?.into_new(Utc::now().date_naive())?;
    db::replace_session(&s.pool, user.user_id, id, &new).await?
        .map(Json).ok_or(ApiError::NotFound)
}

pub(crate) async fn delete(State(s): State<AppState>, user: AuthenticatedUser,
    Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if db::delete_session(&s.pool, user.user_id, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}
```

> `parse_body` is the SPEC-0003 §3.9 helper (maps `JsonRejection` → `400` field
> `"body"`). **Resolved (architect, OQ-C5):** promote it to a new generic
> `crate::http` module both `profile` and `workout` import — *not* `crate::error`,
> which must stay free of axum extractor types (it only knows `IntoResponse`):
>
> ```rust
> // crate::http
> pub(crate) fn parse_body<T>(req: Result<Json<T>, JsonRejection>) -> ApiResult<T> {
>     req.map(|Json(r)| r).map_err(|_| ApiError::Validation { field: "body" })
> }
> ```
>
> `profile/handlers.rs` drops its private copy and imports this (declared in the
> Module(s) line).

### 3.8 `backend/crates/api/src/lib.rs` (extended)

```rust
pub(crate) mod http; // NEW — the promoted parse_body helper (OQ-C5)
pub mod workout; // NEW

pub fn app(state: AppState) -> Router {
    Router::new()
        .merge(health::router())
        .merge(auth::routes())
        .merge(profile::routes())
        .merge(workout::routes()) // NEW
        .with_state(state)
}
```

`ApiError` already carries `NotFound` (added in R-0003); the only error addition
is `#[from] std::num::TryFromIntError` → `Internal` for the `i32::try_from`
position casts (§3.6).

## 4. Non-goals

- Curated exercise catalog / canonical names / equipment — later R (req §4).
- Derived metrics (volume, frequency, `%1RM`, per-muscle-group rollups) — M5.
- `PATCH` / partial update — editing is full-replace `PUT` (req §4).
- Pagination / date-range / muscle filtering on the list endpoint — later.
- Tempo, rest, set type, supersets, session notes — deferred (req §4).
- Stable child (exercise/set) ids across an edit — full-replace assigns new ids.
- Imperial units — canonical metric only (kg), as R-0003.

## 5. Open questions

Owner-level OQ1–OQ4 are settled in the requirement. The five design-level
questions below were **resolved by the `architect` review (2026-05-31,
APPROVE WITH NITS)** — all in favour of the proposal, with OQ-C5 refined. They
are recorded here and folded into §2/§3 above; status is now `Accepted`.

- **OQ-C1 — Response shape. RESOLVED → serialize the `core` aggregate directly.**
  No derived field; a parallel DTO tree would be pure restatement. SAC7 asserts
  literal JSON keys to pin the (now coupled) wire contract. (§2.4)
- **OQ-C2 — List assembly. RESOLVED → three batched queries + in-memory grouping**,
  with per-parent `position` ordering and an empty-id short-circuit. (§2.5)
- **OQ-C3 — `position`. RESOLVED → server-assigned from array index**, contiguous
  0-based per parent, checked `i32::try_from`. (§2.6)
- **OQ-C4 — Edit strategy. RESOLVED → transactional full-replace, new child ids**
  (matches AC5 + owner's full-CRUD decision; diffing unjustified now). (§2.5/§4)
- **OQ-C5 — `parse_body` location. RESOLVED → promote to a new generic
  `crate::http` module** (not `crate::error`, which stays axum-extractor-free);
  `profile/handlers.rs` imports it. (§3.7/§3.8)

## 6. Acceptance criteria

Each maps 1:1 to an R-0004 acceptance criterion and to the qa agent's test.

- [ ] **SAC1 → AC1.** `00003_workout_logs.sql` creates the three tables with the
  specified columns/FKs/cascades; clean-DB migration succeeds; user- and
  session-level cascade deletes verified.
- [ ] **SAC2 → AC2.** `POST /workouts` → `201` + stored nested session (server
  ids); persists the hierarchy owned by the caller; `401` unauthorized.
- [ ] **SAC3 → AC3.** `GET /workouts` → `200` + caller-only sessions, newest
  `performed_on` first; empty array when none; `401`.
- [ ] **SAC4 → AC4.** `GET /workouts/:id` → `200` owned; `404` missing or
  foreign; `401`.
- [ ] **SAC5 → AC5.** `PUT /workouts/:id` → `200` full-replace + `updated_at`
  bump; `404` missing/foreign; `400` invalid (writes nothing); `401`.
- [ ] **SAC6 → AC6.** `DELETE /workouts/:id` → `204`, second delete `404`;
  `404` foreign; `401`.
- [ ] **SAC7 → AC7.** Response carries the full nested shape with the specified
  fields and nullable `muscle_group`/`weight_kg`/`rpe`.
- [ ] **SAC8 → AC8.** Every validation branch → `400`, nothing written. The
  field-label asymmetry is pinned per branch: **semantic** failures report the
  leaf field (`exercises` present-but-`[]` → `"exercises"`; `sets` empty →
  `"sets"`; bad `reps`/`weight_kg`/`rpe`/`name`/`performed_on` → that field),
  while **structural** failures report `"body"` (missing `performed_on`,
  `exercises` missing or non-array, missing `reps`, unknown `muscle_group`).
- [ ] **SAC9 → AC9.** `MuscleGroup` is the single authority; serde and
  `as_str`/`parse` encodings pinned equal by an exhaustive unit test.
- [ ] **SAC10 → AC10.** Cross-user isolation: foreign id → `404` on
  get/put/delete; list never returns another user's sessions.
- [ ] **SAC11 → AC11.** `core::workout` carries unit tests for every AC8/AC9
  rule and stays free of `sqlx`/`axum`/HTTP.
- [ ] **SAC12 → AC12.** ≥ 14 `#[sqlx::test]` integration tests cover the surface,
  including — as the codebase's first transaction — at least one **partial-write
  rollback** test asserting that a failed multi-table insert/replace leaves zero
  rows (the DB-error boundary, beyond AC5/AC8's validation-boundary "writes
  nothing").

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-05-31 | **Workout domain (`WorkoutSession`/`NewWorkoutSession` + value types) in `crates/core`; persistence in `api::db`; HTTP in `api::workout`.** | Continues the R-0002/R-0003 layering; pure reusable validation, auditable seam, thin handlers. |
| 2026-05-31 | **All validation in `core`; no DB `CHECK`s** (SPEC-0002 OQ-A1). | Single source of truth; the DB enforces only referential integrity + ordering indexes. |
| 2026-05-31 | **`insert_session`/`replace_session` use a `sqlx` transaction** (first in the codebase). | Three tables must commit atomically; partial writes would corrupt the aggregate. |
| 2026-05-31 | **Typed serde `Option<MuscleGroup>` in the request; unknown → `400` `"body"`.** | Makes `MuscleGroup` the AC9 authority; rejects bad values before handler logic, as R-0003 does for `Goal`/`Sex`. |
| 2026-05-31 | **Cross-user access → `404`, never `403`.** | Enumeration-safety, consistent with R-0003 AC7 / R-0002. |
| 2026-05-31 | **`position` server-assigned from request array index.** | Deterministic order; client controls it by array order; nothing client-trusted beyond order. (OQ-C3) |
| 2026-05-31 | **Edit = full-replace (delete children + re-insert) in a tx; child ids not stable.** | Unambiguous mutation of a nested aggregate; matches the owner's full-CRUD-with-replace choice; diffing is unjustified complexity now. (OQ-C4) |
| 2026-05-31 | **`reps INTEGER`; `weight_kg`/`rpe` `DOUBLE PRECISION`; `LoadKg` newtype distinct from profile `WeightKg`.** | f64 suffices at 0.1/0.5 resolution (no `rust_decimal`); lifted load has different range/semantics from body weight. |
| 2026-05-31 | **`i32::try_from` position casts surface `TryFromIntError` → `Internal`, not panic.** | No unchecked failures in library code (CLAUDE.md §6); the cast is unreachable in practice but handled explicitly. |
| 2026-05-31 | **No new crate dependencies.** | Reuses sqlx (tx, `NaiveDate`, native arrays via `ANY`), chrono, serde, uuid. |
| 2026-05-31 | **SPEC-0001 §7 lockstep snippet policy remains in force.** | Pedantic/fmt deviations under 1.95.0 patched in spec + impl together. |
| 2026-05-31 | **(architect, OQ-C5) `parse_body` promoted to a new generic `crate::http` module, not `crate::error`.** | `error.rs` must stay free of axum extractor types (only `IntoResponse`); a `http` helper keeps the dependency direction right. Touches the R-0003 `profile/handlers.rs`. |
| 2026-05-31 | **(architect, OQ-C2) list nests children in `position` order and short-circuits the child queries when the user has no sessions.** | Avoids unstable nested order and a degenerate `ANY('{}')` bind. |
| 2026-05-31 | **(architect) ≥1 partial-write rollback test in the qa suite.** | First transaction in the codebase; assert a mid-insert failure leaves zero rows (DB-error boundary, beyond the validation boundary). |
| 2026-05-31 | **(architect, OQ-C1) serialize the `core` aggregate directly; SAC7 asserts literal JSON keys.** | No derived field to justify a DTO tree; the literal-key assertion pins the coupled wire contract. |

## Changelog

- _2026-05-31 — created (Draft). Realizes the accepted R-0004 (OQ1–OQ4 + derived calls inherited). Five design questions (OQ-C1..C5) raised for the architect review. Pending `architect` review before status → Accepted._
- _2026-05-31 — **Accepted.** `architect` review returned APPROVE WITH NITS. All five OQs resolved (OQ-C1/C2/C3/C4 as proposed; OQ-C5 refined to a new `crate::http` module rather than `crate::error`). Applied the four nits in lockstep: SAC8 field-label asymmetry pinned per branch (§6); `exercise_from_row` muscle_group corruption mapper added (§3.6); Module(s) line now lists the new `crate::http` helper + the edited `profile/handlers.rs`; list nested-child ordering + empty-id short-circuit specified (§2.5); partial-write rollback test added to SAC12. Decision log + §5 updated._
