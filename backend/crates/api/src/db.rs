//! Postgres-side types and queries. Maps row shapes to `fitai_core` types
//! at the seam so callers never see `password_hash`.

use chrono::{DateTime, Utc};
use sqlx::{prelude::FromRow, PgPool};
use uuid::Uuid;

use fitai_core::{Email, User, UserId};

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
