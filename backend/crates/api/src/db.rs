//! Postgres-side types and queries. Maps row shapes to `fitai_core` types
//! at the seam so callers never see `password_hash`.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{prelude::FromRow, PgPool, Postgres, Row as _, Transaction};
use uuid::Uuid;

use fitai_core::{
    Angle, BodyFatPercentage, Email, ExerciseName, Goal, Goals, Grams, HeightCm, ImageContentType,
    LoadKg, Macros, MuscleGroup, NewExercise, NewNutritionLog, NewPhoto, NewProfile,
    NewWorkoutSession, NutritionLog, PhotoSession, Profile, Reps, Rpe, SessionPhoto, Sex, User,
    UserId, WeightKg, WorkoutExercise, WorkoutSession, WorkoutSet,
};

use crate::error::{ApiError, ApiResult};

#[derive(Debug, FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    // only crosses the seam via find_row_by_email → login (verify needs it);
    // into_user strips it everywhere else.
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

impl UserRow {
    /// Convert a persisted row into the domain `User`, stripping
    /// `password_hash`. Fallible: a stored `email` that fails
    /// `core::Email::parse` is data corruption (we only ever write
    /// parsed-and-normalized emails), so surface it loudly as a 500 rather
    /// than fabricating a placeholder identity.
    ///
    /// # Errors
    /// Returns [`ApiError::Internal`] when the stored email fails domain
    /// validation (data corruption).
    pub fn into_user(self) -> ApiResult<User> {
        let email = Email::parse(&self.email).map_err(|_| {
            tracing::error!(user_id = %self.id, "stored email failed core::Email::parse — data corruption");
            ApiError::Internal(eyre::eyre!("stored email failed domain validation"))
        })?;
        Ok(User {
            id: UserId(self.id),
            email,
            created_at: self.created_at,
        })
    }
}

/// Look up a user by id, mapping the row to the domain `User`.
///
/// # Errors
/// Returns [`ApiError::Database`] on a query failure, or [`ApiError::Internal`]
/// if the stored row fails domain validation.
pub async fn find_user_by_id(pool: &PgPool, id: UserId) -> ApiResult<Option<User>> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, password_hash, created_at FROM users WHERE id = $1",
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?;
    row.map(UserRow::into_user).transpose()
}

/// Fetch the raw row (including `password_hash`) for a login attempt.
///
/// # Errors
/// Returns [`ApiError::Database`] on a query failure.
pub async fn find_row_by_email(pool: &PgPool, email: &str) -> ApiResult<Option<UserRow>> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, password_hash, created_at FROM users WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Insert a new user, returning its id. Maps a unique-violation to
/// [`ApiError::AlreadyExists`].
///
/// # Errors
/// Returns [`ApiError::AlreadyExists`] when the email is already taken, or
/// [`ApiError::Database`] on any other query failure.
pub async fn insert_user(pool: &PgPool, email: &str, password_hash: &str) -> ApiResult<UserId> {
    let id = Uuid::new_v4();
    let result = sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(email)
        .bind(password_hash)
        .execute(pool)
        .await;

    match result {
        Ok(_) => Ok(UserId(id)),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(ApiError::AlreadyExists)
        }
        Err(e) => Err(ApiError::Database(e)),
    }
}

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
    ///
    /// # Errors
    /// Returns [`ApiError::Internal`] when a stored value fails domain
    /// validation (data corruption).
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

/// Fetch the caller's profile, mapping the row to the domain `Profile`.
///
/// # Errors
/// Returns [`ApiError::Database`] on a query failure, or [`ApiError::Internal`]
/// if the stored row fails domain validation.
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
/// computed `inserted` flag. We read the one `PgRow` directly: `bool` via
/// `try_get("inserted")`, then `ProfileRow::from_row`, which maps by name and
/// ignores the extra `inserted` column.
///
/// # Errors
/// Returns [`ApiError::Database`] on a query failure, or [`ApiError::Internal`]
/// if the stored row fails domain validation on read-back.
pub async fn upsert_profile(
    pool: &PgPool,
    user_id: UserId,
    p: &NewProfile,
) -> ApiResult<(Profile, bool)> {
    let sex = p.sex.map(Sex::as_str);
    let body_fat = p.body_fat_percentage.map(BodyFatPercentage::get);
    let goals: Vec<String> = p
        .goals
        .as_slice()
        .iter()
        .map(|g| g.as_str().to_owned())
        .collect();

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

#[derive(Debug, FromRow)]
pub struct SessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub performed_on: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct ExerciseRow {
    pub id: Uuid,
    pub session_id: Uuid,
    pub position: i32,
    pub name: String,
    pub muscle_group: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct SetRow {
    pub id: Uuid,
    pub exercise_id: Uuid,
    pub position: i32,
    pub reps: i32,
    pub weight_kg: Option<f64>,
    pub rpe: Option<f64>,
}

/// A stored value that fails domain validation is data corruption (we only ever
/// persist validated values), surfaced as a logged 500 — never silently coerced
/// (the `into_profile`/`into_user` discipline). Shared by the workout and
/// nutrition row mappers.
fn corrupt(id: Uuid, what: &'static str) -> ApiError {
    tracing::error!(%id, what, "stored value failed domain validation");
    ApiError::Internal(eyre::eyre!("stored value failed domain validation"))
}

fn set_from_row(r: &SetRow) -> ApiResult<WorkoutSet> {
    let id = r.id;
    Ok(WorkoutSet {
        id,
        position: r.position,
        reps: Reps::try_new(r.reps).map_err(|_| corrupt(id, "reps"))?,
        weight_kg: r
            .weight_kg
            .map(LoadKg::try_new)
            .transpose()
            .map_err(|_| corrupt(id, "weight_kg"))?,
        rpe: r
            .rpe
            .map(Rpe::try_new)
            .transpose()
            .map_err(|_| corrupt(id, "rpe"))?,
    })
}

fn exercise_from_row(r: &ExerciseRow, sets: Vec<WorkoutSet>) -> ApiResult<WorkoutExercise> {
    let id = r.id;
    Ok(WorkoutExercise {
        id,
        position: r.position,
        name: ExerciseName::try_new(&r.name).map_err(|_| corrupt(id, "name"))?,
        muscle_group: r
            .muscle_group
            .as_deref()
            .map(MuscleGroup::parse)
            .transpose()
            .map_err(|_| corrupt(id, "muscle_group"))?,
        sets,
    })
}

/// Insert the validated exercises (and their sets) for `session_id` within an
/// open transaction, assigning server-side ids and 0-based `position`s from the
/// array index. Returns the stored read aggregates.
async fn insert_exercises(
    tx: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
    exercises: &[NewExercise],
) -> ApiResult<Vec<WorkoutExercise>> {
    let mut stored = Vec::with_capacity(exercises.len());
    for (ei, ex) in exercises.iter().enumerate() {
        let position = i32::try_from(ei)?;
        let exercise_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workout_exercises (id, session_id, position, name, muscle_group) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(exercise_id)
        .bind(session_id)
        .bind(position)
        .bind(ex.name.as_str())
        .bind(ex.muscle_group.map(MuscleGroup::as_str))
        .execute(&mut **tx)
        .await?;

        let mut sets = Vec::with_capacity(ex.sets.len());
        for (si, st) in ex.sets.iter().enumerate() {
            let set_position = i32::try_from(si)?;
            let set_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO workout_sets (id, exercise_id, position, reps, weight_kg, rpe) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(set_id)
            .bind(exercise_id)
            .bind(set_position)
            .bind(st.reps.get())
            .bind(st.weight_kg.map(LoadKg::get))
            .bind(st.rpe.map(Rpe::get))
            .execute(&mut **tx)
            .await?;
            sets.push(WorkoutSet {
                id: set_id,
                position: set_position,
                reps: st.reps,
                weight_kg: st.weight_kg,
                rpe: st.rpe,
            });
        }
        stored.push(WorkoutExercise {
            id: exercise_id,
            position,
            name: ex.name.clone(),
            muscle_group: ex.muscle_group,
            sets,
        });
    }
    Ok(stored)
}

/// Load every exercise (with its sets) for the given session ids, grouped by
/// session id. Two batched queries — no N+1, no join row-explosion — with an
/// empty-id short-circuit (SPEC-0004 §2.5 / OQ-C2). The `ORDER BY parent,
/// position` clauses make the grouped push order match `position`.
async fn load_exercises_by_session(
    pool: &PgPool,
    session_ids: &[Uuid],
) -> ApiResult<HashMap<Uuid, Vec<WorkoutExercise>>> {
    if session_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let exercise_rows: Vec<ExerciseRow> = sqlx::query_as(
        "SELECT id, session_id, position, name, muscle_group FROM workout_exercises \
         WHERE session_id = ANY($1) ORDER BY session_id, position",
    )
    .bind(session_ids)
    .fetch_all(pool)
    .await?;

    let exercise_ids: Vec<Uuid> = exercise_rows.iter().map(|r| r.id).collect();
    let set_rows: Vec<SetRow> = if exercise_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as(
            "SELECT id, exercise_id, position, reps, weight_kg, rpe FROM workout_sets \
             WHERE exercise_id = ANY($1) ORDER BY exercise_id, position",
        )
        .bind(&exercise_ids)
        .fetch_all(pool)
        .await?
    };

    let mut sets_by_exercise: HashMap<Uuid, Vec<WorkoutSet>> = HashMap::new();
    for sr in set_rows {
        let exercise_id = sr.exercise_id;
        sets_by_exercise
            .entry(exercise_id)
            .or_default()
            .push(set_from_row(&sr)?);
    }

    let mut exercises_by_session: HashMap<Uuid, Vec<WorkoutExercise>> = HashMap::new();
    for er in exercise_rows {
        let session_id = er.session_id;
        let sets = sets_by_exercise.remove(&er.id).unwrap_or_default();
        exercises_by_session
            .entry(session_id)
            .or_default()
            .push(exercise_from_row(&er, sets)?);
    }
    Ok(exercises_by_session)
}

/// Insert a full session atomically; returns the stored aggregate.
///
/// # Errors
/// Returns [`ApiError::Database`] on any query failure (the transaction rolls
/// back), or [`ApiError::IntConversion`] if a position index exceeds `i32`.
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
    .bind(session_id)
    .bind(user_id.0)
    .bind(new.performed_on)
    .fetch_one(&mut *tx)
    .await?;

    let exercises = insert_exercises(&mut tx, session_id, &new.exercises).await?;
    tx.commit().await?;

    Ok(WorkoutSession {
        id: row.id,
        user_id,
        performed_on: row.performed_on,
        exercises,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

/// All of the caller's sessions, newest `performed_on` first, fully nested.
///
/// # Errors
/// Returns [`ApiError::Database`] on a query failure, or [`ApiError::Internal`]
/// if a stored row fails domain validation on read-back.
pub async fn find_sessions_by_user(
    pool: &PgPool,
    user_id: UserId,
) -> ApiResult<Vec<WorkoutSession>> {
    let session_rows: Vec<SessionRow> = sqlx::query_as(
        "SELECT id, user_id, performed_on, created_at, updated_at FROM workout_sessions \
         WHERE user_id = $1 ORDER BY performed_on DESC, created_at DESC",
    )
    .bind(user_id.0)
    .fetch_all(pool)
    .await?;

    let session_ids: Vec<Uuid> = session_rows.iter().map(|r| r.id).collect();
    let mut exercises_by_session = load_exercises_by_session(pool, &session_ids).await?;

    Ok(session_rows
        .into_iter()
        .map(|row| WorkoutSession {
            id: row.id,
            user_id,
            performed_on: row.performed_on,
            exercises: exercises_by_session.remove(&row.id).unwrap_or_default(),
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect())
}

/// One session if it exists and is owned by the caller, else `None` (→ 404).
///
/// # Errors
/// Returns [`ApiError::Database`] on a query failure, or [`ApiError::Internal`]
/// if a stored row fails domain validation on read-back.
pub async fn find_session_by_id(
    pool: &PgPool,
    user_id: UserId,
    id: Uuid,
) -> ApiResult<Option<WorkoutSession>> {
    let row: Option<SessionRow> = sqlx::query_as(
        "SELECT id, user_id, performed_on, created_at, updated_at FROM workout_sessions \
         WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id.0)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let mut exercises_by_session = load_exercises_by_session(pool, &[row.id]).await?;
    Ok(Some(WorkoutSession {
        id: row.id,
        user_id,
        performed_on: row.performed_on,
        exercises: exercises_by_session.remove(&row.id).unwrap_or_default(),
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

/// Full-replace edit within a transaction; `None` when the session is missing
/// or owned by another user (→ 404). The session row is updated in place
/// (`created_at` preserved, `updated_at` bumped); the children are deleted
/// (sets cascade) and re-inserted with new ids.
///
/// # Errors
/// Returns [`ApiError::Database`] on any query failure (the transaction rolls
/// back), or [`ApiError::IntConversion`] if a position index exceeds `i32`.
pub async fn replace_session(
    pool: &PgPool,
    user_id: UserId,
    id: Uuid,
    new: &NewWorkoutSession,
) -> ApiResult<Option<WorkoutSession>> {
    let mut tx = pool.begin().await?;
    let row: Option<SessionRow> = sqlx::query_as(
        "UPDATE workout_sessions SET performed_on = $1, updated_at = NOW() \
         WHERE id = $2 AND user_id = $3 \
         RETURNING id, user_id, performed_on, created_at, updated_at",
    )
    .bind(new.performed_on)
    .bind(id)
    .bind(user_id.0)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        // Missing or foreign: nothing written, transaction dropped (rollback).
        return Ok(None);
    };

    sqlx::query("DELETE FROM workout_exercises WHERE session_id = $1")
        .bind(row.id)
        .execute(&mut *tx)
        .await?;

    let exercises = insert_exercises(&mut tx, row.id, &new.exercises).await?;
    tx.commit().await?;

    Ok(Some(WorkoutSession {
        id: row.id,
        user_id,
        performed_on: row.performed_on,
        exercises,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

/// Delete the caller's session (children cascade); `false` when the session is
/// missing or owned by another user (→ 404).
///
/// # Errors
/// Returns [`ApiError::Database`] on a query failure.
pub async fn delete_session(pool: &PgPool, user_id: UserId, id: Uuid) -> ApiResult<bool> {
    let result = sqlx::query("DELETE FROM workout_sessions WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id.0)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

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
    /// validation is data corruption → logged 500 (reuses the shared `corrupt`
    /// helper, as the workout row mappers do).
    ///
    /// # Errors
    /// Returns [`ApiError::Internal`] when a stored macro fails domain validation.
    pub fn into_nutrition_log(self) -> ApiResult<NutritionLog> {
        let id = self.id;
        let macros = Macros {
            protein: Grams::try_new(self.protein_g, "protein_g")
                .map_err(|_| corrupt(id, "protein_g"))?,
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

const NUTRITION_COLUMNS: &str =
    "id, user_id, performed_on, protein_g, carbs_g, fat_g, created_at, updated_at";

/// Insert a nutrition log; maps a per-day unique violation to
/// [`ApiError::AlreadyExists`] (→ 409), as `insert_user` maps a duplicate email.
///
/// # Errors
/// Returns [`ApiError::AlreadyExists`] when the caller already has a log for
/// that date, or [`ApiError::Database`] on any other query failure.
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
    .bind(id)
    .bind(user_id.0)
    .bind(new.performed_on)
    .bind(new.macros.protein.get())
    .bind(new.macros.carbs.get())
    .bind(new.macros.fat.get())
    .fetch_one(pool)
    .await;

    match result {
        Ok(row) => row.into_nutrition_log(),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(ApiError::AlreadyExists)
        }
        Err(e) => Err(ApiError::Database(e)),
    }
}

/// All of the caller's logs, newest `performed_on` first.
///
/// # Errors
/// Returns [`ApiError::Database`] on a query failure, or [`ApiError::Internal`]
/// if a stored row fails domain validation on read-back.
pub async fn find_nutrition_logs_by_user(
    pool: &PgPool,
    user_id: UserId,
) -> ApiResult<Vec<NutritionLog>> {
    let rows: Vec<NutritionRow> = sqlx::query_as(&format!(
        "SELECT {NUTRITION_COLUMNS} FROM nutrition_logs \
         WHERE user_id = $1 ORDER BY performed_on DESC, created_at DESC"
    ))
    .bind(user_id.0)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(NutritionRow::into_nutrition_log)
        .collect()
}

/// One log if it exists and is owned by the caller, else `None` (→ 404).
///
/// # Errors
/// Returns [`ApiError::Database`] on a query failure, or [`ApiError::Internal`]
/// if the stored row fails domain validation on read-back.
pub async fn find_nutrition_log_by_id(
    pool: &PgPool,
    user_id: UserId,
    id: Uuid,
) -> ApiResult<Option<NutritionLog>> {
    let row: Option<NutritionRow> = sqlx::query_as(&format!(
        "SELECT {NUTRITION_COLUMNS} FROM nutrition_logs WHERE id = $1 AND user_id = $2"
    ))
    .bind(id)
    .bind(user_id.0)
    .fetch_optional(pool)
    .await?;

    row.map(NutritionRow::into_nutrition_log).transpose()
}

/// Full-replace edit; `None` when the log is missing or owned by another user
/// (→ 404). The row is updated in place (`created_at` preserved, `updated_at`
/// bumped). A `performed_on` collision with another of the caller's logs
/// surfaces as a unique violation, auto-mapped to `AlreadyExists`/409 by
/// `ApiError::into_response` (error.rs) — no pre-check query is issued
/// (SPEC-0005 §2.5 / OQ-C3).
///
/// # Errors
/// Returns [`ApiError::AlreadyExists`] on a date collision (via the auto-map),
/// [`ApiError::Database`] on any other query failure, or [`ApiError::Internal`]
/// if the stored row fails domain validation on read-back.
pub async fn update_nutrition_log(
    pool: &PgPool,
    user_id: UserId,
    id: Uuid,
    new: &NewNutritionLog,
) -> ApiResult<Option<NutritionLog>> {
    let row: Option<NutritionRow> = sqlx::query_as(
        "UPDATE nutrition_logs \
         SET performed_on = $1, protein_g = $2, carbs_g = $3, fat_g = $4, updated_at = NOW() \
         WHERE id = $5 AND user_id = $6 \
         RETURNING id, user_id, performed_on, protein_g, carbs_g, fat_g, created_at, updated_at",
    )
    .bind(new.performed_on)
    .bind(new.macros.protein.get())
    .bind(new.macros.carbs.get())
    .bind(new.macros.fat.get())
    .bind(id)
    .bind(user_id.0)
    .fetch_optional(pool)
    .await?;

    row.map(NutritionRow::into_nutrition_log).transpose()
}

/// Delete the caller's log; `false` when it is missing or owned by another user
/// (→ 404).
///
/// # Errors
/// Returns [`ApiError::Database`] on a query failure.
pub async fn delete_nutrition_log(pool: &PgPool, user_id: UserId, id: Uuid) -> ApiResult<bool> {
    let result = sqlx::query("DELETE FROM nutrition_logs WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id.0)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// --- R-0006 photo sessions -------------------------------------------------

#[derive(Debug, FromRow)]
pub struct PhotoSessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub performed_on: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct PhotoRow {
    pub id: Uuid,
    pub session_id: Uuid,
    pub angle: Option<String>,
    pub storage_key: String,
    pub content_type: String,
    pub byte_size: i64,
    pub created_at: DateTime<Utc>,
}

/// The owner-scoped storage location of one photo (download/delete need the key
/// + content type without the full aggregate).
pub struct PhotoLocation {
    pub storage_key: String,
    pub content_type: String,
}

fn into_session_photo(row: &PhotoRow) -> ApiResult<SessionPhoto> {
    let angle = match &row.angle {
        Some(a) => Some(Angle::parse(a).map_err(|_| corrupt(row.id, "angle"))?),
        None => None,
    };
    let content_type =
        ImageContentType::parse(&row.content_type).map_err(|_| corrupt(row.id, "content_type"))?;
    Ok(SessionPhoto {
        id: row.id,
        angle,
        content_type,
        byte_size: row.byte_size,
        created_at: row.created_at,
    })
}

const PHOTO_COLUMNS: &str =
    "id, session_id, angle, storage_key, content_type, byte_size, created_at";

/// Create an empty photo session for the caller (`performed_on` = today).
///
/// # Errors
/// [`ApiError::Database`] on a query failure.
pub async fn insert_photo_session(
    pool: &PgPool,
    user_id: UserId,
    performed_on: NaiveDate,
) -> ApiResult<PhotoSession> {
    let row: PhotoSessionRow = sqlx::query_as(
        "INSERT INTO photo_sessions (id, user_id, performed_on) VALUES ($1, $2, $3) \
         RETURNING id, user_id, performed_on, created_at, updated_at",
    )
    .bind(Uuid::new_v4())
    .bind(user_id.0)
    .bind(performed_on)
    .fetch_one(pool)
    .await?;
    Ok(PhotoSession {
        id: row.id,
        user_id: UserId(row.user_id),
        performed_on: row.performed_on,
        photos: Vec::new(),
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

/// Lightweight ownership check for the upload/photo-delete authorize path —
/// avoids assembling the full photo list.
///
/// # Errors
/// [`ApiError::Database`] on a query failure.
pub async fn session_exists_for_user(
    pool: &PgPool,
    user_id: UserId,
    session_id: Uuid,
) -> ApiResult<bool> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM photo_sessions WHERE id = $1 AND user_id = $2)",
    )
    .bind(session_id)
    .bind(user_id.0)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

async fn photos_for_session(pool: &PgPool, session_id: Uuid) -> ApiResult<Vec<SessionPhoto>> {
    let rows: Vec<PhotoRow> = sqlx::query_as(&format!(
        "SELECT {PHOTO_COLUMNS} FROM photo_session_photos WHERE session_id = $1 \
         ORDER BY created_at, id"
    ))
    .bind(session_id)
    .fetch_all(pool)
    .await?;
    rows.iter().map(into_session_photo).collect()
}

fn assemble(row: &PhotoSessionRow, photos: Vec<SessionPhoto>) -> PhotoSession {
    PhotoSession {
        id: row.id,
        user_id: UserId(row.user_id),
        performed_on: row.performed_on,
        photos,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

/// All of the caller's sessions, newest `performed_on` first, each with its
/// photos' metadata.
///
/// # Errors
/// [`ApiError::Database`] on a query failure; [`ApiError::Internal`] if a stored
/// photo fails domain validation on read-back.
pub async fn find_photo_sessions_by_user(
    pool: &PgPool,
    user_id: UserId,
) -> ApiResult<Vec<PhotoSession>> {
    let sessions: Vec<PhotoSessionRow> = sqlx::query_as(
        "SELECT id, user_id, performed_on, created_at, updated_at FROM photo_sessions \
         WHERE user_id = $1 ORDER BY performed_on DESC, created_at DESC",
    )
    .bind(user_id.0)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::with_capacity(sessions.len());
    for row in sessions {
        let photos = photos_for_session(pool, row.id).await?;
        out.push(assemble(&row, photos));
    }
    Ok(out)
}

/// One session with its photos if it exists and is owned by the caller, else
/// `None` (→ 404).
///
/// # Errors
/// [`ApiError::Database`] on a query failure; [`ApiError::Internal`] on corrupt
/// stored photo metadata.
pub async fn find_photo_session_by_id(
    pool: &PgPool,
    user_id: UserId,
    session_id: Uuid,
) -> ApiResult<Option<PhotoSession>> {
    let row: Option<PhotoSessionRow> = sqlx::query_as(
        "SELECT id, user_id, performed_on, created_at, updated_at FROM photo_sessions \
         WHERE id = $1 AND user_id = $2",
    )
    .bind(session_id)
    .bind(user_id.0)
    .fetch_optional(pool)
    .await?;

    match row {
        None => Ok(None),
        Some(row) => {
            let photos = photos_for_session(pool, row.id).await?;
            Ok(Some(assemble(&row, photos)))
        }
    }
}

/// Insert a photo row (after its bytes are in the store).
///
/// # Errors
/// [`ApiError::Database`] on a query failure; [`ApiError::Internal`] if the
/// just-inserted row fails read-back validation.
pub async fn insert_photo(
    pool: &PgPool,
    session_id: Uuid,
    photo_id: Uuid,
    new: &NewPhoto,
    storage_key: &str,
) -> ApiResult<SessionPhoto> {
    let angle = new.angle.map(serde_plain_angle);
    let row: PhotoRow = sqlx::query_as(&format!(
        "INSERT INTO photo_session_photos \
         (id, session_id, angle, storage_key, content_type, byte_size) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING {PHOTO_COLUMNS}"
    ))
    .bind(photo_id)
    .bind(session_id)
    .bind(angle)
    .bind(storage_key)
    .bind(new.content_type.as_str())
    .bind(new.byte_size)
    .fetch_one(pool)
    .await?;
    into_session_photo(&row)
}

/// The owner-scoped storage location of one photo (download/delete), else `None`
/// (→ 404). Ownership is enforced by the join on `photo_sessions.user_id`.
///
/// # Errors
/// [`ApiError::Database`] on a query failure.
pub async fn find_photo_location(
    pool: &PgPool,
    user_id: UserId,
    session_id: Uuid,
    photo_id: Uuid,
) -> ApiResult<Option<PhotoLocation>> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT p.storage_key, p.content_type FROM photo_session_photos p \
         JOIN photo_sessions s ON p.session_id = s.id \
         WHERE p.id = $1 AND s.id = $2 AND s.user_id = $3",
    )
    .bind(photo_id)
    .bind(session_id)
    .bind(user_id.0)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(storage_key, content_type)| PhotoLocation {
        storage_key,
        content_type,
    }))
}

/// One photo of a session as a match candidate: enough to read its bytes
/// (`storage_key`), choose by angle, and tell the estimator the encoding. The
/// `storage_key` stays server-side (it never crosses the wire). (R-0013 §2.5.)
pub struct MatchCandidate {
    pub angle: Option<Angle>,
    pub content_type: ImageContentType,
    pub storage_key: String,
}

/// The owner-scoped photos of a session, in stored order, as match candidates
/// (R-0013). Ownership is enforced by the join on `photo_sessions.user_id`; an
/// empty result means the session has no photos (the caller maps that to 422).
///
/// # Errors
/// [`ApiError::Database`] on a query failure; [`ApiError::Internal`] if a stored
/// `angle`/`content_type` fails domain validation (corrupt data).
pub async fn match_candidates_for_session(
    pool: &PgPool,
    user_id: UserId,
    session_id: Uuid,
) -> ApiResult<Vec<MatchCandidate>> {
    let rows: Vec<(Uuid, Option<String>, String, String)> = sqlx::query_as(
        "SELECT p.id, p.angle, p.content_type, p.storage_key FROM photo_session_photos p \
         JOIN photo_sessions s ON p.session_id = s.id \
         WHERE s.id = $1 AND s.user_id = $2 ORDER BY p.created_at ASC, p.id ASC",
    )
    .bind(session_id)
    .bind(user_id.0)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|(id, angle, content_type, storage_key)| {
            let angle = angle
                .map(|a| Angle::parse(&a).map_err(|_| corrupt(id, "angle")))
                .transpose()?;
            let content_type =
                ImageContentType::parse(&content_type).map_err(|_| corrupt(id, "content_type"))?;
            Ok(MatchCandidate {
                angle,
                content_type,
                storage_key,
            })
        })
        .collect()
}

/// Delete one photo row. Returns `false` (→ 404) when missing/foreign.
///
/// # Errors
/// [`ApiError::Database`] on a query failure.
pub async fn delete_photo_row(pool: &PgPool, photo_id: Uuid) -> ApiResult<bool> {
    let result = sqlx::query("DELETE FROM photo_session_photos WHERE id = $1")
        .bind(photo_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// The storage keys of every photo in the caller's session (for byte cleanup on
/// session delete).
///
/// # Errors
/// [`ApiError::Database`] on a query failure.
pub async fn photo_keys_for_session(pool: &PgPool, session_id: Uuid) -> ApiResult<Vec<String>> {
    let keys: Vec<(String,)> =
        sqlx::query_as("SELECT storage_key FROM photo_session_photos WHERE session_id = $1")
            .bind(session_id)
            .fetch_all(pool)
            .await?;
    Ok(keys.into_iter().map(|(k,)| k).collect())
}

/// Delete the caller's session (FK-cascades its photo rows). `false` (→ 404)
/// when missing/foreign.
///
/// # Errors
/// [`ApiError::Database`] on a query failure.
pub async fn delete_photo_session(
    pool: &PgPool,
    user_id: UserId,
    session_id: Uuid,
) -> ApiResult<bool> {
    let result = sqlx::query("DELETE FROM photo_sessions WHERE id = $1 AND user_id = $2")
        .bind(session_id)
        .bind(user_id.0)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// ---------------------------------------------------------------------------
// user_programs (R-0014, SPEC-0014 §2.3)
// ---------------------------------------------------------------------------

/// A `user_programs` row returned from the DB. The `program` and `diet` fields
/// are JSONB blobs deserialized into their typed structs by the caller.
#[derive(Debug, FromRow)]
pub struct UserProgramRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub archetype_id: String,
    pub source_session_id: Option<Uuid>,
    pub program: serde_json::Value,
    pub diet: serde_json::Value,
    pub active: bool,
    pub chosen_at: DateTime<Utc>,
}

/// Insert a new program row, deactivate every previous active row for the same
/// user, and return the inserted row — all inside one transaction.
///
/// # Errors
/// [`ApiError::Database`] on any query failure.
pub async fn insert_program(
    pool: &PgPool,
    user_id: UserId,
    archetype_id: &str,
    source_session_id: Option<Uuid>,
    program: &fitai_core::program::GeneratedProgram,
    diet: &fitai_core::program::GeneratedDiet,
) -> ApiResult<UserProgramRow> {
    let program_json = serde_json::to_value(program)
        .map_err(|e| ApiError::Internal(eyre::eyre!("serialise program: {e}")))?;
    let diet_json = serde_json::to_value(diet)
        .map_err(|e| ApiError::Internal(eyre::eyre!("serialise diet: {e}")))?;

    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;

    sqlx::query("UPDATE user_programs SET active = FALSE WHERE user_id = $1 AND active = TRUE")
        .bind(user_id.0)
        .execute(&mut *tx)
        .await?;

    let row = sqlx::query_as::<_, UserProgramRow>(
        "INSERT INTO user_programs \
           (user_id, archetype_id, source_session_id, program, diet) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id, user_id, archetype_id, source_session_id, \
                   program, diet, active, chosen_at",
    )
    .bind(user_id.0)
    .bind(archetype_id)
    .bind(source_session_id)
    .bind(&program_json)
    .bind(&diet_json)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(row)
}

/// Fetch the most-recently-chosen active program for the caller.
///
/// # Errors
/// [`ApiError::Database`] on a query failure.
pub async fn get_current_program(
    pool: &PgPool,
    user_id: UserId,
) -> ApiResult<Option<UserProgramRow>> {
    Ok(sqlx::query_as::<_, UserProgramRow>(
        "SELECT id, user_id, archetype_id, source_session_id, \
                program, diet, active, chosen_at \
         FROM user_programs \
         WHERE user_id = $1 AND active = TRUE \
         ORDER BY chosen_at DESC \
         LIMIT 1",
    )
    .bind(user_id.0)
    .fetch_optional(pool)
    .await?)
}

/// Fetch the caller's program history, newest first, with limit/offset
/// pagination.
///
/// Returns `(rows, total_count)`.
///
/// # Errors
/// [`ApiError::Database`] on any query failure.
pub async fn get_program_history(
    pool: &PgPool,
    user_id: UserId,
    limit: i64,
    offset: i64,
) -> ApiResult<(Vec<UserProgramRow>, i64)> {
    let rows = sqlx::query_as::<_, UserProgramRow>(
        "SELECT id, user_id, archetype_id, source_session_id, \
                program, diet, active, chosen_at \
         FROM user_programs \
         WHERE user_id = $1 \
         ORDER BY chosen_at DESC \
         LIMIT $2 OFFSET $3",
    )
    .bind(user_id.0)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM user_programs WHERE user_id = $1")
        .bind(user_id.0)
        .fetch_one(pool)
        .await?;

    Ok((rows, total.0))
}

/// The canonical lowercase token for an [`Angle`] (its serde encoding) for the
/// `angle` column.
fn serde_plain_angle(angle: Angle) -> &'static str {
    match angle {
        Angle::Front => "front",
        Angle::Back => "back",
        Angle::Left => "left",
        Angle::Right => "right",
        Angle::Other => "other",
    }
}
