# SPEC-0003 — User profile CRUD

- **Status:** Accepted
- **Realizes:** R-0003
- **Author:** Claude (main session), with owner
- **Created:** 2026-05-30
- **Depends on:** SPEC-0002 (Implemented) — reuses `AppState`, the `AuthenticatedUser` extractor, `ApiError`, the `db` seam, and the migration/CI/dev-DB machinery
- **Module(s):** `backend/crates/core/profile` (new), `backend/crates/api/profile` (new), `backend/crates/api/{db,error,lib}` (extended), `backend/migrations/` (new file)

## 1. Motivation

Realizes [R-0003](../requirements/0003-user-profile.md): an authenticated user
can create, read, and replace their own fitness profile — the physiological and
goal inputs the ML layer (M4 archetype matching, M5 response inference) consumes
as priors. The profile is a **1:1 resource** keyed by the token's `sub`, exposed
at `/profile/me` with `GET` (read) and `PUT` (create-or-replace upsert).

R-0003 is the spec where `crates/core` gains its **second domain** (the profile
aggregate and its value types) and the api gains its **second persisted table**
and **second HTTP surface** — both built on the R-0002 primitives, not new ones.
R-0003's AC1–AC9 map 1:1 to §6 (SAC1–SAC9).

## 2. Design

### 2.1 Repository layout (additions)

```
fitAI/
├── backend/
│   ├── migrations/
│   │   └── 00002_user_profiles.sql   # NEW — user_profiles table (1:1 with users)
│   └── crates/
│       ├── core/src/
│       │   ├── lib.rs                 # extended — pub mod profile + re-exports
│       │   └── profile.rs             # NEW — Profile, NewProfile, Goal, Sex, newtypes, ProfileError
│       └── api/src/
│           ├── lib.rs                 # extended — pub mod profile + .merge(profile::routes())
│           ├── error.rs               # extended — ApiError::NotFound (404)
│           ├── db.rs                  # extended — ProfileRow, find_profile_by_user, upsert_profile
│           └── profile/               # NEW
│               ├── mod.rs             # routes() -> Router<AppState>
│               └── handlers.rs        # get_me, put_me, ProfileRequest, ProfileResponse
```

No new crate dependencies. The profile path reuses what R-0002 already pulled
in: `chrono` (`NaiveDate` ↔ Postgres `DATE`, enabled by the existing `chrono`
sqlx feature), native sqlx Postgres array support for `goals TEXT[]`, `serde`,
and `f64`/`i32` builtins. *Decision in §7.*

### 2.2 The domain lives in `core`, persistence and HTTP in `api`

Mirrors the R-0002 layering exactly:

- **`core::profile`** is pure (no `sqlx`, no `axum`, no http). It owns the
  closed enums (`Sex`, `Goal`), the range-checked measurement newtypes
  (`HeightCm`, `WeightKg`, `BodyFatPercentage`), the non-empty deduplicated
  `Goals` set, the validated write model (`NewProfile`), the read aggregate
  (`Profile`), and the typed `ProfileError`. **All validation is here** —
  parse-don't-validate, same as `Email`.
- **`api::db`** carries `ProfileRow` (the wire-to-DB shape) and the two queries,
  mapping rows to `core::Profile` at the seam (the same place `UserRow::into_user`
  lives). A corrupt stored value becomes a logged 500, never a fabricated value.
- **`api::profile`** carries the HTTP DTOs (`ProfileRequest`/`ProfileResponse`)
  and the two handlers, which bridge wire JSON ↔ `core` and own no business rules
  beyond field-error mapping.

### 2.3 Validation authority and the request→core bridge

`core::Goal` and `core::Sex` are the **single source of truth** for the
controlled vocabularies (AC6). The request DTO deserializes them **typed**
(`Vec<Goal>`, `Option<Sex>`) so an unknown member is rejected by serde before a
handler runs (→ 400). The remaining rules — DOB-in-future, age ∈ [13, 120],
height/weight/body-fat ranges, goals non-empty + duplicate-free — live in
`NewProfile::new(..., today)`, which the handler invokes and whose `ProfileError`
it maps to `ApiError::Validation { field: e.field() }` (→ 400 naming the field).

`today` is **injected** into `NewProfile::new` (not read from the clock inside
`core`) so age validation and age derivation are deterministic and unit-testable
— the same testability discipline as R-0002's injected `jwt_ttl`.

**Field-label asymmetry (intentional).** Because goals/sex deserialize *typed*,
an out-of-vocabulary member is serde-rejected to `400 field:"body"`, whereas an
*empty* or *duplicated* `goals` (validated in `NewProfile::new`) yields
`400 field:"goals"`. Same endpoint, same input class, two field labels. AC5/AC6
require only **status 400** for the vocabulary case, so this is in-spec; it is
called out here as a known, deliberate trade (single vocabulary authority in
`core`) rather than a surprise for QA. *Architect finding 7.*

### 2.4 The two encodings of `Sex`/`Goal`

Each closed enum is serialized at **two boundaries** with the **same canonical
strings**: serde (`rename_all`) for JSON request/response, and `as_str`/`parse`
for the SQL `TEXT`/`TEXT[]` columns. A `core` unit test pins that the two agree
**exhaustively over every variant** (for each `Goal`/`Sex` value `v`: assert
`parse(as_str(v)) == Ok(v)` and that its serde-serialized string equals
`as_str(v)`), so a newly added member cannot silently drift the two encodings
apart. (JSON and SQL are genuinely different boundaries; a single
`Display`/`FromStr` could serve both but `serde` is idiomatic for JSON DTOs and
explicit `as_str`/`parse` is idiomatic for hand-written SQL binding — keeping
both, guarded by a test, is clearer than forcing one mechanism to serve both.)
*Architect question OQ-B4, §5.*

### 2.5 Authorization scoping (AC7)

The subject is **always** `AuthenticatedUser.user_id` (the token's `sub`). No
user identifier is accepted in the path or body. `find_profile_by_user` and
`upsert_profile` are both keyed by that id, so cross-user read/write is
structurally impossible — there is no code path that takes a caller-supplied
target id. The R-0002 extractor (which also confirms the user still exists) gates
every profile route.

`find_profile_by_user` returns `ApiResult<Option<Profile>>`: a **missing** row
short-circuits as `Ok(None)` (the handler maps it to `ApiError::NotFound` → 404),
while a **corrupt** row that fails `into_profile` returns `Err(Internal)` → 500.
Both the 404 and 500 paths flow through this one function; the `Option` is
absence, the `Err` is corruption. *Architect finding 2.*

### 2.6 Persistence: table, upsert, and the 201/200 distinction

`user_profiles` is 1:1 with `users` via `user_id UUID PRIMARY KEY REFERENCES
users(id) ON DELETE CASCADE`. `PUT` is an atomic `INSERT … ON CONFLICT (user_id)
DO UPDATE …` that bumps `updated_at = NOW()` on the update branch and
`RETURNING …, (xmax = 0) AS inserted`. The `xmax = 0` test is Postgres's
canonical "did this upsert insert (true) or update (false)?" idiom; the handler
maps `true → 201 Created`, `false → 200 OK` (the owner-settled distinction).
*Decision + architect question OQ-B1 in §5/§7.*

**No DB `CHECK` constraints** duplicate the domain ranges or the enum
vocabularies — following R-0002's OQ-A1 resolution (a Postgres regex/CHECK would
drift from `core`). The DB enforces only referential integrity (`PRIMARY KEY`,
`REFERENCES … ON DELETE CASCADE`, `NOT NULL` on required columns). `core` is the
single validation authority. *Decision in §7.*

`goals` is stored as a Postgres `TEXT[]` (not a normalized join table): for a
small closed set on a 1:1 row it is the simplest faithful representation. A
`user_profile_goals` join table would buy queryability the ML layer does not yet
need. *Architect question OQ-B2, §5.*

### 2.7 Numeric representation

`height_cm` is `INTEGER` ↔ `i32` (whole centimetres). `weight_kg` and
`body_fat_percentage` are `DOUBLE PRECISION` ↔ `f64`. Body metrics need 0.1
resolution, not accounting-grade exactness, so `f64` is adequate and avoids
adding a `rust_decimal`/`BigDecimal` dependency and its sqlx feature. The
one-decimal values the client sends round-trip exactly through JSON. *Decision +
architect question OQ-B3 in §5/§7.*

## 3. Code outline

The files below are the agreed implementation shape per `CLAUDE.md` §4.4. Tests
are authored by the `qa` agent during step 3, scoped to R-0003, against §6.
Snippets are representative (final form is reconciled in step-5 lockstep with the
pinned 1.95.0 toolchain, SPEC-0001 §7 policy).

### 3.1 `backend/migrations/00002_user_profiles.sql`

```sql
-- R-0003 / SPEC-0003 — user_profiles table (1:1 with users).
-- Validation (ranges, enum vocabularies, non-empty goals) lives in
-- crates/core, not in DB CHECKs (follows SPEC-0002 OQ-A1). The DB enforces
-- referential integrity only.

CREATE TABLE user_profiles (
    user_id             UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    date_of_birth       DATE NOT NULL,
    height_cm           INTEGER NOT NULL,
    weight_kg           DOUBLE PRECISION NOT NULL,
    sex                 TEXT,
    body_fat_percentage DOUBLE PRECISION,
    goals               TEXT[] NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### 3.2 `backend/crates/core/src/lib.rs` (extended)

```rust
//! fitAI domain types. Pure: no DB, no HTTP, no I/O.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod profile;
pub mod user;

pub use profile::{
    BodyFatPercentage, Goal, Goals, HeightCm, NewProfile, Profile, ProfileError, Sex, WeightKg,
};
pub use user::{Email, EmailParseError, User, UserId};
```

### 3.3 `backend/crates/core/src/profile.rs` — enums + canonical strings

```rust
//! User profile domain: the `Profile` aggregate and its validated value types.
//! Pure — no DB, no HTTP. Same parse-don't-validate style as `user`.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::UserId;

/// Biological sex. Optional on a profile; drives sex-specific ML priors.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sex {
    Male,
    Female,
}

impl Sex {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Sex::Male => "male",
            Sex::Female => "female",
        }
    }

    /// Parse the canonical SQL string (inverse of [`Sex::as_str`]).
    ///
    /// # Errors
    /// [`ProfileError::SexUnknown`] for any value outside the controlled set.
    pub fn parse(raw: &str) -> Result<Self, ProfileError> {
        match raw {
            "male" => Ok(Sex::Male),
            "female" => Ok(Sex::Female),
            _ => Err(ProfileError::SexUnknown),
        }
    }
}

/// A training goal. The controlled set is closed; parsing rejects anything else.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Goal {
    LoseFat,
    BuildMuscle,
    Recomp,
    Maintain,
    GainStrength,
}

impl Goal {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Goal::LoseFat => "lose_fat",
            Goal::BuildMuscle => "build_muscle",
            Goal::Recomp => "recomp",
            Goal::Maintain => "maintain",
            Goal::GainStrength => "gain_strength",
        }
    }

    /// Parse the canonical SQL string (inverse of [`Goal::as_str`]).
    ///
    /// # Errors
    /// [`ProfileError::GoalUnknown`] for any value outside the controlled set.
    pub fn parse(raw: &str) -> Result<Self, ProfileError> {
        match raw {
            "lose_fat" => Ok(Goal::LoseFat),
            "build_muscle" => Ok(Goal::BuildMuscle),
            "recomp" => Ok(Goal::Recomp),
            "maintain" => Ok(Goal::Maintain),
            "gain_strength" => Ok(Goal::GainStrength),
            _ => Err(ProfileError::GoalUnknown),
        }
    }
}
```

### 3.4 `core/src/profile.rs` — measurement newtypes + `Goals`

```rust
/// Height in whole centimetres, range [50, 300].
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct HeightCm(i32);

impl HeightCm {
    pub const MIN: i32 = 50;
    pub const MAX: i32 = 300;

    /// # Errors
    /// [`ProfileError::HeightOutOfRange`] when outside `[50, 300]`.
    pub fn try_new(cm: i32) -> Result<Self, ProfileError> {
        if (Self::MIN..=Self::MAX).contains(&cm) {
            Ok(Self(cm))
        } else {
            Err(ProfileError::HeightOutOfRange)
        }
    }

    #[must_use]
    pub fn get(self) -> i32 {
        self.0
    }
}

/// Weight in kilograms (0.1 resolution), range [20.0, 500.0].
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct WeightKg(f64);

impl WeightKg {
    pub const MIN: f64 = 20.0;
    pub const MAX: f64 = 500.0;

    /// # Errors
    /// [`ProfileError::WeightOutOfRange`] when outside `[20, 500]` or not finite.
    pub fn try_new(kg: f64) -> Result<Self, ProfileError> {
        if kg.is_finite() && (Self::MIN..=Self::MAX).contains(&kg) {
            Ok(Self(kg))
        } else {
            Err(ProfileError::WeightOutOfRange)
        }
    }

    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// Body-fat percentage (0.1 resolution), range [1.0, 75.0].
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct BodyFatPercentage(f64);

impl BodyFatPercentage {
    pub const MIN: f64 = 1.0;
    pub const MAX: f64 = 75.0;

    /// # Errors
    /// [`ProfileError::BodyFatOutOfRange`] when outside `[1, 75]` or not finite.
    pub fn try_new(pct: f64) -> Result<Self, ProfileError> {
        if pct.is_finite() && (Self::MIN..=Self::MAX).contains(&pct) {
            Ok(Self(pct))
        } else {
            Err(ProfileError::BodyFatOutOfRange)
        }
    }

    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// A non-empty, duplicate-free list of goals (input order preserved).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Goals(Vec<Goal>);

impl Goals {
    /// # Errors
    /// [`ProfileError::GoalsEmpty`] if empty; [`ProfileError::GoalsDuplicate`]
    /// if any goal repeats.
    pub fn new(goals: Vec<Goal>) -> Result<Self, ProfileError> {
        if goals.is_empty() {
            return Err(ProfileError::GoalsEmpty);
        }
        let mut seen = std::collections::HashSet::new();
        for g in &goals {
            if !seen.insert(*g) {
                return Err(ProfileError::GoalsDuplicate);
            }
        }
        Ok(Self(goals))
    }

    #[must_use]
    pub fn as_slice(&self) -> &[Goal] {
        &self.0
    }
}
```

### 3.5 `core/src/profile.rs` — age, `NewProfile`, `Profile`, `ProfileError`

```rust
/// Inclusive age bounds (years). Min 13 is a conservative minor-data floor
/// pending the M8 legal review.
pub const MIN_AGE: i32 = 13;
pub const MAX_AGE: i32 = 120;

/// Whole years from `dob` to `today`.
#[must_use]
pub fn age_on(dob: NaiveDate, today: NaiveDate) -> i32 {
    let mut age = today.year() - dob.year();
    if (today.month(), today.day()) < (dob.month(), dob.day()) {
        age -= 1;
    }
    age
}

/// Validated, writable profile fields. No identity (the token's) and no
/// timestamps (the DB's) — only what the client supplies, proven valid.
#[derive(Clone, Debug, PartialEq)]
pub struct NewProfile {
    pub date_of_birth: NaiveDate,
    pub height_cm: HeightCm,
    pub weight_kg: WeightKg,
    pub sex: Option<Sex>,
    pub body_fat_percentage: Option<BodyFatPercentage>,
    pub goals: Goals,
}

impl NewProfile {
    /// Validate raw inputs. `today` is injected for deterministic age checks.
    ///
    /// # Errors
    /// The first [`ProfileError`] encountered (field-named via
    /// [`ProfileError::field`]).
    pub fn new(
        date_of_birth: NaiveDate,
        height_cm: i32,
        weight_kg: f64,
        goals: Vec<Goal>,
        sex: Option<Sex>,
        body_fat_percentage: Option<f64>,
        today: NaiveDate,
    ) -> Result<Self, ProfileError> {
        if date_of_birth > today {
            return Err(ProfileError::DateOfBirthInFuture);
        }
        let age = age_on(date_of_birth, today);
        if !(MIN_AGE..=MAX_AGE).contains(&age) {
            return Err(ProfileError::AgeOutOfRange);
        }
        Ok(Self {
            date_of_birth,
            height_cm: HeightCm::try_new(height_cm)?,
            weight_kg: WeightKg::try_new(weight_kg)?,
            sex,
            body_fat_percentage: body_fat_percentage.map(BodyFatPercentage::try_new).transpose()?,
            goals: Goals::new(goals)?,
        })
    }
}

/// The full read aggregate, reconstructed from a persisted row.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Profile {
    pub user_id: UserId,
    pub date_of_birth: NaiveDate,
    pub height_cm: HeightCm,
    pub weight_kg: WeightKg,
    pub sex: Option<Sex>,
    pub body_fat_percentage: Option<BodyFatPercentage>,
    pub goals: Goals,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Profile {
    #[must_use]
    pub fn age_on(&self, today: NaiveDate) -> i32 {
        age_on(self.date_of_birth, today)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProfileError {
    #[error("date_of_birth is in the future")]
    DateOfBirthInFuture,
    #[error("age is outside the allowed range")]
    AgeOutOfRange,
    #[error("height_cm is outside the allowed range")]
    HeightOutOfRange,
    #[error("weight_kg is outside the allowed range")]
    WeightOutOfRange,
    #[error("body_fat_percentage is outside the allowed range")]
    BodyFatOutOfRange,
    #[error("goals must not be empty")]
    GoalsEmpty,
    #[error("goals must not contain duplicates")]
    GoalsDuplicate,
    #[error("unknown goal")]
    GoalUnknown,
    #[error("unknown sex")]
    SexUnknown,
}

impl ProfileError {
    /// The request field this error concerns — drives `ApiError::Validation`.
    #[must_use]
    pub fn field(&self) -> &'static str {
        match self {
            ProfileError::DateOfBirthInFuture | ProfileError::AgeOutOfRange => "date_of_birth",
            ProfileError::HeightOutOfRange => "height_cm",
            ProfileError::WeightOutOfRange => "weight_kg",
            ProfileError::BodyFatOutOfRange => "body_fat_percentage",
            ProfileError::GoalsEmpty | ProfileError::GoalsDuplicate | ProfileError::GoalUnknown => {
                "goals"
            }
            ProfileError::SexUnknown => "sex",
        }
    }
}
```

### 3.6 `backend/crates/api/src/error.rs` — `NotFound` variant (extended)

```rust
#[derive(Debug, Error)]
pub enum ApiError {
    // … existing variants unchanged …
    #[error("not found")]
    NotFound,
}

// inside the existing `match &self { … }` in IntoResponse::into_response
// (borrows self, no data to move — same shape as the other simple arms):
ApiError::NotFound => (StatusCode::NOT_FOUND, json!({"error": "not_found"})),
```

### 3.7 `backend/crates/api/src/db.rs` — profile row + queries (extended)

```rust
use chrono::NaiveDate;
use fitai_core::{Goal, Goals, HeightCm, NewProfile, Profile, Sex, WeightKg, BodyFatPercentage};

#[derive(Debug, FromRow)]
pub struct ProfileRow {
    pub user_id: Uuid,
    pub date_of_birth: NaiveDate,
    pub height_cm: i32,
    pub weight_kg: f64,
    pub sex: Option<String>,
    pub body_fat_percentage: Option<f64>,
    pub goals: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProfileRow {
    /// Reconstruct the domain `Profile`. A stored value that fails domain
    /// validation is data corruption (we only ever persist validated values),
    /// surfaced as a logged 500 — never silently coerced (cf. `into_user`).
    pub fn into_profile(self) -> ApiResult<Profile> {
        let user_id = self.user_id;
        let corrupt = move |what: &'static str| {
            tracing::error!(%user_id, what, "stored profile value failed domain validation");
            ApiError::Internal(eyre::eyre!("stored profile failed domain validation"))
        };

        let height_cm = HeightCm::try_new(self.height_cm).map_err(|_| corrupt("height_cm"))?;
        let weight_kg = WeightKg::try_new(self.weight_kg).map_err(|_| corrupt("weight_kg"))?;
        let body_fat_percentage = self
            .body_fat_percentage
            .map(BodyFatPercentage::try_new)
            .transpose()
            .map_err(|_| corrupt("body_fat_percentage"))?;
        let sex = self
            .sex
            .as_deref()
            .map(Sex::parse)
            .transpose()
            .map_err(|_| corrupt("sex"))?;
        let goals = self
            .goals
            .iter()
            .map(|g| Goal::parse(g))
            .collect::<Result<Vec<_>, _>>()
            .and_then(Goals::new)
            .map_err(|_| corrupt("goals"))?;

        Ok(Profile {
            user_id: UserId(self.user_id),
            date_of_birth: self.date_of_birth,
            height_cm,
            weight_kg,
            sex,
            body_fat_percentage,
            goals,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

pub async fn find_profile_by_user(pool: &PgPool, user_id: UserId) -> ApiResult<Option<Profile>> {
    let row = sqlx::query_as::<_, ProfileRow>(
        "SELECT user_id, date_of_birth, height_cm, weight_kg, sex, \
         body_fat_percentage, goals, created_at, updated_at \
         FROM user_profiles WHERE user_id = $1",
    )
    .bind(user_id.0)
    .fetch_optional(pool)
    .await?;
    row.map(ProfileRow::into_profile).transpose()
}

/// Upsert the caller's profile. Returns the stored aggregate and whether this
/// call inserted (→ 201) versus replaced (→ 200).
///
/// The single `RETURNING` row carries all nine profile columns plus a
/// computed `inserted` flag. Rather than a `#[sqlx(flatten)]` wrapper struct
/// (a flattened `ProfileRow` *plus* a sibling scalar is the one fragile sqlx
/// construct here — architect blocking finding 1), we read the one `PgRow`
/// directly: `bool` via `try_get("inserted")`, then `ProfileRow::from_row`,
/// which maps by name and simply ignores the extra `inserted` column.
pub async fn upsert_profile(
    pool: &PgPool,
    user_id: UserId,
    p: &NewProfile,
) -> ApiResult<(Profile, bool)> {
    use sqlx::Row as _; // for try_get
    use sqlx::FromRow as _; // for ProfileRow::from_row

    let sex = p.sex.map(Sex::as_str);
    let body_fat = p.body_fat_percentage.map(BodyFatPercentage::get);
    let goals: Vec<String> = p.goals.as_slice().iter().map(|g| g.as_str().to_owned()).collect();

    // `xmax = 0` is Postgres's canonical "did this upsert INSERT (true) or
    // UPDATE (false)?" signal for a plain INSERT … ON CONFLICT DO UPDATE.
    let row = sqlx::query(
        "INSERT INTO user_profiles \
           (user_id, date_of_birth, height_cm, weight_kg, sex, body_fat_percentage, goals) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (user_id) DO UPDATE SET \
           date_of_birth = EXCLUDED.date_of_birth, \
           height_cm = EXCLUDED.height_cm, \
           weight_kg = EXCLUDED.weight_kg, \
           sex = EXCLUDED.sex, \
           body_fat_percentage = EXCLUDED.body_fat_percentage, \
           goals = EXCLUDED.goals, \
           updated_at = NOW() \
         RETURNING user_id, date_of_birth, height_cm, weight_kg, sex, \
           body_fat_percentage, goals, created_at, updated_at, (xmax = 0) AS inserted",
    )
    .bind(user_id.0)
    .bind(p.date_of_birth)
    .bind(p.height_cm.get())
    .bind(p.weight_kg.get())
    .bind(sex)
    .bind(body_fat)
    .bind(&goals)
    .fetch_one(pool)
    .await?;

    let inserted: bool = row.try_get("inserted")?;
    let profile = ProfileRow::from_row(&row)?.into_profile()?;
    Ok((profile, inserted))
}
```

> `ProfileRow::from_row(&row)?` and `row.try_get(...)?` both surface a
> `sqlx::Error`, which `?` converts to `ApiError::Database` via the existing
> `#[from]` — no new error plumbing. The `RETURNING` column list names exactly
> the nine `ProfileRow` fields (plus `inserted`), so the by-name `FromRow`
> mapping resolves. *Architect blocking finding 1 resolved; OQ-B1 confirmed.*

### 3.8 `backend/crates/api/src/profile/mod.rs`

```rust
//! Profile surface: GET/PUT /profile/me.

mod handlers;

use axum::{routing::get, Router};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/profile/me", get(handlers::get_me).put(handlers::put_me))
}
```

### 3.9 `backend/crates/api/src/profile/handlers.rs`

```rust
//! HTTP handlers for the profile endpoints.

use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use fitai_core::{BodyFatPercentage, Goal, NewProfile, Profile, Sex, UserId};

use crate::{
    auth::AuthenticatedUser,
    db,
    error::{ApiError, ApiResult},
    AppState,
};

#[derive(Debug, Deserialize)]
pub(crate) struct ProfileRequest {
    date_of_birth: NaiveDate,
    height_cm: i32,
    weight_kg: f64,
    goals: Vec<Goal>,
    #[serde(default)]
    sex: Option<Sex>,
    #[serde(default)]
    body_fat_percentage: Option<f64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ProfileResponse {
    user_id: UserId,
    date_of_birth: NaiveDate,
    age: i32,
    height_cm: i32,
    weight_kg: f64,
    sex: Option<Sex>,
    body_fat_percentage: Option<f64>,
    goals: Vec<Goal>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ProfileResponse {
    fn from_profile(p: &Profile, today: NaiveDate) -> Self {
        Self {
            age: p.age_on(today),
            user_id: p.user_id,
            date_of_birth: p.date_of_birth,
            height_cm: p.height_cm.get(),
            weight_kg: p.weight_kg.get(),
            sex: p.sex,
            body_fat_percentage: p.body_fat_percentage.map(BodyFatPercentage::get),
            goals: p.goals.as_slice().to_vec(),
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

/// Map any serde/body rejection (missing field, bad type, malformed JSON) to a
/// 400 — without this, axum's `Json` extractor rejects before the handler runs.
fn parse_body(req: Result<Json<ProfileRequest>, JsonRejection>) -> ApiResult<ProfileRequest> {
    req.map(|Json(r)| r).map_err(|_| ApiError::Validation { field: "body" })
}

pub(crate) async fn get_me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<ProfileResponse>> {
    let profile = db::find_profile_by_user(&state.pool, user.user_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let today = Utc::now().date_naive();
    Ok(Json(ProfileResponse::from_profile(&profile, today)))
}

pub(crate) async fn put_me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    req: Result<Json<ProfileRequest>, JsonRejection>,
) -> ApiResult<(StatusCode, Json<ProfileResponse>)> {
    let req = parse_body(req)?;
    let today = Utc::now().date_naive();
    let new = NewProfile::new(
        req.date_of_birth,
        req.height_cm,
        req.weight_kg,
        req.goals,
        req.sex,
        req.body_fat_percentage,
        today,
    )
    .map_err(|e| ApiError::Validation { field: e.field() })?;

    let (profile, inserted) = db::upsert_profile(&state.pool, user.user_id, &new).await?;
    let status = if inserted { StatusCode::CREATED } else { StatusCode::OK };
    Ok((status, Json(ProfileResponse::from_profile(&profile, today))))
}
```

### 3.10 `backend/crates/api/src/lib.rs` (extended)

```rust
pub mod auth;
pub mod db;
pub mod error;
mod health;
pub mod profile; // NEW

pub fn app(state: AppState) -> Router {
    Router::new()
        .merge(health::router())
        .merge(auth::routes())
        .merge(profile::routes()) // NEW
        .with_state(state)
}
```

## 4. Non-goals

- Tape/circumference measurements and any time-series body-measurement history (M2 logging, R-0005+).
- Progress photos / photo-derived body-comp proxy (M6).
- Archetype matching / initial-program assignment (M4) — R-0003 only stores the inputs matching will consume.
- Imperial units / unit-system conversion (separate R if ever needed; canonical metric only here).
- Profile/account deletion and GDPR export (M8 / R-0024). The `ON DELETE CASCADE` FK only keeps integrity when a future flow removes a user.
- `PATCH`/partial update — `PUT` is full-replace upsert.
- Multiple profiles per user — strictly 1:1.
- Querying/aggregating users by goal or metric (analytics) — no indexes or join table for it yet (see OQ-B2).
- A `sqlx prepare` offline cache step — unchanged from SPEC-0002 §4 (CI has a live DB).

## 5. Open questions

All four architect questions were settled in the 2026-05-30 design review
(APPROVE WITH NITS; resolutions in §7):

- **OQ-B1 — *resolved, kept.*** 201/200 via `RETURNING (xmax = 0) AS inserted` — the canonical atomic insert-vs-update signal; preferred over a `created_at = updated_at` comparison (which couples status to timestamp semantics). The *read-back* mechanism changed: a `#[sqlx(flatten)]` wrapper was the one fragile sqlx construct (blocking finding 1), so §3.7 now reads the single `PgRow` directly (`try_get("inserted")` + `ProfileRow::from_row`).
- **OQ-B2 — *resolved.*** `goals` stays a `TEXT[]`. A closed 5-value set on a strictly 1:1 row has no relational pull; a join table buys by-goal queryability that §4 explicitly defers. Revisit at the analytics R.
- **OQ-B3 — *resolved.*** `f64`/`DOUBLE PRECISION` for weight & body-fat — physiological estimates at 0.1 resolution, not monetary; the `is_finite()` guard in the newtypes covers the one real f64 footgun. `rust_decimal` would be premature.
- **OQ-B4 — *resolved.*** Dual serde + `as_str`/`parse` kept, with the guarding unit test made **exhaustive over all variants** (§2.4, SAC8) so an added member cannot drift the encodings.

For the **owner**: none — R-0003 OQ1–OQ4 were settled at requirement discussion (2026-05-30) and the five derived engineering calls are recorded in the requirement's §6.

## 6. Acceptance criteria

Each SAC maps back to an R-0003 AC; each becomes one or more `qa` agent tests.

- [ ] **SAC1 → AC1.** `00002_user_profiles.sql` exists; migrating a fresh DB succeeds. `information_schema.columns WHERE table_name = 'user_profiles'` shows `user_id, date_of_birth, height_cm, weight_kg, sex, body_fat_percentage, goals, created_at, updated_at` with the expected types/nullability. A `DELETE FROM users` cascades: the matching `user_profiles` row is gone.
- [ ] **SAC2 → AC2.** `GET /profile/me` with a valid `Bearer` token: 404 + `{"error":"not_found"}` when no profile exists; 200 + the profile JSON when it does; 401 + `{"error":"unauthorized"}` for missing/invalid token (delegated to the R-0002 extractor).
- [ ] **SAC3 → AC3.** `PUT /profile/me` with a valid token and a well-formed body: 201 on first write; 200 on a second (replacing) write with a strictly greater `updated_at`; after N calls exactly one `user_profiles` row exists for that user; 401 for missing/invalid token.
- [ ] **SAC4 → AC4.** The `GET`/`PUT` body carries `user_id`, `date_of_birth` (RFC 3339 date), a derived integer `age`, `height_cm`, `weight_kg`, `sex` (`"male"`/`"female"`/`null`), `body_fat_percentage` (or `null`), `goals` (array of canonical strings), `created_at`, `updated_at` (RFC 3339). A profile written with a known DOB reports the arithmetically correct `age`; omitted `sex`/`body_fat_percentage` serialize as `null`.
- [ ] **SAC5 → AC5.** Each of these returns 400 + `{"error":"validation","field":<f>}` and writes nothing: future `date_of_birth` (field `date_of_birth`); DOB implying age `< 13` or `> 120` (`date_of_birth`); `height_cm` ∉ [50,300] (`height_cm`); `weight_kg` ∉ [20,500] (`weight_kg`); `body_fat_percentage` present and ∉ [1,75] (`body_fat_percentage`); `goals` empty or duplicated (`goals`); a goal outside the controlled set (400, serde-rejected); `sex` not `male`/`female` (400, serde-rejected); a missing required field (400).
- [ ] **SAC6 → AC6.** The controlled goal set is exactly `lose_fat, build_muscle, recomp, maintain, gain_strength`, defined once as `core::Goal` and shared by request parsing, SQL storage, and the JSON response. A multi-goal body (e.g. `["build_muscle","lose_fat"]`) round-trips.
- [ ] **SAC7 → AC7.** Two distinct authenticated users each `PUT` and `GET`; user A's write never mutates B's row and A's read never returns B's data. No cross-user identifier is accepted in path or body — the subject is always the token's `sub`.
- [ ] **SAC8 → AC8.** `core::profile` has unit tests for every validation rule in SAC5/SAC6 (age boundaries 12/13/120/121, each range edge, goals empty/duplicate, unknown goal/sex, and `as_str`↔serde agreement) and `age_on` across a birthday boundary. `crates/core` compiles with no `sqlx`/`axum`/http dependency (purity preserved).
- [ ] **SAC9 → AC9.** At least ten `#[sqlx::test(migrations = "../../migrations")]` integration tests pass, covering: GET-before-write (404), PUT first (201), PUT replace (200 + `updated_at` bump + single row), GET-after-write (200 + correct derived `age` + null optionals), the SAC5 400 branches, unauthorized GET/PUT (401), and the SAC7 cross-user isolation case. Exact list authored by qa in step 3.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-05-30 | **Profile domain (`Profile`, `NewProfile`, `Goal`, `Sex`, measurement newtypes, `Goals`, `ProfileError`) lives in `crates/core`; persistence in `api::db`, HTTP in `api::profile`.** | Mirrors the R-0002 Email/User layering; keeps validation pure and reusable, the persistence seam auditable, and handlers thin. |
| 2026-05-30 | **All field validation in `core` (`NewProfile::new` + newtype `try_new`); no DB `CHECK` constraints.** Follows SPEC-0002 OQ-A1. | A Postgres CHECK/regex would duplicate and drift from `core`. The DB enforces only referential integrity (PK, FK ON DELETE CASCADE, NOT NULL). |
| 2026-05-30 | **`today` injected into `NewProfile::new` and `age_on`/`Profile::age_on`.** | Deterministic, unit-testable age validation/derivation — same discipline as R-0002's injected `jwt_ttl`. |
| 2026-05-30 | **Typed serde deserialization of `Vec<Goal>`/`Option<Sex>` in the request.** | Makes `core::Goal`/`core::Sex` the single vocabulary authority (AC6); unknown members are serde-rejected to 400 before any handler logic. |
| 2026-05-30 | **`Sex`/`Goal` carry two encodings — serde (JSON) and `as_str`/`parse` (SQL) — sharing canonical strings, pinned equal by a unit test.** | JSON DTOs and hand-written SQL binding are different idioms; one mechanism forced to serve both is less clear than two guarded by a test. (OQ-B4) |
| 2026-05-30 | **`user_profiles` is 1:1 via `user_id` PK + FK `REFERENCES users(id) ON DELETE CASCADE`.** | One profile per account; create/replace collapse to upsert (owner OQ4). Cascade keeps integrity for the future deletion flow (R-0024). |
| 2026-05-30 | **`PUT` is `INSERT … ON CONFLICT (user_id) DO UPDATE …`; 201 vs 200 from `RETURNING (xmax = 0) AS inserted`.** | Atomic upsert (no read-modify race); `xmax = 0` is Postgres's canonical insert-vs-update signal for the owner-settled 201/200 split. (OQ-B1) |
| 2026-05-30 | **`goals` stored as Postgres `TEXT[]` (not a join table).** | Simplest faithful shape for a small closed set on a 1:1 row; no by-goal query requirement yet. (OQ-B2) |
| 2026-05-30 | **`height_cm` `INTEGER`/`i32`; `weight_kg` & `body_fat_percentage` `DOUBLE PRECISION`/`f64`.** | 0.1 resolution suffices for body metrics; `f64` avoids a `rust_decimal`/`BigDecimal` dependency + sqlx feature; one-decimal values round-trip exactly through JSON. (OQ-B3) |
| 2026-05-30 | **Store DOB, derive `age` at read time.** | Owner-settled (requirement §6). Age never goes stale. |
| 2026-05-30 | **Minimum age 13.** | Owner-settled (requirement §6). Conservative minor-data floor; revisit at M8 legal review. |
| 2026-05-30 | **New `ApiError::NotFound` (404 `{"error":"not_found"}`) added to the shared error enum.** | `GET /profile/me` needs a distinct 404 for "authenticated but no profile yet"; reusable by later resource specs. |
| 2026-05-30 | **No new crate dependencies.** | Reuses sqlx (chrono `DATE`/`NaiveDate`, native Postgres arrays), chrono, serde, `f64`/`i32`. §2 "no premature anything". |
| 2026-05-30 | **SPEC-0001 §7 lockstep snippet policy remains in force.** | Any clippy-pedantic/fmt deviations under the pinned 1.95.0 toolchain get patched in spec + impl together, as in R-0001/R-0002. |
| 2026-05-30 | **§3.7 `upsert_profile` reads a single `PgRow` directly** (`try_get("inserted")` + `ProfileRow::from_row(&row)?.into_profile()?`) **instead of a `#[sqlx(flatten)]` wrapper struct.** | Architect blocking finding 1: the flatten wrapper was the one fragile sqlx construct on the path; a direct read keeps the insert-vs-update flag and the row mapping explicit and side-by-side. |

## Changelog

- _2026-05-30 — created (Draft). OQ1–OQ4 + five derived calls inherited from the accepted R-0003. Four architect questions (OQ-B1..B4) raised for the design review._
- _2026-05-30 — `architect` design review: **APPROVE WITH NITS** (one blocking finding). Applied in lockstep: blocking finding 1 (§3.7 direct `PgRow` read replaces `#[sqlx(flatten)]` `UpsertRow`); minor 3 (dropped dead `#[allow(clippy::too_many_arguments)]` on `NewProfile::new`, added its `# Errors` section); nits 8/9 (all public fallible fns carry proper `/// # Errors` sections; `NotFound` arm placed inside the existing `match &self`). Documented the request field-label asymmetry (§2.3) and the 404-vs-500 boundary (§2.5); made the §2.4 dual-encoding test exhaustive over all variants. OQ-B1..OQ-B4 resolved (§5). Status → **Accepted**; SPEC-0003 may proceed to step 3 (qa red suite)._
