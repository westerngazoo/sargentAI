//! `User` and its identifier / email value-object.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// User identifier — newtype around `Uuid` so it can't be mixed with other
/// id types in handler signatures.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(pub Uuid);

impl UserId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Email newtype with a validating constructor. The format is checked by
/// the `validator` crate at the handler boundary; this type guarantees
/// "well-formed at construction" so downstream code can rely on the
/// invariant without re-validating. It is also the single normalization
/// authority (trim + lowercase) so the write path and the login-lookup path
/// can never disagree on the stored value.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Email(String);

#[derive(Debug, Error)]
#[error("invalid email format")]
pub struct EmailParseError;

impl Email {
    /// Construct from a `&str`. Returns `EmailParseError` on a malformed
    /// input. Trusts only basic shape: presence of `@` with non-empty
    /// local + domain parts and a dot in the domain. Stricter checking is
    /// the `validator` crate's job at the handler boundary.
    ///
    /// # Errors
    /// Returns [`EmailParseError`] when the input is not a well-formed
    /// address.
    pub fn parse(raw: &str) -> Result<Self, EmailParseError> {
        let trimmed = raw.trim();
        let (local, domain) = trimmed.split_once('@').ok_or(EmailParseError)?;
        if local.is_empty() || domain.is_empty() || !domain.contains('.') {
            return Err(EmailParseError);
        }
        Ok(Self(trimmed.to_ascii_lowercase()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Domain `User`. Note: no `password_hash` field — that's a persistence
/// detail kept in `fitai_api::db::UserRow`. No `Deserialize`: `User` is only
/// ever produced from a `UserRow` (DB), never parsed from the wire — and
/// `Email` intentionally has no `Deserialize` (it must go through `parse`).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct User {
    pub id: UserId,
    pub email: Email,
    pub created_at: DateTime<Utc>,
}
