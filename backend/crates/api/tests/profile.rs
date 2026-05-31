//! R-0003 user-profile integration suite — GET/PUT /profile/me.
//!
//! Authored by the qa agent during R-0003 step 3 (test planning), BEFORE the
//! profile implementation exists. Pre-implementation red state = the
//! `/profile/me` routes are absent (404/405) and the `user_profiles` table /
//! migration are absent, so every assertion below fails. Implementation step 5
//! (the `00002_user_profiles.sql` migration, the `core::profile` module, the
//! `api::profile` handlers, and the `ApiError::NotFound` variant) makes these
//! green.
//!
//! Every test is `#[sqlx::test(migrations = "../../migrations")]` per
//! SPEC-0003 §6 (SAC9): sqlx provisions a fresh per-test database, applies the
//! migrations (including the new `user_profiles` table once it exists), and
//! hands a connected `PgPool` to the test — trivially isolated.
//!
//! SAC → test traceability lives in the qa sign-off report; each test below is
//! tagged inline with the SAC/AC branch it verifies.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Weight/body-fat assertions scale by 10 and `.round()` to integer-valued
// f64s before comparing — exact by construction; `==` is correct here.
#![allow(clippy::float_cmp)]
// Test doc comments quote JSON/array literals as prose, not code.
#![allow(clippy::doc_markdown)]

mod common;

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use common::{body_json, build_app, get_with_auth, put_json_with_auth, register_and_token};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

/// A well-formed PUT body (age 30 on any plausible "today").
fn valid_body() -> Value {
    json!({
        "date_of_birth": "1996-01-01",
        "height_cm": 180,
        "weight_kg": 80.0,
        "goals": ["build_muscle"],
        "sex": "male",
        "body_fat_percentage": 20.0
    })
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

// ---------------------------------------------------------------------------
// AC1 / SAC1: migration applied — user_profiles table shape + cascade.
// (The migration is applied transitively by `#[sqlx::test]`; these tests pin
// the column set/nullability and the ON DELETE CASCADE behaviour directly.)
// ---------------------------------------------------------------------------

/// AC1: the `user_profiles` table exists with exactly the expected columns and
/// the expected nullability (required columns NOT NULL; sex/body_fat nullable).
#[sqlx::test(migrations = "../../migrations")]
async fn migration_creates_user_profiles_table_with_expected_columns(pool: PgPool) {
    let rows = sqlx::query(
        "SELECT column_name, is_nullable \
         FROM information_schema.columns WHERE table_name = 'user_profiles'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let mut columns: Vec<(String, String)> = rows
        .iter()
        .map(|r| {
            (
                r.get::<String, _>("column_name"),
                r.get::<String, _>("is_nullable"),
            )
        })
        .collect();
    columns.sort();

    let names: Vec<&str> = columns.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "body_fat_percentage",
            "created_at",
            "date_of_birth",
            "goals",
            "height_cm",
            "sex",
            "updated_at",
            "user_id",
            "weight_kg",
        ],
        "user_profiles must have exactly the nine expected columns"
    );

    let nullable = |col: &str| -> &str {
        columns
            .iter()
            .find(|(n, _)| n == col)
            .map(|(_, n)| n.as_str())
            .unwrap()
    };
    for required in [
        "user_id",
        "date_of_birth",
        "height_cm",
        "weight_kg",
        "goals",
        "created_at",
        "updated_at",
    ] {
        assert_eq!(nullable(required), "NO", "`{required}` must be NOT NULL");
    }
    assert_eq!(nullable("sex"), "YES", "sex must be nullable");
    assert_eq!(
        nullable("body_fat_percentage"),
        "YES",
        "body_fat_percentage must be nullable"
    );
}

/// AC1: deleting a `users` row cascades to its `user_profiles` row.
#[sqlx::test(migrations = "../../migrations")]
async fn deleting_user_cascades_to_profile(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "cascade@b.com", "8charsmin").await;

    let resp = put_json_with_auth(&app, "/profile/me", Some(&bearer(&token)), valid_body()).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let uid = uuid::Uuid::parse_str(&user_id).unwrap();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(uid)
        .execute(&pool)
        .await
        .unwrap();

    let remaining: i64 = sqlx::query("SELECT COUNT(*) AS n FROM user_profiles WHERE user_id = $1")
        .bind(uid)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("n");
    assert_eq!(
        remaining, 0,
        "deleting the user must cascade to its profile"
    );
}

// ---------------------------------------------------------------------------
// AC2 / SAC2: GET /profile/me.
// ---------------------------------------------------------------------------

/// AC2: GET with a valid token but no profile yet → 404 + `{"error":"not_found"}`.
#[sqlx::test(migrations = "../../migrations")]
async fn get_before_any_profile_is_not_found(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "noprofile@b.com", "8charsmin").await;

    let resp = get_with_auth(&app, "/profile/me", Some(&bearer(&token))).await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await, json!({ "error": "not_found" }));
}

/// AC2: GET with no Authorization header → 401.
#[sqlx::test(migrations = "../../migrations")]
async fn get_without_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool);

    let resp = get_with_auth(&app, "/profile/me", None).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(resp).await, json!({ "error": "unauthorized" }));
}

/// AC2: GET with a malformed token → 401.
#[sqlx::test(migrations = "../../migrations")]
async fn get_with_invalid_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool);

    let resp = get_with_auth(&app, "/profile/me", Some("Bearer not.a.jwt")).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// AC3 / SAC3: PUT /profile/me — create (201), replace (200), single row.
// ---------------------------------------------------------------------------

/// AC3: first PUT for a caller → 201 Created with the stored profile.
#[sqlx::test(migrations = "../../migrations")]
async fn put_first_time_creates_with_201(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "create@b.com", "8charsmin").await;

    let resp = put_json_with_auth(&app, "/profile/me", Some(&bearer(&token)), valid_body()).await;

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert_eq!(body["user_id"].as_str().unwrap(), user_id);
    assert_eq!(body["height_cm"].as_i64().unwrap(), 180);

    let count: i64 = sqlx::query("SELECT COUNT(*) AS n FROM user_profiles")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("n");
    assert_eq!(count, 1, "exactly one profile row after first write");
}

/// AC3: a second PUT replaces in place → 200 OK, a strictly greater
/// `updated_at`, and still exactly one row for that user (upsert, no duplicate).
#[sqlx::test(migrations = "../../migrations")]
async fn put_replace_returns_200_bumps_updated_at_single_row(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "replace@b.com", "8charsmin").await;

    let first = put_json_with_auth(&app, "/profile/me", Some(&bearer(&token)), valid_body()).await;
    assert_eq!(first.status(), StatusCode::CREATED);
    let first_body = body_json(first).await;
    let created_at: DateTime<Utc> = first_body["created_at"].as_str().unwrap().parse().unwrap();
    let first_updated: DateTime<Utc> = first_body["updated_at"].as_str().unwrap().parse().unwrap();

    let mut changed = valid_body();
    changed["weight_kg"] = json!(85.5);
    let second = put_json_with_auth(&app, "/profile/me", Some(&bearer(&token)), changed).await;

    assert_eq!(second.status(), StatusCode::OK, "replace must return 200");
    let second_body = body_json(second).await;
    assert_eq!(
        (second_body["weight_kg"].as_f64().unwrap() * 10.0).round(),
        855.0,
        "the replaced weight must be persisted"
    );
    let second_created: DateTime<Utc> =
        second_body["created_at"].as_str().unwrap().parse().unwrap();
    let second_updated: DateTime<Utc> =
        second_body["updated_at"].as_str().unwrap().parse().unwrap();
    assert_eq!(second_created, created_at, "created_at must be preserved");
    assert!(
        second_updated > first_updated,
        "updated_at must be bumped on replace ({second_updated} !> {first_updated})"
    );

    let count: i64 = sqlx::query("SELECT COUNT(*) AS n FROM user_profiles WHERE user_id = $1")
        .bind(uuid::Uuid::parse_str(&user_id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("n");
    assert_eq!(count, 1, "replace must never create a second row");
}

/// AC3: PUT with no Authorization header → 401 (and writes nothing).
#[sqlx::test(migrations = "../../migrations")]
async fn put_without_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool.clone());

    let resp = put_json_with_auth(&app, "/profile/me", None, valid_body()).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let count: i64 = sqlx::query("SELECT COUNT(*) AS n FROM user_profiles")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("n");
    assert_eq!(count, 0, "an unauthorized PUT must write nothing");
}

// ---------------------------------------------------------------------------
// AC4 / SAC4: response shape — full field set, derived age, null optionals.
// ---------------------------------------------------------------------------

/// AC4: GET after a write → 200 with the full field set, the arithmetically
/// correct derived `age`, and canonical-string `sex`/`goals`.
#[sqlx::test(migrations = "../../migrations")]
async fn get_after_write_returns_full_body_with_derived_age(pool: PgPool) {
    let app = build_app(pool);
    let (user_id, token) = register_and_token(&app, "shape@b.com", "8charsmin").await;

    put_json_with_auth(&app, "/profile/me", Some(&bearer(&token)), valid_body()).await;

    let resp = get_with_auth(&app, "/profile/me", Some(&bearer(&token))).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    assert_eq!(body["user_id"].as_str().unwrap(), user_id);
    assert_eq!(body["date_of_birth"].as_str().unwrap(), "1996-01-01");
    assert_eq!(body["height_cm"].as_i64().unwrap(), 180);
    assert_eq!((body["weight_kg"].as_f64().unwrap() * 10.0).round(), 800.0);
    assert_eq!(body["sex"].as_str().unwrap(), "male");
    assert_eq!(
        (body["body_fat_percentage"].as_f64().unwrap() * 10.0).round(),
        200.0
    );
    assert_eq!(
        body["goals"].as_array().unwrap(),
        &vec![json!("build_muscle")]
    );

    // age is derived (DOB 1996-01-01): with "today" >= 2026-01-01, age is 30.
    let age = body["age"].as_i64().unwrap();
    assert!(
        (29..=30).contains(&age),
        "derived age for a 1996-01-01 DOB must be 29 or 30, was {age}"
    );

    assert!(
        body["created_at"]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .is_ok(),
        "created_at must be an RFC3339 timestamp"
    );
    assert!(
        body["updated_at"]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .is_ok(),
        "updated_at must be an RFC3339 timestamp"
    );
}

/// AC4: omitted optionals serialize as JSON `null`, not absent.
#[sqlx::test(migrations = "../../migrations")]
async fn omitted_optionals_serialize_as_null(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "nulls@b.com", "8charsmin").await;

    let body = json!({
        "date_of_birth": "1996-01-01",
        "height_cm": 180,
        "weight_kg": 80.0,
        "goals": ["maintain"]
    });
    let resp = put_json_with_auth(&app, "/profile/me", Some(&bearer(&token)), body).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;

    assert!(body["sex"].is_null(), "omitted sex must be null");
    assert!(
        body["body_fat_percentage"].is_null(),
        "omitted body_fat_percentage must be null"
    );
}

// ---------------------------------------------------------------------------
// AC6 / SAC6: multi-goal body round-trips.
// ---------------------------------------------------------------------------

/// AC6: a multi-goal body (e.g. `["build_muscle", "lose_fat"]`) round-trips
/// through write and read, order preserved.
#[sqlx::test(migrations = "../../migrations")]
async fn multi_goal_body_round_trips(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "multigoal@b.com", "8charsmin").await;

    let mut body = valid_body();
    body["goals"] = json!(["build_muscle", "lose_fat"]);
    let resp = put_json_with_auth(&app, "/profile/me", Some(&bearer(&token)), body).await;

    assert_eq!(resp.status(), StatusCode::CREATED);
    assert_eq!(
        body_json(resp).await["goals"].as_array().unwrap(),
        &vec![json!("build_muscle"), json!("lose_fat")]
    );
}

// ---------------------------------------------------------------------------
// AC5 / SAC5: validation — each branch returns 400 and writes nothing.
// ---------------------------------------------------------------------------

/// Assert a PUT body is rejected with 400, and (when `field` is supplied)
/// carries `{"error":"validation","field":<field>}`. Also confirms nothing was
/// persisted.
async fn assert_rejected(pool: &PgPool, email: &str, mutate: Value, field: Option<&str>) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, email, "8charsmin").await;

    let resp = put_json_with_auth(&app, "/profile/me", Some(&bearer(&token)), mutate).await;

    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "invalid body for {email} must be 400"
    );
    if let Some(field) = field {
        assert_eq!(
            body_json(resp).await,
            json!({ "error": "validation", "field": field }),
            "validation error for {email} must name field `{field}`"
        );
    }

    let count: i64 = sqlx::query("SELECT COUNT(*) AS n FROM user_profiles")
        .fetch_one(pool)
        .await
        .unwrap()
        .get("n");
    assert_eq!(count, 0, "a rejected PUT for {email} must write nothing");
}

/// AC5: a future `date_of_birth` → 400 field `date_of_birth`.
#[sqlx::test(migrations = "../../migrations")]
async fn put_future_dob_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["date_of_birth"] = json!("2999-01-01");
    assert_rejected(&pool, "futuredob@b.com", body, Some("date_of_birth")).await;
}

/// AC5: a DOB implying age < 13 → 400 field `date_of_birth`.
#[sqlx::test(migrations = "../../migrations")]
async fn put_age_below_minimum_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["date_of_birth"] = json!("2020-01-01");
    assert_rejected(&pool, "tooyoung@b.com", body, Some("date_of_birth")).await;
}

/// AC5: a DOB implying age > 120 → 400 field `date_of_birth`.
#[sqlx::test(migrations = "../../migrations")]
async fn put_age_above_maximum_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["date_of_birth"] = json!("1850-01-01");
    assert_rejected(&pool, "tooold@b.com", body, Some("date_of_birth")).await;
}

/// AC5: `height_cm` outside [50,300] → 400 field `height_cm`.
#[sqlx::test(migrations = "../../migrations")]
async fn put_height_out_of_range_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["height_cm"] = json!(49);
    assert_rejected(&pool, "badheight@b.com", body, Some("height_cm")).await;
}

/// AC5: `weight_kg` outside [20,500] → 400 field `weight_kg`.
#[sqlx::test(migrations = "../../migrations")]
async fn put_weight_out_of_range_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["weight_kg"] = json!(10.0);
    assert_rejected(&pool, "badweight@b.com", body, Some("weight_kg")).await;
}

/// AC5: `body_fat_percentage` present and outside [1,75] → 400 field
/// `body_fat_percentage`.
#[sqlx::test(migrations = "../../migrations")]
async fn put_body_fat_out_of_range_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["body_fat_percentage"] = json!(90.0);
    assert_rejected(&pool, "badbf@b.com", body, Some("body_fat_percentage")).await;
}

/// AC5: empty `goals` → 400 field `goals`.
#[sqlx::test(migrations = "../../migrations")]
async fn put_empty_goals_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["goals"] = json!([]);
    assert_rejected(&pool, "emptygoals@b.com", body, Some("goals")).await;
}

/// AC5: duplicate `goals` → 400 field `goals`.
#[sqlx::test(migrations = "../../migrations")]
async fn put_duplicate_goals_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["goals"] = json!(["build_muscle", "build_muscle"]);
    assert_rejected(&pool, "dupgoals@b.com", body, Some("goals")).await;
}

/// AC5/AC6: a goal outside the controlled set → 400 (serde-rejected to field
/// `body`, per SPEC-0003 §2.3 — AC5/AC6 require only status 400 here).
#[sqlx::test(migrations = "../../migrations")]
async fn put_unknown_goal_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["goals"] = json!(["bulk"]);
    assert_rejected(&pool, "unknowngoal@b.com", body, None).await;
}

/// AC5: `sex` not one of male/female → 400 (serde-rejected, status only).
#[sqlx::test(migrations = "../../migrations")]
async fn put_unknown_sex_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["sex"] = json!("other");
    assert_rejected(&pool, "unknownsex@b.com", body, None).await;
}

/// AC5: a missing required field (`height_cm`) → 400.
#[sqlx::test(migrations = "../../migrations")]
async fn put_missing_required_field_is_rejected(pool: PgPool) {
    let body = json!({
        "date_of_birth": "1996-01-01",
        "weight_kg": 80.0,
        "goals": ["maintain"]
    });
    assert_rejected(&pool, "missingfield@b.com", body, None).await;
}

// ---------------------------------------------------------------------------
// AC7 / SAC7: cross-user isolation.
// ---------------------------------------------------------------------------

/// AC7: user A's PUT never mutates user B's row, and A's GET never returns B's
/// data. The subject is always the token's `sub`.
#[sqlx::test(migrations = "../../migrations")]
async fn profiles_are_isolated_per_user(pool: PgPool) {
    let app = build_app(pool.clone());
    let (id_a, token_a) = register_and_token(&app, "alice@b.com", "8charsmin").await;
    let (id_b, token_b) = register_and_token(&app, "bob@b.com", "8charsmin").await;
    assert_ne!(id_a, id_b);

    // B writes a distinctive profile first.
    let mut body_b = valid_body();
    body_b["height_cm"] = json!(200);
    body_b["goals"] = json!(["gain_strength"]);
    let resp_b = put_json_with_auth(&app, "/profile/me", Some(&bearer(&token_b)), body_b).await;
    assert_eq!(resp_b.status(), StatusCode::CREATED);

    // A writes its own, different profile.
    let mut body_a = valid_body();
    body_a["height_cm"] = json!(160);
    body_a["goals"] = json!(["lose_fat"]);
    let resp_a = put_json_with_auth(&app, "/profile/me", Some(&bearer(&token_a)), body_a).await;
    assert_eq!(resp_a.status(), StatusCode::CREATED);
    assert_eq!(
        body_json(resp_a).await["user_id"].as_str().unwrap(),
        id_a,
        "A's write must be keyed by A's own sub"
    );

    // A's GET returns A's data only.
    let get_a = get_with_auth(&app, "/profile/me", Some(&bearer(&token_a))).await;
    let a_body = body_json(get_a).await;
    assert_eq!(a_body["user_id"].as_str().unwrap(), id_a);
    assert_eq!(a_body["height_cm"].as_i64().unwrap(), 160);
    assert_eq!(
        a_body["goals"].as_array().unwrap(),
        &vec![json!("lose_fat")]
    );

    // B's row is untouched by A's write.
    let get_b = get_with_auth(&app, "/profile/me", Some(&bearer(&token_b))).await;
    let b_body = body_json(get_b).await;
    assert_eq!(b_body["user_id"].as_str().unwrap(), id_b);
    assert_eq!(
        b_body["height_cm"].as_i64().unwrap(),
        200,
        "A's write must not mutate B's profile"
    );
    assert_eq!(
        b_body["goals"].as_array().unwrap(),
        &vec![json!("gain_strength")]
    );

    // Two distinct rows, one per user.
    let count: i64 = sqlx::query("SELECT COUNT(*) AS n FROM user_profiles")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("n");
    assert_eq!(count, 2, "each user must own exactly one distinct row");
}
