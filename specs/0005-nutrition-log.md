# SPEC-0005 — Nutrition log

- **Status:** Implemented
- **Realizes:** R-0005
- **Author:** Claude (main session), with owner
- **Created:** 2026-05-31
- **Depends on:** SPEC-0002 (Implemented), SPEC-0003 (Implemented), SPEC-0004 (Implemented) — reuses `AppState`, the `AuthenticatedUser` extractor, `ApiError` (incl. `NotFound`, `AlreadyExists` → 409, the unique-violation auto-map), the `db` seam and its `into_*` corruption discipline, the `crate::http::parse_body` helper, the parse-don't-validate core layering, and the migration/CI/dev-DB machinery
- **Module(s):** `backend/crates/core/nutrition` (new), `backend/crates/api/nutrition` (new), `backend/crates/api/{db,lib}` (extended), `backend/migrations/` (new file)

## 1. Motivation

Realizes [R-0005](../requirements/0005-nutrition-log.md): an authenticated user
can create, read, edit, and delete their own daily **nutrition logs** — the
second primary signal (alongside R-0004's workout log) the ML response-inference
engine (M5) consumes. A log is a flat, **per-day** record (one row per
`(user_id, performed_on)`) capturing the three macronutrients in grams; total
**calories are derived** (`4·protein + 4·carbs + 9·fat`), never stored. The
collection is full CRUD under `/nutrition`, owned by the token's `sub`.

R-0005 is deliberately **simpler than R-0004**: one table (no nested hierarchy,
no transaction, no in-memory assembly), built entirely on the R-0002/R-0003/
R-0004 primitives. Its one novelty is a **derived response field** — `calories`
— which reuses the R-0003 `ProfileResponse`/`age` pattern (a response DTO that
adds a computed field), not the R-0004 pattern (serialize the aggregate
directly, which had no derived field).

## 2. Design

### 2.1 Shape

One table, one row per user per day, FK `ON DELETE CASCADE` so deleting a user
removes their logs; a `UNIQUE (user_id, performed_on)` constraint enforces the
per-day grain:

```
users (R-0002)
  └── nutrition_logs (id, user_id→users, performed_on,
                      protein_g, carbs_g, fat_g, created_at, updated_at)
                      UNIQUE (user_id, performed_on)
```

The ML layer (M5) consumes daily macros and total calories directly. R-0005
stores macros **verbatim** in grams; `calories` is the only derived value and is
computed on read by a single `core` authority — never stored, so it cannot drift
from the macros that determine it.

### 2.2 Layering (preserves the R-0002/R-0003/R-0004 purity boundary)

- **`core::nutrition`** (pure — no `sqlx`/`axum`/HTTP):
  - value type: `Grams` (validated numeric newtype, `[0, 2000]`);
  - value group: `Macros { protein, carbs, fat }` with the **`calories()`**
    derivation (the AC9 single authority);
  - **write model** (validated, no identity/timestamps): `NewNutritionLog` —
    built through `::new(..)` returning `Result<_, NutritionError>`; `::new`
    takes `today` (injected) for the `performed_on`-not-future check;
  - **read aggregate** (reconstructed from a row, *not* `Serialize` — see §2.4):
    `NutritionLog` — carries the server-assigned `Uuid` id, `user_id`,
    `performed_on`, `Macros`, and timestamps; exposes `calories()` by delegating
    to `Macros`;
  - typed `NutritionError` with a `.field()` method naming the offending request
    field (drives `ApiError::Validation { field }`).
- **`api::db`** (persistence seam): `NutritionRow` (`FromRow`) and the five
  queries (`insert_nutrition_log`, `find_nutrition_logs_by_user`,
  `find_nutrition_log_by_id`, `update_nutrition_log`, `delete_nutrition_log`).
  The row maps back to the core aggregate via `into_nutrition_log`; a stored
  value that fails domain validation is data corruption → logged 500 (the
  `into_profile`/`into_user`/`corrupt` discipline).
- **`api::nutrition`** (HTTP): the `NutritionRequest` DTO, the `NutritionResponse`
  DTO (adds derived `calories`), the five handlers, and `routes()`. Validation is
  `core`'s job; handlers are thin.

### 2.3 Request parsing & validation

`NutritionRequest` deserializes **raw** scalars (`performed_on: NaiveDate`,
`protein_g`/`carbs_g`/`fat_g: f64`). The handler calls
`NewNutritionLog::new(req, today)`, which validates every rule in AC8 and returns
the first `NutritionError`; the handler maps it to
`ApiError::Validation { field: e.field() }` (`400`, nothing written). A
malformed/missing body (missing `performed_on`, a macro field absent or
non-numeric) is a `JsonRejection` → `400` field `"body"` via `parse_body` (the
structural-vs-semantic field-label asymmetry from SPEC-0003 §2.3 / SPEC-0004
§2.3: structural failures report `"body"`, semantic failures report the leaf
field).

**Macro field attribution.** All three macros are the same `Grams` newtype, so a
fieldless `Grams::try_new` could not tell the handler *which* macro was out of
range. `Macros::new(protein_g, carbs_g, fat_g)` is therefore the validating
constructor: it runs each value through `Grams::try_new` and, on failure, tags
the error with the offending field (`"protein_g"` / `"carbs_g"` / `"fat_g"`) so
`NutritionError::field()` is accurate per AC8. *(Design question OQ-C2 — see §5.)*

### 2.4 Response shape — derived `calories`

The wire contract (AC7) includes `calories`, which is **not** a stored column and
**not** a field on the `NutritionLog` aggregate. Following the R-0003
`ProfileResponse`/`age` precedent (`profile/handlers.rs`), the HTTP layer owns a
dedicated `NutritionResponse` DTO that flattens `Macros` to `protein_g`/
`carbs_g`/`fat_g` and adds `calories` computed via the aggregate's `calories()`
method:

```rust
#[derive(Debug, Serialize)]
pub(crate) struct NutritionResponse {
    id: Uuid,
    user_id: UserId,
    performed_on: NaiveDate,
    protein_g: f64,
    carbs_g: f64,
    fat_g: f64,
    calories: f64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

Because `calories` is derived, the `core` aggregate is intentionally **not**
`Serialize` (unlike R-0004's aggregates) — the response DTO is the sole wire
authority, and `calories()` is the sole calorie authority (AC9). The SAC7 test
asserts the **literal JSON keys** (incl. `calories`) to pin the contract.
*(Design question OQ-C1 — see §5.)*

### 2.5 Persistence

A nutrition log is a single row, so — unlike R-0004 — **no transaction** is
needed; each query is one statement.

- **`insert_nutrition_log` (AC2):** plain `INSERT … RETURNING`. A
  `(user_id, performed_on)` unique violation is matched explicitly and mapped to
  `ApiError::AlreadyExists` (→ `409`), exactly as `insert_user` maps a duplicate
  email. Nothing is written on conflict.
- **`find_nutrition_logs_by_user` (AC3):** `SELECT … WHERE user_id = $1 ORDER BY
  performed_on DESC, created_at DESC`; maps each row via `into_nutrition_log`.
  Empty result → `[]`.
- **`find_nutrition_log_by_id` (AC4):** `SELECT … WHERE id = $1 AND user_id = $2`
  → `None` (→ `404`) when missing **or** owned by another user. Ownership is
  never leaked via a distinct status.
- **`update_nutrition_log` (AC5):** `UPDATE nutrition_logs SET performed_on = $1,
  protein_g = $2, carbs_g = $3, fat_g = $4, updated_at = NOW() WHERE id = $5 AND
  user_id = $6 RETURNING …`; `fetch_optional` → `None` (→ `404`) when missing/
  foreign. If the new `performed_on` collides with another of the caller's logs,
  the unique constraint raises a `sqlx` unique violation → `ApiError::Database`,
  which `ApiError::into_response` already maps to `AlreadyExists`/`409` ("surfaces
  here when callers didn't pre-check"). No pre-check query is issued.
  *(Design question OQ-C3 — see §5.)*
- **`delete_nutrition_log` (AC6):** `DELETE … WHERE id = $1 AND user_id = $2`;
  `rows_affected() > 0` → `204`, else `404`. A second delete → `404`.

Every id-addressed query carries `AND user_id = $caller`; no id from the path is
ever trusted as an owner (AC10).

### 2.6 SQL & domain types

`protein_g`/`carbs_g`/`fat_g` are `DOUBLE PRECISION`/`f64` (allows fractional
grams, e.g. `0.5`; no `rust_decimal`, consistent with SPEC-0004 §2.7's
`weight_kg`/`rpe`). `performed_on DATE`/`NaiveDate`. Id `UUID`/`Uuid` generated
app-side (`Uuid::new_v4()`). `created_at`/`updated_at TIMESTAMPTZ DEFAULT NOW()`.
Validation lives in `core`, never DB `CHECK`s (SPEC-0002 OQ-A1); the DB enforces
referential integrity, the per-day unique constraint, and a `user_id` index for
the list lookup and cascade. Calories have **no column** — derived only.
*(Design question OQ-C5 — see §5.)*

## 3. Code outline

Snippets are representative (final form reconciled in step-5 lockstep with the
pinned 1.95.0 toolchain, SPEC-0001 §7 policy). Tests are authored by `qa` in
step 3 against §6.

### 3.1 `backend/migrations/00004_nutrition_logs.sql`

```sql
-- R-0005 / SPEC-0005 — nutrition log (one row per user per day).
-- Validation lives in crates/core (SPEC-0002 OQ-A1); the DB enforces
-- referential integrity, per-day uniqueness, and the list-lookup index only.
-- Calories are derived (4·protein + 4·carbs + 9·fat), never stored.

CREATE TABLE nutrition_logs (
    id           UUID PRIMARY KEY,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    performed_on DATE NOT NULL,
    protein_g    DOUBLE PRECISION NOT NULL,
    carbs_g      DOUBLE PRECISION NOT NULL,
    fat_g        DOUBLE PRECISION NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, performed_on)
);
CREATE INDEX idx_nutrition_logs_user_id ON nutrition_logs (user_id);
```

### 3.2 `backend/crates/core/src/lib.rs` (extended)

```rust
pub mod nutrition;
pub mod profile;
pub mod user;
pub mod workout;

pub use nutrition::{Grams, Macros, NewNutritionLog, NutritionError, NutritionLog};
// … existing profile / user / workout re-exports unchanged …
```

### 3.3 `core/src/nutrition.rs` — `Grams`, `Macros`, `NutritionError`

```rust
//! Nutrition-log domain: the `NutritionLog` aggregate, its `Macros` value group,
//! and the calorie derivation. Pure — no DB, no HTTP. Parse-don't-validate.

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use crate::UserId;

/// A macronutrient mass in grams, range [0, 2000]. `0` is valid (e.g. a
/// zero-fat day); negatives and absurd values are rejected.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Grams(f64);

impl Grams {
    pub const MAX: f64 = 2000.0;

    /// Validate a macro mass, tagging failures with `field` so the caller can
    /// report which macro was out of range (three fields share this newtype).
    ///
    /// # Errors
    /// [`NutritionError::MacroOutOfRange`] when not finite, `< 0`, or `> 2000`.
    pub fn try_new(value: f64, field: &'static str) -> Result<Self, NutritionError> {
        if value.is_finite() && (0.0..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(NutritionError::MacroOutOfRange { field })
        }
    }

    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// The three macronutrients and their derived energy. Calorie derivation lives
/// here as the single authority (AC9): `4·protein + 4·carbs + 9·fat`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Macros {
    pub protein: Grams,
    pub carbs: Grams,
    pub fat: Grams,
}

impl Macros {
    const KCAL_PER_G_PROTEIN: f64 = 4.0;
    const KCAL_PER_G_CARB: f64 = 4.0;
    const KCAL_PER_G_FAT: f64 = 9.0;

    /// Validate the three macros, attributing range failures to their field.
    ///
    /// # Errors
    /// The first [`NutritionError::MacroOutOfRange`] among protein/carbs/fat.
    pub fn new(protein_g: f64, carbs_g: f64, fat_g: f64) -> Result<Self, NutritionError> {
        Ok(Self {
            protein: Grams::try_new(protein_g, "protein_g")?,
            carbs: Grams::try_new(carbs_g, "carbs_g")?,
            fat: Grams::try_new(fat_g, "fat_g")?,
        })
    }

    /// Total energy in kilocalories (the AC9 single authority).
    #[must_use]
    pub fn calories(&self) -> f64 {
        Self::KCAL_PER_G_PROTEIN * self.protein.get()
            + Self::KCAL_PER_G_CARB * self.carbs.get()
            + Self::KCAL_PER_G_FAT * self.fat.get()
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum NutritionError {
    #[error("performed_on is in the future")]
    PerformedOnInFuture,
    #[error("macro `{field}` is outside the allowed range")]
    MacroOutOfRange { field: &'static str },
}

impl NutritionError {
    /// The request field this error concerns — drives `ApiError::Validation`.
    #[must_use]
    pub fn field(&self) -> &'static str {
        match self {
            NutritionError::PerformedOnInFuture => "performed_on",
            NutritionError::MacroOutOfRange { field } => field,
        }
    }
}
```

### 3.4 `core/src/nutrition.rs` — write model

```rust
/// A validated nutrition log (no identity/timestamps).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NewNutritionLog {
    pub performed_on: NaiveDate,
    pub macros: Macros,
}

impl NewNutritionLog {
    /// `today` is injected for a deterministic future-date check.
    ///
    /// # Errors
    /// [`NutritionError::PerformedOnInFuture`] or the first macro range error.
    pub fn new(
        performed_on: NaiveDate,
        protein_g: f64,
        carbs_g: f64,
        fat_g: f64,
        today: NaiveDate,
    ) -> Result<Self, NutritionError> {
        if performed_on > today {
            return Err(NutritionError::PerformedOnInFuture);
        }
        Ok(Self {
            performed_on,
            macros: Macros::new(protein_g, carbs_g, fat_g)?,
        })
    }
}
```

> The future-date check precedes macro validation so a future date reports
> `"performed_on"` even when a macro is also invalid — a deterministic,
> documented precedence (mirrors `NewWorkoutSession::new`).

### 3.5 `core/src/nutrition.rs` — read aggregate

```rust
/// A stored nutrition log, reconstructed from a row. Not `Serialize`: the wire
/// shape carries a derived `calories` the aggregate doesn't store, so the HTTP
/// `NutritionResponse` DTO owns serialization (cf. R-0003 `ProfileResponse`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NutritionLog {
    pub id: Uuid,
    pub user_id: UserId,
    pub performed_on: NaiveDate,
    pub macros: Macros,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NutritionLog {
    /// Derived total energy (delegates to the `Macros` authority).
    #[must_use]
    pub fn calories(&self) -> f64 {
        self.macros.calories()
    }
}
```

### 3.6 `backend/crates/api/src/db.rs` — row + queries (extended)

```rust
use fitai_core::{Grams, Macros, NewNutritionLog, NutritionLog /* …existing… */};

#[derive(Debug, FromRow)]
pub struct NutritionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub performed_on: NaiveDate,
    pub protein_g: f64,
    pub carbs_g: f64,
    pub fat_g: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NutritionRow {
    /// Reconstruct the domain `NutritionLog`. A stored macro that fails domain
    /// validation is data corruption → logged 500 (the `into_profile`
    /// discipline).
    ///
    /// # Errors
    /// [`ApiError::Internal`] when a stored macro fails domain validation.
    pub fn into_nutrition_log(self) -> ApiResult<NutritionLog> {
        // Reuses the existing free `corrupt(id, what)` helper (db.rs) shared by
        // the workout `set_from_row`/`exercise_from_row` mappers — no parallel
        // closure.
        let id = self.id;
        let macros = Macros {
            protein: Grams::try_new(self.protein_g, "protein_g").map_err(|_| corrupt(id, "protein_g"))?,
            carbs: Grams::try_new(self.carbs_g, "carbs_g").map_err(|_| corrupt(id, "carbs_g"))?,
            fat: Grams::try_new(self.fat_g, "fat_g").map_err(|_| corrupt(id, "fat_g"))?,
        };
        Ok(NutritionLog {
            id: self.id,
            user_id: UserId(self.user_id),
            performed_on: self.performed_on,
            macros,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Insert a nutrition log; maps a per-day unique violation to `AlreadyExists`.
///
/// # Errors
/// [`ApiError::AlreadyExists`] (→ 409) when the caller already has a log for
/// that date; [`ApiError::Database`] on any other query failure.
pub async fn insert_nutrition_log(
    pool: &PgPool,
    user_id: UserId,
    new: &NewNutritionLog,
) -> ApiResult<NutritionLog> {
    let id = Uuid::new_v4();
    let result = sqlx::query_as::<_, NutritionRow>(
        "INSERT INTO nutrition_logs (id, user_id, performed_on, protein_g, carbs_g, fat_g) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, user_id, performed_on, protein_g, carbs_g, fat_g, created_at, updated_at",
    )
    .bind(id).bind(user_id.0).bind(new.performed_on)
    .bind(new.macros.protein.get()).bind(new.macros.carbs.get()).bind(new.macros.fat.get())
    .fetch_one(pool)
    .await;

    match result {
        Ok(row) => row.into_nutrition_log(),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Err(ApiError::AlreadyExists),
        Err(e) => Err(ApiError::Database(e)),
    }
}

/// All of the caller's logs, newest `performed_on` first.
pub async fn find_nutrition_logs_by_user(pool: &PgPool, user_id: UserId)
    -> ApiResult<Vec<NutritionLog>> { /* SELECT … WHERE user_id = $1 ORDER BY performed_on DESC, created_at DESC */ }

/// One log if it exists and is owned by the caller, else `None` (→ 404).
pub async fn find_nutrition_log_by_id(pool: &PgPool, user_id: UserId, id: Uuid)
    -> ApiResult<Option<NutritionLog>> { /* WHERE id = $1 AND user_id = $2 */ }

/// Full-replace edit; `None` when missing/foreign (→ 404). A `performed_on`
/// collision with another of the caller's logs surfaces as a unique violation,
/// auto-mapped to `AlreadyExists`/409 by `ApiError::into_response` (error.rs) —
/// no pre-check query is issued (SPEC-0005 §2.5 / OQ-C3).
pub async fn update_nutrition_log(pool: &PgPool, user_id: UserId, id: Uuid, new: &NewNutritionLog)
    -> ApiResult<Option<NutritionLog>> { /* UPDATE … WHERE id AND user_id RETURNING …; fetch_optional */ }

/// Delete the caller's log; `false` when missing/foreign (→ 404).
pub async fn delete_nutrition_log(pool: &PgPool, user_id: UserId, id: Uuid) -> ApiResult<bool> {
    let result = sqlx::query("DELETE FROM nutrition_logs WHERE id = $1 AND user_id = $2")
        .bind(id).bind(user_id.0).execute(pool).await?;
    Ok(result.rows_affected() > 0)
}
```

### 3.7 `backend/crates/api/src/nutrition/{mod,handlers}.rs`

```rust
// mod.rs — routes (axum 0.7 path syntax `:id`).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/nutrition", post(handlers::create).get(handlers::list))
        .route(
            "/nutrition/:id",
            get(handlers::get_one).put(handlers::replace).delete(handlers::delete),
        )
}
```

```rust
// handlers.rs
#[derive(Debug, Deserialize)]
pub(crate) struct NutritionRequest {
    performed_on: NaiveDate,
    protein_g: f64,
    carbs_g: f64,
    fat_g: f64,
}

impl NutritionRequest {
    fn into_new(self, today: NaiveDate) -> ApiResult<NewNutritionLog> {
        NewNutritionLog::new(self.performed_on, self.protein_g, self.carbs_g, self.fat_g, today)
            .map_err(|e| ApiError::Validation { field: e.field() })
    }
}

// NutritionResponse (see §2.4) adds derived `calories`.
impl NutritionResponse {
    fn from_log(l: &NutritionLog) -> Self {
        Self {
            id: l.id, user_id: l.user_id, performed_on: l.performed_on,
            protein_g: l.macros.protein.get(), carbs_g: l.macros.carbs.get(),
            fat_g: l.macros.fat.get(), calories: l.calories(),
            created_at: l.created_at, updated_at: l.updated_at,
        }
    }
}

pub(crate) async fn create(State(s): State<AppState>, user: AuthenticatedUser,
    req: Result<Json<NutritionRequest>, JsonRejection>)
    -> ApiResult<(StatusCode, Json<NutritionResponse>)> {
    let new = parse_body(req)?.into_new(Utc::now().date_naive())?;
    let log = db::insert_nutrition_log(&s.pool, user.user_id, &new).await?;
    Ok((StatusCode::CREATED, Json(NutritionResponse::from_log(&log))))
}

pub(crate) async fn list(State(s): State<AppState>, user: AuthenticatedUser)
    -> ApiResult<Json<Vec<NutritionResponse>>> {
    let logs = db::find_nutrition_logs_by_user(&s.pool, user.user_id).await?;
    Ok(Json(logs.iter().map(NutritionResponse::from_log).collect()))
}

pub(crate) async fn get_one(State(s): State<AppState>, user: AuthenticatedUser,
    Path(id): Path<Uuid>) -> ApiResult<Json<NutritionResponse>> {
    db::find_nutrition_log_by_id(&s.pool, user.user_id, id).await?
        .map(|l| Json(NutritionResponse::from_log(&l))).ok_or(ApiError::NotFound)
}

pub(crate) async fn replace(State(s): State<AppState>, user: AuthenticatedUser,
    Path(id): Path<Uuid>, req: Result<Json<NutritionRequest>, JsonRejection>)
    -> ApiResult<Json<NutritionResponse>> {
    let new = parse_body(req)?.into_new(Utc::now().date_naive())?;
    db::update_nutrition_log(&s.pool, user.user_id, id, &new).await?
        .map(|l| Json(NutritionResponse::from_log(&l))).ok_or(ApiError::NotFound)
}

pub(crate) async fn delete(State(s): State<AppState>, user: AuthenticatedUser,
    Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if db::delete_nutrition_log(&s.pool, user.user_id, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}
```

### 3.8 `backend/crates/api/src/lib.rs` (extended)

```rust
pub mod nutrition; // NEW

pub fn app(state: AppState) -> Router {
    Router::new()
        .merge(health::router())
        .merge(auth::routes())
        .merge(profile::routes())
        .merge(workout::routes())
        .merge(nutrition::routes()) // NEW
        .with_state(state)
}
```

No `ApiError` variant is added — `NotFound`, `AlreadyExists` (→ 409), `Validation`,
and the unique-violation auto-map already exist (R-0002/R-0003).

## 4. Non-goals

- Barcode scan / food-database lookup — manual entry only (req §4).
- Per-meal breakdown — one aggregate row per day (req §4).
- Micronutrients, fiber, sugar, sodium, water, alcohol — deferred (req §4).
- Stored / label-value calories — calories are always derived (req §4).
- Macro targets / goals / adherence scoring — M5, R-0017+ (req §4).
- `PATCH` / partial update — editing is full-replace `PUT` (req §4).
- Pagination / date-range filtering on the list endpoint — later (req §4).
- Imperial units / alternate energy units — grams + kcal only (req §4).

## 5. Open questions

Owner-level forks (per-day grain; calories derived 4/4/9, never stored) are
settled in the requirement. The five design-level questions below were
**resolved by the `architect` review (2026-05-31, APPROVE WITH NITS)** — all
five approved as proposed. They are folded into §2/§3 above; status is now
`Accepted`.

- **OQ-C1 — Response shape. RESOLVED → dedicated `NutritionResponse` DTO; core
  aggregate not `Serialize`.** Unlike R-0004 (no derived field), R-0005 has a
  genuine wire field absent from the aggregate, so the DTO is the only honest
  way to add it — exactly the R-0003 `ProfileResponse`/`age` precedent. SAC7
  pins the literal JSON keys. (§2.4)
- **OQ-C2 — Macro range-error attribution. RESOLVED → `Grams::try_new(value,
  field)` + single `NutritionError::MacroOutOfRange { field }`.** Lighter than
  three near-identical variants; `field` is a compile-time constant supplied
  only by `Macros::new`, so there is no stringly-typed-domain risk. (§2.3/§3.3)
- **OQ-C3 — `PUT` date-collision. RESOLVED → rely on the unique constraint +
  `into_response` auto-map; `POST` matches the violation explicitly like
  `insert_user`.** The asymmetry is deliberate and pre-existing; both paths
  yield 409 and write nothing, and `PUT` avoids a redundant (TOCTOU-prone)
  pre-check query. (§2.5)
- **OQ-C4 — Home of `calories()`. RESOLVED → on `Macros`, delegated by
  `NutritionLog`.** Correct SRP placement: calories are a pure function of the
  three grams; the named `KCAL_PER_G_*` consts keep the Atwater factors in one
  place. (§3.3/§3.5)
- **OQ-C5 — `Grams` numeric type. RESOLVED → `f64`/`DOUBLE PRECISION`, no
  `rust_decimal`.** Consistent with SPEC-0004/SPEC-0003; `is_finite()` guards
  NaN/Inf; rounding is presentational and the model tolerates it. (§2.6)

## 6. Acceptance criteria

Each maps 1:1 to an R-0005 acceptance criterion and to the qa agent's test.

- [ ] **SAC1 → AC1.** `00004_nutrition_logs.sql` creates `nutrition_logs` with
  the specified columns, the `(user_id, performed_on)` unique constraint, the FK
  cascade, and **no `calories` column**; clean-DB migration succeeds; user-level
  cascade delete verified.
- [ ] **SAC2 → AC2.** `POST /nutrition` → `201` + stored log incl. derived
  `calories`; persists owned by the caller; `409` on duplicate date (nothing
  written); `401` unauthorized.
- [ ] **SAC3 → AC3.** `GET /nutrition` → `200` + caller-only logs, newest
  `performed_on` first; empty array when none; `401`.
- [ ] **SAC4 → AC4.** `GET /nutrition/:id` → `200` owned; `404` missing or
  foreign; `401`.
- [ ] **SAC5 → AC5.** `PUT /nutrition/:id` → `200` full-replace + `updated_at`
  bump (recomputed `calories`); `404` missing/foreign; `409` date-collision;
  `400` invalid (writes nothing); `401`.
- [ ] **SAC6 → AC6.** `DELETE /nutrition/:id` → `204`, second delete `404`;
  `404` foreign; `401`.
- [ ] **SAC7 → AC7.** Response carries the literal keys `id`, `user_id`,
  `performed_on`, `protein_g`, `carbs_g`, `fat_g`, `calories`, `created_at`,
  `updated_at`; `calories` equals `4·protein + 4·carbs + 9·fat`.
- [ ] **SAC8 → AC8.** Every validation branch → `400`, nothing written. The
  structural-vs-semantic field-label asymmetry is pinned per branch, including
  the same field under both outcomes:
  - **semantic** (present-but-invalid value) → the leaf field: future
    `performed_on` → `"performed_on"`; a present macro `< 0` or `> 2000` →
    its field (`"protein_g"`/`"carbs_g"`/`"fat_g"`);
  - **structural** (shape failure) → `"body"`: missing `performed_on`; a macro
    **absent**; a macro **present-but-non-numeric** (e.g. a string).

  QA pins both outcomes for the same field — e.g. `protein_g: -1` → `400`
  `"protein_g"` versus `protein_g: "x"` → `400` `"body"`.
- [ ] **SAC9 → AC9.** `Macros::calories()` is the single calorie authority; no
  calories value is read from request or DB; a unit test pins the 4/4/9 formula
  (incl. a fractional-gram case).
- [ ] **SAC10 → AC10.** Cross-user isolation: foreign id → `404` on
  get/put/delete; list never returns another user's logs.
- [ ] **SAC11 → AC11.** `core::nutrition` carries unit tests for every AC8 rule
  and the AC9 formula, and stays free of `sqlx`/`axum`/HTTP.
- [ ] **SAC12 → AC12.** ≥ 12 `#[sqlx::test]` integration tests cover the surface,
  including `POST` duplicate-date `409` and `PUT` date-collision `409`.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-05-31 | **Nutrition domain (`NutritionLog`/`NewNutritionLog` + `Macros`/`Grams`) in `crates/core`; persistence in `api::db`; HTTP in `api::nutrition`.** | Continues the R-0002/R-0003/R-0004 layering; pure reusable validation, auditable seam, thin handlers. |
| 2026-05-31 | **Single table, no transaction.** | One row per log; nothing to make atomic across statements (unlike R-0004's three tables). Simpler than R-0004 by design. |
| 2026-05-31 | **Per-day grain via `UNIQUE (user_id, performed_on)`; `POST` dup → 409, `PUT` collision → 409.** | Enforces the owner's per-day decision in the DB; reuses the existing `AlreadyExists`/409 machinery (explicit match on insert, auto-map on update). |
| 2026-05-31 | **Calories derived (`Macros::calories()`), never stored; no `calories` column.** | Owner decision; macros are the single source of truth, removing macro/calorie drift. |
| 2026-05-31 | **Derived `calories` served via a `NutritionResponse` DTO; core aggregate not `Serialize`.** | Mirrors R-0003 `ProfileResponse`/`age`; the wire shape owns the derived field. (OQ-C1) |
| 2026-05-31 | **`Grams::try_new(value, field)` + single `NutritionError::MacroOutOfRange { field }`.** | Three same-typed macro fields need distinct error attribution for AC8 without three near-identical variants. (OQ-C2) |
| 2026-05-31 | **`calories()` lives on `Macros`, delegated by `NutritionLog`.** | One AC9 authority reachable from the read aggregate and any future use. (OQ-C4) |
| 2026-05-31 | **`protein_g`/`carbs_g`/`fat_g` `DOUBLE PRECISION`/`f64`; no `rust_decimal`.** | Fractional grams suffice; consistent with SPEC-0004's f64 macro/load values; rounding is presentational. (OQ-C5) |
| 2026-05-31 | **Cross-user access → `404`, never `403`.** | Enumeration-safety, consistent with R-0004/R-0003/R-0002. |
| 2026-05-31 | **Future-date check precedes macro validation (deterministic field precedence).** | A future date reports `"performed_on"` even if a macro is also invalid; documented, mirrors `NewWorkoutSession::new`. |
| 2026-05-31 | **No new crate dependencies.** | Reuses sqlx, chrono, serde, uuid, thiserror, eyre. |
| 2026-05-31 | **SPEC-0001 §7 lockstep snippet policy remains in force.** | Pedantic/fmt deviations under 1.95.0 patched in spec + impl together. |
| 2026-05-31 | **(architect) `into_nutrition_log` reuses the existing free `corrupt(id, what)` helper, not a parallel closure.** | Keeps one corruption-mapping authority shared with the workout row mappers (db.rs); step-5 doesn't duplicate it. |
| 2026-05-31 | **(architect) SAC8 pins both outcomes for the same field — present-but-out-of-range (`400` leaf field) vs present-but-non-numeric (`400` `"body"`).** | Makes the structural-vs-semantic asymmetry testable per branch, as SPEC-0004 SAC8 required. |

## Changelog

- _2026-05-31 — created (Draft). Realizes the accepted R-0005 (per-day grain; calories derived 4/4/9, never stored). Five design questions (OQ-C1..C5) raised for the architect review._
- _2026-05-31 — **Accepted.** `architect` review returned APPROVE WITH NITS; all five OQs approved as proposed (OQ-C1 dedicated response DTO; OQ-C2 `Grams::try_new(value, field)` + single error variant; OQ-C3 explicit-on-insert/auto-mapped-on-update 409 asymmetry; OQ-C4 `calories()` on `Macros`; OQ-C5 `f64` grams). Applied the three nits in lockstep: `into_nutrition_log` reuses the free `corrupt` helper (§3.6); SAC8 pins the out-of-range-vs-non-numeric pairing (§6); `update_nutrition_log` rustdoc names the `error.rs` auto-map source (§3.6). Decision log + §5 updated._
