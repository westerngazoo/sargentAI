//! R-0004 workout-log integration suite — POST/GET/PUT/DELETE /workouts.
//!
//! Authored by the qa agent during R-0004 step 3 (test planning), BEFORE the
//! workout implementation exists. Pre-implementation red state = the
//! `00003_workout_logs.sql` migration and the `/workouts` routes are absent, so
//! every assertion below fails. Implementation step 5 (the migration, the
//! `core::workout` module, the `api::workout` handlers, the transactional
//! `api::db` queries, and the `crate::http::parse_body` helper) makes them green.
//!
//! Every test is `#[sqlx::test(migrations = "../../migrations")]` per SPEC-0004
//! §6 (the R-0002/R-0003 harness): sqlx provisions a fresh per-test database,
//! applies the migrations (including the new workout tables once they exist),
//! and hands a connected `PgPool` to the test — trivially isolated.
//!
//! SAC → test traceability lives in the qa sign-off report; each test below is
//! tagged inline with the SAC/AC branch it verifies.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Numeric assertions compare integer-valued / exact f64s that pass through the
// transparent newtypes unchanged — `==` is correct here.
#![allow(clippy::float_cmp)]
// Test doc comments quote JSON/array literals as prose, not code.
#![allow(clippy::doc_markdown)]

mod common;

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use common::{
    body_json, build_app, delete_with_auth, get_with_auth, post_json_with_auth,
    put_json_with_auth, register_and_token,
};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

/// A well-formed session body: one chest exercise with two sets. `performed_on`
/// is a clearly-past date so it is never in the future on any plausible "today".
fn valid_body() -> Value {
    json!({
        "performed_on": "2026-05-01",
        "exercises": [{
            "name": "Bench Press",
            "muscle_group": "chest",
            "sets": [
                { "reps": 10, "weight_kg": 100.0, "rpe": 8.0 },
                { "reps": 8, "weight_kg": 105.0, "rpe": 9.0 }
            ]
        }]
    })
}

/// A second, distinctively-dated body (a *later* `performed_on`), used to assert
/// newest-first ordering.
fn valid_body_newer() -> Value {
    json!({
        "performed_on": "2026-05-15",
        "exercises": [{
            "name": "Deadlift",
            "muscle_group": "back",
            "sets": [{ "reps": 5, "weight_kg": 180.0, "rpe": 9.5 }]
        }]
    })
}

/// COUNT(*) over a workout table — used to assert "writes nothing" / cascade.
async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query(&format!("SELECT COUNT(*) AS n FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
        .get("n")
}

// ===========================================================================
// AC1 / SAC1: migration applied — three tables, columns/nullability, cascades.
// ===========================================================================

/// AC1: the three workout tables exist with the expected columns and the
/// expected nullability (required NOT NULL; muscle_group/weight_kg/rpe nullable).
#[sqlx::test(migrations = "../../migrations")]
async fn migration_creates_workout_tables_with_expected_columns(pool: PgPool) {
    let column_set = |rows: &[sqlx::postgres::PgRow]| -> Vec<(String, String)> {
        let mut cols: Vec<(String, String)> = rows
            .iter()
            .map(|r| {
                (
                    r.get::<String, _>("column_name"),
                    r.get::<String, _>("is_nullable"),
                )
            })
            .collect();
        cols.sort();
        cols
    };
    let fetch = |table: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query(
                "SELECT column_name, is_nullable FROM information_schema.columns \
                 WHERE table_name = $1",
            )
            .bind(table)
            .fetch_all(&pool)
            .await
            .unwrap()
        }
    };

    let sessions = column_set(&fetch("workout_sessions").await);
    let names: Vec<&str> = sessions.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        vec!["created_at", "id", "performed_on", "updated_at", "user_id"],
        "workout_sessions must have exactly the five expected columns"
    );

    let exercises = column_set(&fetch("workout_exercises").await);
    let names: Vec<&str> = exercises.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        vec!["id", "muscle_group", "name", "position", "session_id"],
        "workout_exercises must have exactly the five expected columns"
    );
    assert_eq!(
        exercises.iter().find(|(n, _)| n == "muscle_group").unwrap().1,
        "YES",
        "muscle_group must be nullable"
    );

    let sets = column_set(&fetch("workout_sets").await);
    let names: Vec<&str> = sets.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        vec!["exercise_id", "id", "position", "reps", "rpe", "weight_kg"],
        "workout_sets must have exactly the six expected columns"
    );
    for nullable in ["weight_kg", "rpe"] {
        assert_eq!(
            sets.iter().find(|(n, _)| n == nullable).unwrap().1,
            "YES",
            "`{nullable}` must be nullable"
        );
    }
    assert_eq!(
        sets.iter().find(|(n, _)| n == "reps").unwrap().1,
        "NO",
        "reps must be NOT NULL"
    );
}

/// AC1: deleting a `users` row cascades down sessions -> exercises -> sets.
#[sqlx::test(migrations = "../../migrations")]
async fn deleting_user_cascades_to_sessions_exercises_sets(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "cascade@b.com", "8charsmin").await;

    let resp = post_json_with_auth(&app, "/workouts", Some(&bearer(&token)), valid_body()).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    assert_eq!(count(&pool, "workout_sessions").await, 1);
    assert_eq!(count(&pool, "workout_exercises").await, 1);
    assert_eq!(count(&pool, "workout_sets").await, 2);

    let uid = uuid::Uuid::parse_str(&user_id).unwrap();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(uid)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(count(&pool, "workout_sessions").await, 0, "sessions must cascade");
    assert_eq!(count(&pool, "workout_exercises").await, 0, "exercises must cascade");
    assert_eq!(count(&pool, "workout_sets").await, 0, "sets must cascade");
}

/// AC1: deleting a session cascades to its exercises and sets.
#[sqlx::test(migrations = "../../migrations")]
async fn deleting_session_cascades_to_exercises_sets(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, "sescascade@b.com", "8charsmin").await;

    let created = post_json_with_auth(&app, "/workouts", Some(&bearer(&token)), valid_body()).await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let session_id = body_json(created).await["id"].as_str().unwrap().to_string();

    sqlx::query("DELETE FROM workout_sessions WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&session_id).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(count(&pool, "workout_exercises").await, 0, "exercises must cascade");
    assert_eq!(count(&pool, "workout_sets").await, 0, "sets must cascade");
}

// ===========================================================================
// AC2 / SAC2: POST /workouts — 201 + nested ids + persistence + 401.
// ===========================================================================

/// AC2: POST with a valid token -> 201 with the stored session, server-generated
/// ids at every level, and the full hierarchy persisted owned by the caller.
#[sqlx::test(migrations = "../../migrations")]
async fn post_creates_session_with_nested_ids_and_persists(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "create@b.com", "8charsmin").await;

    let resp = post_json_with_auth(&app, "/workouts", Some(&bearer(&token)), valid_body()).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;

    assert_eq!(body["user_id"].as_str().unwrap(), user_id, "owned by the caller");
    assert!(
        body["id"].as_str().unwrap().parse::<uuid::Uuid>().is_ok(),
        "session id must be a server-generated UUID"
    );
    let exercise = &body["exercises"][0];
    assert!(
        exercise["id"].as_str().unwrap().parse::<uuid::Uuid>().is_ok(),
        "exercise id must be a server-generated UUID"
    );
    let set = &exercise["sets"][0];
    assert!(
        set["id"].as_str().unwrap().parse::<uuid::Uuid>().is_ok(),
        "set id must be a server-generated UUID"
    );

    // The full hierarchy is persisted, owned by the caller.
    let owned: i64 = sqlx::query("SELECT COUNT(*) AS n FROM workout_sessions WHERE user_id = $1")
        .bind(uuid::Uuid::parse_str(&user_id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("n");
    assert_eq!(owned, 1, "the session must be persisted owned by the caller");
    assert_eq!(count(&pool, "workout_exercises").await, 1);
    assert_eq!(count(&pool, "workout_sets").await, 2);
}

/// AC2: server-assigned `position` is contiguous and 0-based per parent
/// (SPEC-0004 §2.6 / SAC7).
#[sqlx::test(migrations = "../../migrations")]
async fn post_assigns_contiguous_zero_based_positions(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "positions@b.com", "8charsmin").await;

    let body = json!({
        "performed_on": "2026-05-01",
        "exercises": [
            { "name": "Squat", "muscle_group": "legs",
              "sets": [{ "reps": 5 }, { "reps": 5 }, { "reps": 5 }] },
            { "name": "Leg Curl", "muscle_group": "legs",
              "sets": [{ "reps": 12 }] }
        ]
    });
    let resp = post_json_with_auth(&app, "/workouts", Some(&bearer(&token)), body).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;

    let exercises = body["exercises"].as_array().unwrap();
    assert_eq!(exercises[0]["position"].as_i64().unwrap(), 0);
    assert_eq!(exercises[1]["position"].as_i64().unwrap(), 1);

    let sets = exercises[0]["sets"].as_array().unwrap();
    assert_eq!(sets.len(), 3);
    for (i, set) in sets.iter().enumerate() {
        assert_eq!(set["position"].as_i64().unwrap(), i as i64);
    }
}

/// AC2: POST with no/invalid token -> 401 and writes nothing.
#[sqlx::test(migrations = "../../migrations")]
async fn post_without_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool.clone());

    let resp = post_json_with_auth(&app, "/workouts", None, valid_body()).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(count(&pool, "workout_sessions").await, 0, "unauthorized POST writes nothing");
}

// ===========================================================================
// AC3 / SAC3: GET /workouts — list newest-first, own-only, empty array, 401.
// ===========================================================================

/// AC3: GET with no logged sessions -> 200 with an empty array.
#[sqlx::test(migrations = "../../migrations")]
async fn list_when_empty_returns_empty_array(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "emptylist@b.com", "8charsmin").await;

    let resp = get_with_auth(&app, "/workouts", Some(&bearer(&token))).await;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(body_json(resp).await, json!([]), "no sessions -> empty array");
}

/// AC3: GET lists the caller's sessions ordered by performed_on descending
/// (newest first), each carrying its full nested exercises/sets.
#[sqlx::test(migrations = "../../migrations")]
async fn list_returns_caller_sessions_newest_first_with_nested(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "order@b.com", "8charsmin").await;

    // Insert the older session FIRST, then the newer — so creation order differs
    // from the expected performed_on-descending result order.
    let older = post_json_with_auth(&app, "/workouts", Some(&bearer(&token)), valid_body()).await;
    assert_eq!(older.status(), StatusCode::CREATED);
    let newer =
        post_json_with_auth(&app, "/workouts", Some(&bearer(&token)), valid_body_newer()).await;
    assert_eq!(newer.status(), StatusCode::CREATED);

    let resp = get_with_auth(&app, "/workouts", Some(&bearer(&token))).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let sessions = body.as_array().unwrap();

    assert_eq!(sessions.len(), 2);
    assert_eq!(
        sessions[0]["performed_on"].as_str().unwrap(),
        "2026-05-15",
        "newest performed_on must come first"
    );
    assert_eq!(sessions[1]["performed_on"].as_str().unwrap(), "2026-05-01");
    // Nested data travels with the list element.
    assert_eq!(sessions[0]["exercises"][0]["name"].as_str().unwrap(), "Deadlift");
    assert_eq!(sessions[0]["exercises"][0]["sets"][0]["reps"].as_i64().unwrap(), 5);
}

/// AC3: GET with no token -> 401.
#[sqlx::test(migrations = "../../migrations")]
async fn list_without_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool);

    let resp = get_with_auth(&app, "/workouts", None).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// AC4 / SAC4 + AC10 / SAC10: GET /workouts/:id — 200 owned, 404 missing/foreign,
// 401, nested shape.
// ===========================================================================

/// AC4: GET /:id for an owned session -> 200 with the full nested session.
#[sqlx::test(migrations = "../../migrations")]
async fn get_one_owned_returns_200_with_nested(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "getone@b.com", "8charsmin").await;

    let created = post_json_with_auth(&app, "/workouts", Some(&bearer(&token)), valid_body()).await;
    let session_id = body_json(created).await["id"].as_str().unwrap().to_string();

    let resp = get_with_auth(&app, &format!("/workouts/{session_id}"), Some(&bearer(&token))).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["id"].as_str().unwrap(), session_id);
    assert_eq!(body["exercises"][0]["name"].as_str().unwrap(), "Bench Press");
    assert_eq!(body["exercises"][0]["sets"].as_array().unwrap().len(), 2);
}

/// AC4: GET /:id for a non-existent id -> 404 (ownership never leaked).
#[sqlx::test(migrations = "../../migrations")]
async fn get_one_missing_is_not_found(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "getmissing@b.com", "8charsmin").await;

    let unknown = uuid::Uuid::new_v4();
    let resp = get_with_auth(&app, &format!("/workouts/{unknown}"), Some(&bearer(&token))).await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await, json!({ "error": "not_found" }));
}

/// AC4/AC10: GET /:id for another user's session -> 404 (never 403; existence
/// is never leaked via a distinct status).
#[sqlx::test(migrations = "../../migrations")]
async fn get_one_foreign_session_is_not_found(pool: PgPool) {
    let app = build_app(pool);
    let (_id_a, token_a) = register_and_token(&app, "ownerA@b.com", "8charsmin").await;
    let (_id_b, token_b) = register_and_token(&app, "intruderB@b.com", "8charsmin").await;

    let created = post_json_with_auth(&app, "/workouts", Some(&bearer(&token_a)), valid_body()).await;
    let session_id = body_json(created).await["id"].as_str().unwrap().to_string();

    // B asks for A's session — must look identical to a missing id.
    let resp =
        get_with_auth(&app, &format!("/workouts/{session_id}"), Some(&bearer(&token_b))).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// AC4: GET /:id with no token -> 401.
#[sqlx::test(migrations = "../../migrations")]
async fn get_one_without_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool);
    let id = uuid::Uuid::new_v4();

    let resp = get_with_auth(&app, &format!("/workouts/{id}"), None).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// AC7 / SAC7: the GET /:id body carries the full nested shape with the literal
/// AC7 keys, and nullable muscle_group/weight_kg/rpe serialize as JSON null.
#[sqlx::test(migrations = "../../migrations")]
async fn response_carries_full_nested_shape_with_nullable_fields(pool: PgPool) {
    let app = build_app(pool);
    let (user_id, token) = register_and_token(&app, "shape@b.com", "8charsmin").await;

    // A bodyweight set with NO muscle_group, NO weight_kg, NO rpe.
    let body = json!({
        "performed_on": "2026-05-01",
        "exercises": [{
            "name": "Pull Up",
            "sets": [{ "reps": 8 }]
        }]
    });
    let created = post_json_with_auth(&app, "/workouts", Some(&bearer(&token)), body).await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let session_id = body_json(created).await["id"].as_str().unwrap().to_string();

    let resp = get_with_auth(&app, &format!("/workouts/{session_id}"), Some(&bearer(&token))).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    // Session-level literal keys.
    assert_eq!(body["user_id"].as_str().unwrap(), user_id);
    assert_eq!(body["performed_on"].as_str().unwrap(), "2026-05-01");
    assert!(
        body["created_at"].as_str().unwrap().parse::<DateTime<Utc>>().is_ok(),
        "created_at must be an RFC3339 timestamp"
    );
    assert!(
        body["updated_at"].as_str().unwrap().parse::<DateTime<Utc>>().is_ok(),
        "updated_at must be an RFC3339 timestamp"
    );

    // Exercise-level literal keys + nullable muscle_group.
    let exercise = &body["exercises"][0];
    assert_eq!(exercise["position"].as_i64().unwrap(), 0);
    assert_eq!(exercise["name"].as_str().unwrap(), "Pull Up");
    assert!(exercise["muscle_group"].is_null(), "omitted muscle_group must be null");

    // Set-level literal keys + nullable weight_kg/rpe.
    let set = &exercise["sets"][0];
    assert_eq!(set["position"].as_i64().unwrap(), 0);
    assert_eq!(set["reps"].as_i64().unwrap(), 8);
    assert!(set["weight_kg"].is_null(), "omitted weight_kg must be null");
    assert!(set["rpe"].is_null(), "omitted rpe must be null");
}

// ===========================================================================
// AC5 / SAC5: PUT /workouts/:id — full-replace 200 + updated_at bump + new
// child ids, 404 missing/foreign, 400 invalid (writes nothing), 401.
// ===========================================================================

/// AC5: PUT /:id replaces the session transactionally -> 200, bumps updated_at,
/// updates performed_on, and assigns NEW child ids (full-replace, not diff).
#[sqlx::test(migrations = "../../migrations")]
async fn put_full_replace_returns_200_bumps_updated_at_new_child_ids(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, "replace@b.com", "8charsmin").await;

    let created = post_json_with_auth(&app, "/workouts", Some(&bearer(&token)), valid_body()).await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let first = body_json(created).await;
    let session_id = first["id"].as_str().unwrap().to_string();
    let created_at: DateTime<Utc> = first["created_at"].as_str().unwrap().parse().unwrap();
    let first_updated: DateTime<Utc> = first["updated_at"].as_str().unwrap().parse().unwrap();
    let old_exercise_id = first["exercises"][0]["id"].as_str().unwrap().to_string();
    let old_set_id = first["exercises"][0]["sets"][0]["id"].as_str().unwrap().to_string();

    // Replace with a different shape (a single new exercise/set, new date).
    let replacement = json!({
        "performed_on": "2026-05-20",
        "exercises": [{
            "name": "Overhead Press",
            "muscle_group": "shoulders",
            "sets": [{ "reps": 6, "weight_kg": 60.0, "rpe": 7.5 }]
        }]
    });
    let resp = put_json_with_auth(
        &app,
        &format!("/workouts/{session_id}"),
        Some(&bearer(&token)),
        replacement,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "full-replace must return 200");
    let second = body_json(resp).await;

    assert_eq!(second["id"].as_str().unwrap(), session_id, "session id is stable");
    assert_eq!(second["performed_on"].as_str().unwrap(), "2026-05-20", "date updated");
    assert_eq!(second["exercises"][0]["name"].as_str().unwrap(), "Overhead Press");
    assert_eq!(second["exercises"][0]["sets"].as_array().unwrap().len(), 1);

    let second_created: DateTime<Utc> = second["created_at"].as_str().unwrap().parse().unwrap();
    let second_updated: DateTime<Utc> = second["updated_at"].as_str().unwrap().parse().unwrap();
    assert_eq!(second_created, created_at, "created_at preserved");
    assert!(
        second_updated > first_updated,
        "updated_at must be bumped ({second_updated} !> {first_updated})"
    );

    // Child ids are NOT stable across a full-replace (OQ-C4 / SAC5).
    assert_ne!(
        second["exercises"][0]["id"].as_str().unwrap(),
        old_exercise_id,
        "a full-replace must assign a new exercise id"
    );
    assert_ne!(
        second["exercises"][0]["sets"][0]["id"].as_str().unwrap(),
        old_set_id,
        "a full-replace must assign a new set id"
    );

    // The replaced rows are gone; exactly the new ones remain.
    assert_eq!(count(&pool, "workout_sessions").await, 1);
    assert_eq!(count(&pool, "workout_exercises").await, 1, "old exercise replaced, not duplicated");
    assert_eq!(count(&pool, "workout_sets").await, 1, "old sets replaced, not duplicated");
}

/// AC5: PUT /:id for a non-existent id -> 404.
#[sqlx::test(migrations = "../../migrations")]
async fn put_missing_is_not_found(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "putmissing@b.com", "8charsmin").await;

    let unknown = uuid::Uuid::new_v4();
    let resp =
        put_json_with_auth(&app, &format!("/workouts/{unknown}"), Some(&bearer(&token)), valid_body())
            .await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// AC5/AC10: PUT /:id against another user's session -> 404 and leaves it
/// untouched.
#[sqlx::test(migrations = "../../migrations")]
async fn put_foreign_session_is_not_found_and_untouched(pool: PgPool) {
    let app = build_app(pool);
    let (_id_a, token_a) = register_and_token(&app, "putownerA@b.com", "8charsmin").await;
    let (_id_b, token_b) = register_and_token(&app, "putintruderB@b.com", "8charsmin").await;

    let created = post_json_with_auth(&app, "/workouts", Some(&bearer(&token_a)), valid_body()).await;
    let session_id = body_json(created).await["id"].as_str().unwrap().to_string();

    // B tries to overwrite A's session.
    let resp = put_json_with_auth(
        &app,
        &format!("/workouts/{session_id}"),
        Some(&bearer(&token_b)),
        valid_body_newer(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // A's session is unchanged.
    let get_a = get_with_auth(&app, &format!("/workouts/{session_id}"), Some(&bearer(&token_a))).await;
    assert_eq!(get_a.status(), StatusCode::OK);
    let body = body_json(get_a).await;
    assert_eq!(body["performed_on"].as_str().unwrap(), "2026-05-01", "A's session must be untouched");
    assert_eq!(body["exercises"][0]["name"].as_str().unwrap(), "Bench Press");
}

/// AC5/AC8: PUT /:id with an invalid body -> 400 and writes nothing (the
/// existing rows are preserved unchanged).
#[sqlx::test(migrations = "../../migrations")]
async fn put_invalid_body_is_rejected_and_writes_nothing(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, "putinvalid@b.com", "8charsmin").await;

    let created = post_json_with_auth(&app, "/workouts", Some(&bearer(&token)), valid_body()).await;
    let session_id = body_json(created).await["id"].as_str().unwrap().to_string();

    // reps out of range -> 400 field "reps".
    let mut bad = valid_body();
    bad["exercises"][0]["sets"][0]["reps"] = json!(0);
    let resp =
        put_json_with_auth(&app, &format!("/workouts/{session_id}"), Some(&bearer(&token)), bad).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body_json(resp).await, json!({ "error": "validation", "field": "reps" }));

    // The original rows are intact (the rejected PUT replaced nothing).
    assert_eq!(count(&pool, "workout_sessions").await, 1);
    assert_eq!(count(&pool, "workout_exercises").await, 1);
    assert_eq!(count(&pool, "workout_sets").await, 2, "a rejected PUT must not delete children");
}

/// AC5: PUT /:id with no token -> 401.
#[sqlx::test(migrations = "../../migrations")]
async fn put_without_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool);
    let id = uuid::Uuid::new_v4();

    let resp = put_json_with_auth(&app, &format!("/workouts/{id}"), None, valid_body()).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// AC6 / SAC6 + AC10: DELETE /workouts/:id — 204 then 404, foreign 404, 401.
// ===========================================================================

/// AC6: DELETE /:id of an owned session -> 204; a second DELETE -> 404; the
/// children are gone (cascade).
#[sqlx::test(migrations = "../../migrations")]
async fn delete_owned_then_second_delete_is_not_found(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, "delete@b.com", "8charsmin").await;

    let created = post_json_with_auth(&app, "/workouts", Some(&bearer(&token)), valid_body()).await;
    let session_id = body_json(created).await["id"].as_str().unwrap().to_string();

    let first = delete_with_auth(&app, &format!("/workouts/{session_id}"), Some(&bearer(&token))).await;
    assert_eq!(first.status(), StatusCode::NO_CONTENT);

    assert_eq!(count(&pool, "workout_sessions").await, 0);
    assert_eq!(count(&pool, "workout_exercises").await, 0, "children cascade on delete");
    assert_eq!(count(&pool, "workout_sets").await, 0);

    // A second DELETE of the same id -> 404.
    let second =
        delete_with_auth(&app, &format!("/workouts/{session_id}"), Some(&bearer(&token))).await;
    assert_eq!(second.status(), StatusCode::NOT_FOUND);
}

/// AC6/AC10: DELETE /:id of another user's session -> 404 and leaves it intact.
#[sqlx::test(migrations = "../../migrations")]
async fn delete_foreign_session_is_not_found_and_untouched(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id_a, token_a) = register_and_token(&app, "delownerA@b.com", "8charsmin").await;
    let (_id_b, token_b) = register_and_token(&app, "delintruderB@b.com", "8charsmin").await;

    let created = post_json_with_auth(&app, "/workouts", Some(&bearer(&token_a)), valid_body()).await;
    let session_id = body_json(created).await["id"].as_str().unwrap().to_string();

    let resp =
        delete_with_auth(&app, &format!("/workouts/{session_id}"), Some(&bearer(&token_b))).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // A's session survives.
    assert_eq!(count(&pool, "workout_sessions").await, 1, "B's delete must not touch A's session");
}

/// AC6: DELETE /:id with no token -> 401.
#[sqlx::test(migrations = "../../migrations")]
async fn delete_without_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool);
    let id = uuid::Uuid::new_v4();

    let resp = delete_with_auth(&app, &format!("/workouts/{id}"), None).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// AC8 / SAC8: every validation branch -> 400, writes nothing. The field-label
// asymmetry (SPEC-0004 §6): SEMANTIC failures report the leaf field; STRUCTURAL
// failures report "body".
// ===========================================================================

/// POST `mutate` as a fresh user; assert 400, optional `{error,field}` body, and
/// that nothing was written. `field == Some(f)` pins a SEMANTIC failure to leaf
/// `f`; `field == None` pins a STRUCTURAL failure (only status asserted, since
/// SPEC-0004 §6 routes these to `"body"`).
async fn assert_rejected(pool: &PgPool, email: &str, mutate: Value, field: Option<&str>) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, email, "8charsmin").await;

    let resp = post_json_with_auth(&app, "/workouts", Some(&bearer(&token)), mutate).await;

    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "invalid body for {email} must be 400"
    );
    if let Some(field) = field {
        assert_eq!(
            body_json(resp).await,
            json!({ "error": "validation", "field": field }),
            "validation error for {email} must name leaf field `{field}`"
        );
    }

    assert_eq!(count(pool, "workout_sessions").await, 0, "rejected POST for {email} writes no session");
    assert_eq!(count(pool, "workout_exercises").await, 0, "rejected POST for {email} writes no exercise");
    assert_eq!(count(pool, "workout_sets").await, 0, "rejected POST for {email} writes no set");
}

// --- Semantic failures: report the leaf field. ---

/// AC8: performed_on in the future -> 400 field "performed_on".
#[sqlx::test(migrations = "../../migrations")]
async fn post_future_performed_on_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["performed_on"] = json!("2999-01-01");
    assert_rejected(&pool, "future@b.com", body, Some("performed_on")).await;
}

/// AC8: an empty exercises array (present but []) -> 400 field "exercises".
#[sqlx::test(migrations = "../../migrations")]
async fn post_empty_exercises_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["exercises"] = json!([]);
    assert_rejected(&pool, "noex@b.com", body, Some("exercises")).await;
}

/// AC8: an exercise with an empty sets array -> 400 field "sets".
#[sqlx::test(migrations = "../../migrations")]
async fn post_empty_sets_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["exercises"][0]["sets"] = json!([]);
    assert_rejected(&pool, "nosets@b.com", body, Some("sets")).await;
}

/// AC8: a blank/whitespace exercise name -> 400 field "name".
#[sqlx::test(migrations = "../../migrations")]
async fn post_blank_name_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["exercises"][0]["name"] = json!("   ");
    assert_rejected(&pool, "blankname@b.com", body, Some("name")).await;
}

/// AC8: an exercise name longer than 100 chars -> 400 field "name".
#[sqlx::test(migrations = "../../migrations")]
async fn post_name_too_long_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["exercises"][0]["name"] = json!("a".repeat(101));
    assert_rejected(&pool, "longname@b.com", body, Some("name")).await;
}

/// AC8: reps < 1 -> 400 field "reps".
#[sqlx::test(migrations = "../../migrations")]
async fn post_reps_below_minimum_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["exercises"][0]["sets"][0]["reps"] = json!(0);
    assert_rejected(&pool, "lowreps@b.com", body, Some("reps")).await;
}

/// AC8: reps > 10000 -> 400 field "reps".
#[sqlx::test(migrations = "../../migrations")]
async fn post_reps_above_maximum_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["exercises"][0]["sets"][0]["reps"] = json!(10_001);
    assert_rejected(&pool, "highreps@b.com", body, Some("reps")).await;
}

/// AC8: weight_kg present and <= 0 -> 400 field "weight_kg".
#[sqlx::test(migrations = "../../migrations")]
async fn post_weight_zero_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["exercises"][0]["sets"][0]["weight_kg"] = json!(0.0);
    assert_rejected(&pool, "zeroweight@b.com", body, Some("weight_kg")).await;
}

/// AC8: weight_kg present and > 1000 -> 400 field "weight_kg".
#[sqlx::test(migrations = "../../migrations")]
async fn post_weight_above_maximum_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["exercises"][0]["sets"][0]["weight_kg"] = json!(1000.1);
    assert_rejected(&pool, "bigweight@b.com", body, Some("weight_kg")).await;
}

/// AC8: rpe present and outside [6,10] -> 400 field "rpe".
#[sqlx::test(migrations = "../../migrations")]
async fn post_rpe_out_of_range_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["exercises"][0]["sets"][0]["rpe"] = json!(5.5);
    assert_rejected(&pool, "lowrpe@b.com", body, Some("rpe")).await;
}

/// AC8: rpe present and not a multiple of 0.5 -> 400 field "rpe".
#[sqlx::test(migrations = "../../migrations")]
async fn post_rpe_not_half_step_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["exercises"][0]["sets"][0]["rpe"] = json!(7.3);
    assert_rejected(&pool, "oddrpe@b.com", body, Some("rpe")).await;
}

// --- Structural failures: report "body" (serde / JsonRejection). ---

/// AC8: an unknown muscle_group is serde-rejected -> 400 field "body"
/// (SPEC-0004 §2.3/§6 — the typed Option<MuscleGroup> rejects before the handler).
#[sqlx::test(migrations = "../../migrations")]
async fn post_unknown_muscle_group_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["exercises"][0]["muscle_group"] = json!("biceps");
    assert_rejected(&pool, "badmg@b.com", body, Some("body")).await;
}

/// AC8: a missing required `performed_on` -> 400 field "body" (JsonRejection).
#[sqlx::test(migrations = "../../migrations")]
async fn post_missing_performed_on_is_rejected(pool: PgPool) {
    let body = json!({
        "exercises": [{ "name": "Bench Press", "sets": [{ "reps": 10 }] }]
    });
    assert_rejected(&pool, "noperformed@b.com", body, Some("body")).await;
}

/// AC8: a missing required `exercises` array -> 400 field "body".
#[sqlx::test(migrations = "../../migrations")]
async fn post_missing_exercises_is_rejected(pool: PgPool) {
    let body = json!({ "performed_on": "2026-05-01" });
    assert_rejected(&pool, "noexkey@b.com", body, Some("body")).await;
}

/// AC8: a missing required per-set `reps` -> 400 field "body".
#[sqlx::test(migrations = "../../migrations")]
async fn post_missing_reps_is_rejected(pool: PgPool) {
    let body = json!({
        "performed_on": "2026-05-01",
        "exercises": [{ "name": "Bench Press", "sets": [{ "weight_kg": 100.0 }] }]
    });
    assert_rejected(&pool, "noreps@b.com", body, Some("body")).await;
}

// ===========================================================================
// AC10 / SAC10: cross-user isolation — A's list never returns B's sessions.
// ===========================================================================

/// AC10: two distinct users each see only their own sessions in GET /workouts.
#[sqlx::test(migrations = "../../migrations")]
async fn list_is_isolated_per_user(pool: PgPool) {
    let app = build_app(pool);
    let (id_a, token_a) = register_and_token(&app, "isoA@b.com", "8charsmin").await;
    let (id_b, token_b) = register_and_token(&app, "isoB@b.com", "8charsmin").await;
    assert_ne!(id_a, id_b);

    // B logs two sessions; A logs one.
    post_json_with_auth(&app, "/workouts", Some(&bearer(&token_b)), valid_body()).await;
    post_json_with_auth(&app, "/workouts", Some(&bearer(&token_b)), valid_body_newer()).await;
    post_json_with_auth(&app, "/workouts", Some(&bearer(&token_a)), valid_body()).await;

    let list_a = get_with_auth(&app, "/workouts", Some(&bearer(&token_a))).await;
    let a_sessions = body_json(list_a).await;
    let a_arr = a_sessions.as_array().unwrap();
    assert_eq!(a_arr.len(), 1, "A must see only its own session");
    assert_eq!(a_arr[0]["user_id"].as_str().unwrap(), id_a);

    let list_b = get_with_auth(&app, "/workouts", Some(&bearer(&token_b))).await;
    let b_sessions = body_json(list_b).await;
    let b_arr = b_sessions.as_array().unwrap();
    assert_eq!(b_arr.len(), 2, "B must see only its own two sessions");
    for s in b_arr {
        assert_eq!(s["user_id"].as_str().unwrap(), id_b, "no cross-user session leaks");
    }
}

// ===========================================================================
// AC12 / SAC12: partial-write rollback — the codebase's FIRST transaction. A
// multi-table insert that fails partway must leave ZERO rows (atomicity, the
// DB-error boundary beyond AC8's validation boundary).
// ===========================================================================

/// SAC12: force a mid-insert DB failure and assert nothing is committed.
///
/// `insert_session` writes three tables in one transaction (SPEC-0004 §2.5). To
/// provoke a failure *after* the session/exercise rows would be written, we
/// drop the `workout_sets` table inside this isolated per-test DB before the
/// call: the final set INSERT then errors, the `?` bubbles, the transaction is
/// dropped, and the session + exercise writes must roll back — leaving zero rows
/// in the surviving tables. (If writes were non-transactional, the session and
/// exercise rows would survive.) The endpoint returns 500.
#[sqlx::test(migrations = "../../migrations")]
async fn failed_multi_table_insert_rolls_back_completely(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, "rollback@b.com", "8charsmin").await;

    // Sabotage the innermost table so the set INSERT inside the tx fails.
    sqlx::query("DROP TABLE workout_sets")
        .execute(&pool)
        .await
        .unwrap();

    let resp = post_json_with_auth(&app, "/workouts", Some(&bearer(&token)), valid_body()).await;
    assert_eq!(
        resp.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "a DB failure mid-transaction must surface as 500, not a partial 201"
    );

    // The session and exercise writes must have rolled back.
    assert_eq!(
        count(&pool, "workout_sessions").await,
        0,
        "a failed transaction must leave zero session rows (atomic rollback)"
    );
    assert_eq!(
        count(&pool, "workout_exercises").await,
        0,
        "a failed transaction must leave zero exercise rows (atomic rollback)"
    );
}
