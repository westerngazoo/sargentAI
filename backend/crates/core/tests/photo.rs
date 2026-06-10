//! Unit tests for the `fitai_core::photo` domain — the single validation
//! authority for the photo-session write model (SPEC-0006 §2.2, §3.1).
//!
//! Authored by the qa agent during R-0006 step 3 (test planning), BEFORE the
//! `core::photo` module exists. Pre-implementation red state = compile failure
//! (the module / types are absent). Implementation step 5 makes these green.
//!
//! Coverage:
//! - SAC4 → AC4: `Angle` parsing of the controlled set, the unknown-angle error,
//!   and the lowercase serde encoding agreeing with the parse vocabulary;
//! - SAC3 → AC3: `ImageContentType` parsing of the `image/jpeg`/`image/png`
//!   allowlist (and rejection of everything else), `as_str` round-trip;
//! - SAC3 → AC3: `NewPhoto::new` size validation — the `1..=MAX_BYTES` window,
//!   its boundaries, and the empty/oversize rejections, each tagged with the
//!   right `field()`;
//! - SAC5/SAC9 → AC5/AC9: the read aggregates serialize with the literal wire
//!   keys, and `SessionPhoto` JSON carries NO `storage_key` (privacy — the key
//!   is internal-only, SPEC-0006 §2.6).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Test doc comments quote JSON literals and content-type strings as prose.
#![allow(clippy::doc_markdown)]

use chrono::{DateTime, NaiveDate, Utc};
use fitai_core::{
    Angle, ImageContentType, NewPhoto, PhotoError, PhotoSession, SessionPhoto, UserId, MAX_BYTES,
};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// SAC4 / AC4: Angle — the controlled set front/back/left/right/other.
// ---------------------------------------------------------------------------

/// Every `Angle` variant paired with its canonical wire string — keeps the
/// agreement tests exhaustive.
const ALL_ANGLES: [(Angle, &str); 5] = [
    (Angle::Front, "front"),
    (Angle::Back, "back"),
    (Angle::Left, "left"),
    (Angle::Right, "right"),
    (Angle::Other, "other"),
];

#[test]
fn angle_parse_accepts_every_controlled_value() {
    for (angle, raw) in ALL_ANGLES {
        assert_eq!(
            Angle::parse(raw),
            Ok(angle),
            "parse({raw:?}) must yield {angle:?}"
        );
    }
}

#[test]
fn angle_parse_rejects_an_unknown_value() {
    let err = Angle::parse("sideways").expect_err("an unknown angle must be rejected");
    assert_eq!(err, PhotoError::AngleUnknown);
    assert_eq!(err.field(), "angle");
}

#[test]
fn angle_parse_is_case_sensitive_to_the_canonical_lowercase() {
    // The controlled vocabulary is lowercase; a different casing is not a member.
    assert_eq!(Angle::parse("Front"), Err(PhotoError::AngleUnknown));
    assert_eq!(Angle::parse("BACK"), Err(PhotoError::AngleUnknown));
}

#[test]
fn angle_serde_serializes_as_lowercase_matching_the_parse_vocabulary() {
    for (angle, raw) in ALL_ANGLES {
        let json = serde_json::to_string(&angle).unwrap();
        assert_eq!(
            json,
            format!("\"{raw}\""),
            "serde must lowercase-encode {angle:?} as {raw:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// SAC3 / AC3: ImageContentType — the image/jpeg | image/png allowlist.
// ---------------------------------------------------------------------------

#[test]
fn content_type_parse_accepts_jpeg_and_png() {
    assert_eq!(
        ImageContentType::parse("image/jpeg"),
        Ok(ImageContentType::ImageJpeg)
    );
    assert_eq!(
        ImageContentType::parse("image/png"),
        Ok(ImageContentType::ImagePng)
    );
}

#[test]
fn content_type_as_str_round_trips() {
    for ct in [ImageContentType::ImageJpeg, ImageContentType::ImagePng] {
        assert_eq!(
            ImageContentType::parse(ct.as_str()),
            Ok(ct),
            "parse(as_str(v)) must equal v for {ct:?}"
        );
    }
    assert_eq!(ImageContentType::ImageJpeg.as_str(), "image/jpeg");
    assert_eq!(ImageContentType::ImagePng.as_str(), "image/png");
}

#[test]
fn content_type_parse_rejects_non_image_types() {
    for raw in [
        "image/gif",
        "image/webp",
        "application/json",
        "text/plain",
        "application/octet-stream",
        "",
    ] {
        let err = ImageContentType::parse(raw)
            .expect_err(&format!("{raw:?} is not an allowed image content type"));
        assert_eq!(err, PhotoError::ContentTypeUnsupported);
        assert_eq!(err.field(), "content_type");
    }
}

// ---------------------------------------------------------------------------
// SAC3 / AC3: NewPhoto::new — size validation over the [1, MAX_BYTES] window.
// ---------------------------------------------------------------------------

#[test]
fn max_bytes_is_ten_mebibytes() {
    assert_eq!(MAX_BYTES, 10 * 1024 * 1024);
}

#[test]
fn new_photo_accepts_a_one_byte_image() {
    let photo = NewPhoto::new(Some(Angle::Front), ImageContentType::ImagePng, 1)
        .expect("a 1-byte image is in range");
    assert_eq!(photo.angle, Some(Angle::Front));
    assert_eq!(photo.content_type, ImageContentType::ImagePng);
    assert_eq!(photo.byte_size, 1);
}

#[test]
fn new_photo_accepts_an_image_at_the_max_size() {
    let photo = NewPhoto::new(None, ImageContentType::ImageJpeg, MAX_BYTES)
        .expect("exactly MAX_BYTES is in range");
    assert_eq!(photo.byte_size, MAX_BYTES);
}

#[test]
fn new_photo_rejects_an_empty_image() {
    let err = NewPhoto::new(Some(Angle::Back), ImageContentType::ImagePng, 0)
        .expect_err("a 0-byte image must be rejected");
    assert_eq!(err, PhotoError::ByteSizeOutOfRange);
    assert_eq!(err.field(), "file");
}

#[test]
fn new_photo_rejects_a_negative_byte_size() {
    let err = NewPhoto::new(None, ImageContentType::ImagePng, -1)
        .expect_err("a negative byte size must be rejected");
    assert_eq!(err, PhotoError::ByteSizeOutOfRange);
    assert_eq!(err.field(), "file");
}

#[test]
fn new_photo_rejects_an_oversize_image() {
    let err = NewPhoto::new(None, ImageContentType::ImageJpeg, MAX_BYTES + 1)
        .expect_err("MAX_BYTES + 1 must be rejected");
    assert_eq!(err, PhotoError::ByteSizeOutOfRange);
    assert_eq!(err.field(), "file");
}

#[test]
fn new_photo_accepts_an_absent_angle() {
    let photo = NewPhoto::new(None, ImageContentType::ImagePng, 4096)
        .expect("a photo with no angle is valid (AC4)");
    assert_eq!(photo.angle, None);
}

// ---------------------------------------------------------------------------
// SAC3/SAC4 / AC3/AC4: PhotoError.field() routes each variant to its request
// field, exactly as the workout/nutrition error idiom.
// ---------------------------------------------------------------------------

#[test]
fn photo_error_field_attribution_is_exhaustive() {
    assert_eq!(PhotoError::AngleUnknown.field(), "angle");
    assert_eq!(PhotoError::ContentTypeUnsupported.field(), "content_type");
    assert_eq!(PhotoError::ByteSizeOutOfRange.field(), "file");
}

// ---------------------------------------------------------------------------
// SAC5/SAC9 / AC5/AC9: the read aggregates serialize with the literal wire keys,
// and a SessionPhoto NEVER serializes its internal storage_key. The handlers
// serialize these core aggregates directly (SPEC-0006 §2.6), so the privacy
// contract — no storage_key on the wire — is pinned here at the type level; the
// integration suite re-asserts it end-to-end.
// ---------------------------------------------------------------------------

fn sample_session() -> PhotoSession {
    PhotoSession {
        id: Uuid::nil(),
        user_id: UserId(Uuid::nil()),
        performed_on: NaiveDate::from_ymd_opt(2026, 5, 30).unwrap(),
        created_at: "2026-05-30T12:00:00Z".parse::<DateTime<Utc>>().unwrap(),
        updated_at: "2026-05-30T12:00:00Z".parse::<DateTime<Utc>>().unwrap(),
        photos: vec![SessionPhoto {
            id: Uuid::nil(),
            angle: Some(Angle::Front),
            content_type: ImageContentType::ImageJpeg,
            byte_size: 2048,
            created_at: "2026-05-30T12:00:00Z".parse::<DateTime<Utc>>().unwrap(),
        }],
    }
}

#[test]
fn session_serializes_with_the_expected_keys() {
    let json = serde_json::to_value(sample_session()).unwrap();

    for key in [
        "id",
        "user_id",
        "performed_on",
        "photos",
        "created_at",
        "updated_at",
    ] {
        assert!(json.get(key).is_some(), "session JSON must carry `{key}`");
    }
    assert_eq!(json["performed_on"], serde_json::json!("2026-05-30"));
}

#[test]
fn session_photo_serializes_metadata_only_and_never_the_storage_key() {
    let json = serde_json::to_value(sample_session()).unwrap();
    let photo = &json["photos"][0];

    for key in ["id", "angle", "content_type", "byte_size", "created_at"] {
        assert!(photo.get(key).is_some(), "photo JSON must carry `{key}`");
    }
    assert_eq!(photo["angle"], serde_json::json!("front"));
    assert_eq!(photo["content_type"], serde_json::json!("image/jpeg"));
    assert_eq!(photo["byte_size"], serde_json::json!(2048));

    // The privacy invariant (SPEC-0006 §2.6 / AC9): the storage key never
    // crosses the wire — not as `storage_key`, nor under any alias.
    assert!(
        photo.get("storage_key").is_none(),
        "SessionPhoto JSON must NOT carry a storage_key key"
    );
    let key_names: Vec<&str> = photo
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        key_names.len(),
        5,
        "SessionPhoto must serialize exactly five metadata keys, got {key_names:?}"
    );
}

#[test]
fn session_photo_serializes_an_absent_angle_as_null() {
    let mut session = sample_session();
    session.photos[0].angle = None;
    let json = serde_json::to_value(session).unwrap();
    assert!(
        json["photos"][0]["angle"].is_null(),
        "an absent angle must serialize as null"
    );
}

#[test]
fn session_serializes_an_empty_photo_list_as_an_array() {
    let mut session = sample_session();
    session.photos.clear();
    let json = serde_json::to_value(session).unwrap();
    assert_eq!(
        json["photos"],
        serde_json::json!([]),
        "a session with no photos must serialize photos as []"
    );
}
