//! fitAI domain types. Pure: no DB, no HTTP, no I/O.
//!
//! Persistence and presentation live in the `fitai-api` crate.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod profile;
pub mod user;

pub use profile::{
    BodyFatPercentage, Goal, Goals, HeightCm, NewProfile, Profile, ProfileError, Sex, WeightKg,
};
pub use user::{Email, EmailParseError, User, UserId};
