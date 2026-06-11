//! R-0012 archetype-library read-API integration suite — GET /archetypes and
//! GET /archetypes/:id, plus the wire-privacy contract on `ArchetypeResponse`.
//!
//! Authored by the qa agent during R-0012 step 3 (test planning), BEFORE the
//! `api::archetype` module exists. Pre-implementation red state = the
//! `/archetypes` routes are absent (the router does not merge an archetype
//! router) and `core::archetype::library()` does not exist, so this crate fails
//! to COMPILE. Implementation step 5 (the `core::archetype` module, the
//! `api::archetype` handlers + `ArchetypeResponse` DTO, and the router merge)
//! makes these green.
//!
//! The library is static reference data read from `core::archetype::library()`
//! (SPEC-0012 §2.3) — no migration, no new `AppState` field. Tests still use
//! `#[sqlx::test]` because `register_and_token` needs the auth tables (the read
//! API is authenticated, SPEC-0012 §2.5); the archetypes themselves never touch
//! the DB.
//!
//! SAC → test traceability lives in the qa sign-off report; each test below is
//! tagged inline with the SAC/AC branch it verifies.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Test doc comments quote JSON keys and slug strings as prose.
#![allow(clippy::doc_markdown)]

mod common;

use axum::http::StatusCode;
use common::{body_json, build_app, get_with_auth, register_and_token};
use serde_json::json;
use sqlx::PgPool;

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

/// The six owner-approved kebab-slug ids are an implementation detail of the
/// seed; the suite resolves a real slug from the list response rather than
/// hard-coding one, so it stays robust to the exact slug strings while still
/// pinning count and per-id behaviour.
async fn first_slug(app: &axum::Router, token: &str) -> String {
    let resp = get_with_auth(app, "/archetypes", Some(&bearer(token))).await;
    assert_eq!(resp.status(), StatusCode::OK);
    body_json(resp).await[0]["id"]
        .as_str()
        .expect("each archetype must carry a string id")
        .to_string()
}

// ===========================================================================
// SAC3 / AC3: GET /archetypes — authenticated list of all six.
// ===========================================================================

/// AC3: GET /archetypes with a token → 200 + exactly six elements.
#[sqlx::test(migrations = "../../migrations")]
async fn list_returns_200_and_the_six_archetypes(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "arch-list@b.com", "8charsmin").await;

    let resp = get_with_auth(&app, "/archetypes", Some(&bearer(&token))).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    let items = body.as_array().expect("the list must be a JSON array");
    assert_eq!(
        items.len(),
        6,
        "the read API must list exactly the six seeded archetypes"
    );

    // Each element carries the user-facing wire shape and a unique id.
    let mut ids = std::collections::HashSet::new();
    for item in items {
        let id = item["id"].as_str().expect("each element must carry an id");
        assert!(ids.insert(id.to_string()), "ids in the list must be unique");
        assert!(
            !item["display_name"].as_str().unwrap().is_empty(),
            "each element must carry a non-empty display_name"
        );
        assert!(
            item["goals_served"].as_array().map(Vec::len).unwrap_or(0) >= 1,
            "each element must serve at least one goal"
        );
    }
    assert_eq!(ids.len(), 6);
}

/// AC3: GET /archetypes with no token → 401.
#[sqlx::test(migrations = "../../migrations")]
async fn list_without_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool);
    let resp = get_with_auth(&app, "/archetypes", None).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(resp).await, json!({ "error": "unauthorized" }));
}

/// AC3: GET /archetypes with a malformed token → 401.
#[sqlx::test(migrations = "../../migrations")]
async fn list_with_invalid_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool);
    let resp = get_with_auth(&app, "/archetypes", Some("Bearer not.a.jwt")).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// SAC3 / AC3: GET /archetypes/:id — known slug 200, unknown 404, no token 401.
// ===========================================================================

/// AC3: GET /archetypes/:id for a known slug → 200 with that archetype.
#[sqlx::test(migrations = "../../migrations")]
async fn get_one_known_slug_returns_200(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "arch-get@b.com", "8charsmin").await;

    let slug = first_slug(&app, &token).await;
    let resp = get_with_auth(&app, &format!("/archetypes/{slug}"), Some(&bearer(&token))).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(
        body["id"].as_str().unwrap(),
        slug,
        "GET /archetypes/:id must return the addressed archetype"
    );
    assert!(
        !body["display_name"].as_str().unwrap().is_empty(),
        "the single archetype must carry a display_name"
    );
}

/// AC3: GET /archetypes/:id for every known slug → 200 (each is addressable).
#[sqlx::test(migrations = "../../migrations")]
async fn get_one_resolves_every_listed_slug(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "arch-all@b.com", "8charsmin").await;

    let list = get_with_auth(&app, "/archetypes", Some(&bearer(&token))).await;
    let body = body_json(list).await;
    let slugs: Vec<String> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(slugs.len(), 6);

    for slug in slugs {
        let resp = get_with_auth(&app, &format!("/archetypes/{slug}"), Some(&bearer(&token))).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "slug {slug:?} from the list must resolve via GET /archetypes/:id"
        );
        assert_eq!(body_json(resp).await["id"].as_str().unwrap(), slug);
    }
}

/// AC3: GET /archetypes/:id for an unknown slug → 404 with the uniform body.
#[sqlx::test(migrations = "../../migrations")]
async fn get_one_unknown_slug_is_not_found(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "arch-404@b.com", "8charsmin").await;

    let resp = get_with_auth(&app, "/archetypes/no-such-archetype", Some(&bearer(&token))).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await, json!({ "error": "not_found" }));
}

/// AC3: GET /archetypes/:id with no token → 401.
#[sqlx::test(migrations = "../../migrations")]
async fn get_one_without_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool);
    let resp = get_with_auth(&app, "/archetypes/heavy-duty-mass", None).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// SAC4 / AC4: the wire NEVER carries internal_name or sources. The famous
// research labels and source notes are internal-only (likeness/legal); the
// `ArchetypeResponse` DTO omits them (SPEC-0012 §2.4).
// ===========================================================================

/// AC4: a single archetype's JSON carries `display_name` but NO `internal_name`
/// and NO `sources` key.
#[sqlx::test(migrations = "../../migrations")]
async fn get_one_omits_internal_name_and_sources(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "arch-priv1@b.com", "8charsmin").await;

    let slug = first_slug(&app, &token).await;
    let resp = get_with_auth(&app, &format!("/archetypes/{slug}"), Some(&bearer(&token))).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    // The user-facing field is present.
    assert!(
        body.get("display_name").and_then(|v| v.as_str()).is_some(),
        "the response must carry display_name"
    );
    // The internal-only fields are absent at the top level.
    assert!(
        body.get("internal_name").is_none(),
        "ArchetypeResponse must NOT carry internal_name"
    );
    assert!(
        body.get("sources").is_none(),
        "ArchetypeResponse must NOT carry a sources key at the top level"
    );
    // ...and nowhere nested either (e.g. inside a provenance object).
    let serialized = body.to_string();
    assert!(
        !serialized.contains("internal_name"),
        "the archetype JSON must not contain the substring internal_name"
    );
    assert!(
        !serialized.contains("sources"),
        "the archetype JSON must not contain the substring sources"
    );
}

/// AC4: the same privacy contract holds across the whole list — no element
/// anywhere leaks `internal_name` or `sources`.
#[sqlx::test(migrations = "../../migrations")]
async fn list_never_leaks_internal_name_or_sources(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "arch-priv2@b.com", "8charsmin").await;

    let resp = get_with_auth(&app, "/archetypes", Some(&bearer(&token))).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    for item in body.as_array().unwrap() {
        assert!(
            item.get("internal_name").is_none(),
            "no list element may carry internal_name"
        );
        assert!(
            item.get("sources").is_none(),
            "no list element may carry sources"
        );
    }
    // The famous internal research labels must never appear anywhere in the wire.
    let serialized = body.to_string();
    for internal_label in ["Yates", "Mentzer", "Arnold", "Columbu", "Cutler", "Heath"] {
        assert!(
            !serialized.contains(internal_label),
            "the wire must never expose the internal label {internal_label:?}"
        );
    }
    assert!(
        !serialized.contains("internal_name") && !serialized.contains("sources"),
        "the list JSON must not contain internal_name or sources"
    );
}

// ===========================================================================
// SAC6 / AC6: the response surfaces the matchable frame profile — the numeric
// shoulder_to_waist ratio R-0013 computes distance against, plus the program /
// diet templates and the abstracted content. (No matching is performed here —
// that is R-0013; this only asserts the shape crosses the wire.)
// ===========================================================================

/// AC6: an archetype's JSON exposes its frame_profile with the numeric
/// shoulder_to_waist ratio and the program/diet templates.
#[sqlx::test(migrations = "../../migrations")]
async fn response_exposes_the_matchable_frame_profile_and_templates(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "arch-shape@b.com", "8charsmin").await;

    let slug = first_slug(&app, &token).await;
    let resp = get_with_auth(&app, &format!("/archetypes/{slug}"), Some(&bearer(&token))).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    let frame = &body["frame_profile"];
    let ratio = frame["shoulder_to_waist"]
        .as_f64()
        .expect("frame_profile must carry a numeric shoulder_to_waist");
    assert!(
        (1.0..=2.5).contains(&ratio),
        "the wire ratio {ratio} must be within the documented matchable range"
    );
    assert!(
        frame
            .get("structure_tags")
            .and_then(|v| v.as_array())
            .is_some(),
        "frame_profile must expose a structure_tags array"
    );

    assert!(
        body.get("program_template").is_some(),
        "the response must carry the program_template"
    );
    assert!(
        body.get("diet_template").is_some(),
        "the response must carry the diet_template"
    );
    // The provenance level (confidence) crosses the wire; the source notes do not.
    assert!(
        body.get("confidence").and_then(|v| v.as_str()).is_some(),
        "the response must carry the provenance confidence level"
    );
}
