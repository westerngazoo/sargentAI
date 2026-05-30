//! Postgres-side types and queries. Maps row shapes to `fitai_core` types
//! at the seam so callers never see `password_hash`.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{prelude::FromRow, PgPool, Row as _};
use uuid::Uuid;

use fitai_core::{
    BodyFatPercentage, Email, Goal, Goals, HeightCm, NewProfile, Profile, Sex, User, UserId,
    WeightKg,
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
