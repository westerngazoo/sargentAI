//! Photo-session domain: the photo write model, its value types, and the read
//! aggregates. Pure — no DB, no HTTP, no storage. Parse-don't-validate, as
//! `profile`/`workout`/`nutrition`.
//!
//! The image bytes themselves are never modelled here (they live in the object
//! store); this module owns only the validated metadata.

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use crate::UserId;

/// The maximum accepted image size, in bytes (10 MiB).
pub const MAX_BYTES: i64 = 10 * 1024 * 1024;

/// The angle a progress photo was taken from. Optional metadata; the wire
/// vocabulary is the canonical lowercase set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Angle {
    Front,
    Back,
    Left,
    Right,
    Other,
}

impl Angle {
    /// Parse a wire token into an [`Angle`] (case-sensitive lowercase set).
    ///
    /// # Errors
    /// [`PhotoError::AngleUnknown`] for anything outside the controlled set.
    pub fn parse(s: &str) -> Result<Self, PhotoError> {
        match s {
            "front" => Ok(Self::Front),
            "back" => Ok(Self::Back),
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            "other" => Ok(Self::Other),
            _ => Err(PhotoError::AngleUnknown),
        }
    }
}

/// The accepted image content types (the AC3 allowlist).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ImageContentType {
    #[serde(rename = "image/jpeg")]
    ImageJpeg,
    #[serde(rename = "image/png")]
    ImagePng,
}

impl ImageContentType {
    /// Parse a MIME string into the allowlisted type.
    ///
    /// # Errors
    /// [`PhotoError::ContentTypeUnsupported`] for anything but `image/jpeg`/`image/png`.
    pub fn parse(s: &str) -> Result<Self, PhotoError> {
        match s {
            "image/jpeg" => Ok(Self::ImageJpeg),
            "image/png" => Ok(Self::ImagePng),
            _ => Err(PhotoError::ContentTypeUnsupported),
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ImageJpeg => "image/jpeg",
            Self::ImagePng => "image/png",
        }
    }
}

/// A validated photo write model (no identity/timestamps/storage key).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewPhoto {
    pub angle: Option<Angle>,
    pub content_type: ImageContentType,
    pub byte_size: i64,
}

impl NewPhoto {
    /// Validate a photo's metadata. `byte_size` must be in `[1, MAX_BYTES]`.
    ///
    /// # Errors
    /// [`PhotoError::ByteSizeOutOfRange`] for an empty, negative, or oversize image.
    pub fn new(
        angle: Option<Angle>,
        content_type: ImageContentType,
        byte_size: i64,
    ) -> Result<Self, PhotoError> {
        if !(1..=MAX_BYTES).contains(&byte_size) {
            return Err(PhotoError::ByteSizeOutOfRange);
        }
        Ok(Self {
            angle,
            content_type,
            byte_size,
        })
    }
}

/// A stored photo's metadata, reconstructed from a row. Intentionally serializes
/// **without** its `storage_key` — the key is internal (SPEC-0006 §2.6, AC9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SessionPhoto {
    pub id: Uuid,
    pub angle: Option<Angle>,
    pub content_type: ImageContentType,
    pub byte_size: i64,
    pub created_at: DateTime<Utc>,
}

/// A stored photo session, reconstructed from rows: metadata + its photos.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PhotoSession {
    pub id: Uuid,
    pub user_id: UserId,
    pub performed_on: NaiveDate,
    pub photos: Vec<SessionPhoto>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PhotoError {
    #[error("angle is not a known value")]
    AngleUnknown,
    #[error("content type must be image/jpeg or image/png")]
    ContentTypeUnsupported,
    #[error("image is empty or larger than the limit")]
    ByteSizeOutOfRange,
}

impl PhotoError {
    /// The request field this error concerns — drives `ApiError::Validation`.
    #[must_use]
    pub fn field(&self) -> &'static str {
        match self {
            PhotoError::AngleUnknown => "angle",
            PhotoError::ContentTypeUnsupported => "content_type",
            PhotoError::ByteSizeOutOfRange => "file",
        }
    }
}
