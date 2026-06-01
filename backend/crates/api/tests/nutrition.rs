//! R-0005 nutrition-log integration suite — POST/GET/PUT/DELETE /nutrition.
//!
//! Authored by the qa agent during R-0005 step 3 (test planning), BEFORE the
//! nutrition implementation exists. Pre-implementation red state = the
//! `00004_nutrition_logs.sql` migration and the `/nutrition` routes are absent,
//! so every assertion below fails. Implementation step 5 (the migration, the
//! `core::nutrition` module, the `api::nutrition` handlers, and the `api::db`
//! queries) makes them green.
//!
//! Every test is `#[sqlx::test(migrations = "../../migrations")]` per SPEC-0005
//! §6 (the R-0002/R-0003/R-0004 harness): sqlx provisions a fresh per-test
//! database, applies the migrations (including the new nutrition table once it
//! exists), and hands a connected `PgPool` to the test — trivially isolated.
//!
//! SAC → test traceability lives in the qa sign-off report; each test below is
//! tagged inline with the SAC/AC branch it verifies.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Numeric assertions compare exact f64s (macros pass through unchanged; the
// calorie formula is exact 4/4/9 arithmetic on terminating decimals) — `==` is
// correct here.
#![allow(clippy::float_cmp)]
// Test doc comments quote JSON/array literals as prose, not code.
#![allow(clippy::doc_markdown)]

mod common;

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use common::{
    body_json, build_app, delete_with_auth, get_with_auth, post_json_with_auth, put_json_with_auth,
    register_and_token,
};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

/// A valid nutrition body. `performed_on` is a clearly-past date so it is never
/// in the future on any plausible "today". Macros chosen so calories are an
/// exact, easy-to-assert 4·150 + 4·300 + 9·80 = 2520.
fn valid_body() -> Value {
    json!({
        "performed_on": "2026-05-01",
        "protein_g": 150.0,
        "carbs_g": 300.0,
        "fat_g": 80.0
    })
}

/// A second, distinctively-dated body (a *later* `performed_on`) used to assert
/// newest-first ordering and as a distinct second day.
fn valid_body_newer() -> Value {
    json!({
        "performed_on": "2026-05-15",
        "protein_g": 200.0,
        "carbs_g": 250.0,
        "fat_g": 60.0
    })
}

/// COUNT(*) over `nutrition_logs` — used to assert "writes nothing" / cascade.
async fn count(pool: &PgPool) -> i64 {
    sqlx::query("SELECT COUNT(*) AS n FROM nutrition_logs")
        .fetch_one(pool)
        .await
        .unwrap()
        .get("n")
}

// ===========================================================================
// AC1 / SAC1: migration applied — columns, the per-day unique constraint, the
// FK cascade, and NO calories column.
// ===========================================================================

/// AC1: `nutrition_logs` has exactly the expected columns and NO `calories`.
#[sqlx::test(migrations = "../../migrations")]
async fn migration_creates_nutrition_logs_with_expected_columns(pool: PgPool) {
    let rows = sqlx::query(
        "SELECT column_name FROM information_schema.columns WHERE table_name = 'nutrition_logs'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let mut names: Vec<String> = rows
        .iter()
        .map(|r| r.get::<String, _>("column_name"))
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "carbs_g",
            "created_at",
            "fat_g",
            "id",
            "performed_on",
            "protein_g",
            "updated_at",
            "user_id",
        ],
        "nutrition_logs must have exactly the eight expected columns (no calories column)"
    );
}

/// AC1: deleting a `users` row cascades to its nutrition logs.
#[sqlx::test(migrations = "../../migrations")]
async fn deleting_user_cascades_to_nutrition_logs(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "cascade@b.com", "8charsmin").await;

    let resp = post_json_with_auth(&app, "/nutrition", Some(&bearer(&token)), valid_body()).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    assert_eq!(count(&pool).await, 1);

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&user_id).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(count(&pool).await, 0, "nutrition logs must cascade");
}

// ===========================================================================
// AC2 / SAC2 + AC7/AC9: POST /nutrition — 201 + derived calories + persistence,
// duplicate-date 409, 401.
// ===========================================================================

/// AC2/AC7/AC9: POST -> 201 with the stored log, derived `calories`, all AC7
/// literal keys, persisted owned by the caller.
#[sqlx::test(migrations = "../../migrations")]
async fn post_creates_log_with_derived_calories_and_persists(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "create@b.com", "8charsmin").await;

    let resp = post_json_with_auth(&app, "/nutrition", Some(&bearer(&token)), valid_body()).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;

    assert_eq!(
        body["user_id"].as_str().unwrap(),
        user_id,
        "owned by caller"
    );
    assert!(
        body["id"].as_str().unwrap().parse::<uuid::Uuid>().is_ok(),
        "id must be a server-generated UUID"
    );
    assert_eq!(body["performed_on"].as_str().unwrap(), "2026-05-01");
    assert_eq!(body["protein_g"].as_f64().unwrap(), 150.0);
    assert_eq!(body["carbs_g"].as_f64().unwrap(), 300.0);
    assert_eq!(body["fat_g"].as_f64().unwrap(), 80.0);
    // AC9: calories derived as 4·150 + 4·300 + 9·80 = 2520.
    assert_eq!(
        body["calories"].as_f64().unwrap(),
        2520.0,
        "calories must be derived 4·protein + 4·carbs + 9·fat"
    );
    assert!(
        body["created_at"]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .is_ok(),
        "created_at must be RFC3339"
    );
    assert!(
        body["updated_at"]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .is_ok(),
        "updated_at must be RFC3339"
    );

    // AC7: no stored calories column; the wire carries the literal keys only.
    let keys: Vec<&str> = body
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    for key in [
        "id",
        "user_id",
        "performed_on",
        "protein_g",
        "carbs_g",
        "fat_g",
        "calories",
        "created_at",
        "updated_at",
    ] {
        assert!(keys.contains(&key), "response must carry key `{key}`");
    }

    let owned: i64 = sqlx::query("SELECT COUNT(*) AS n FROM nutrition_logs WHERE user_id = $1")
        .bind(uuid::Uuid::parse_str(&user_id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("n");
    assert_eq!(owned, 1, "the log must be persisted owned by the caller");
}

/// AC2: POST for a date the caller already logged -> 409 and writes nothing.
#[sqlx::test(migrations = "../../migrations")]
async fn post_duplicate_date_is_conflict_and_writes_nothing(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, "dup@b.com", "8charsmin").await;

    let first = post_json_with_auth(&app, "/nutrition", Some(&bearer(&token)), valid_body()).await;
    assert_eq!(first.status(), StatusCode::CREATED);

    // Same performed_on, different macros — still a per-day conflict.
    let mut second_body = valid_body();
    second_body["protein_g"] = json!(10.0);
    let second = post_json_with_auth(&app, "/nutrition", Some(&bearer(&token)), second_body).await;
    assert_eq!(
        second.status(),
        StatusCode::CONFLICT,
        "a second log for the same date must be 409"
    );
    assert_eq!(count(&pool).await, 1, "the conflicting POST writes nothing");
}

/// AC2: POST with no token -> 401 and writes nothing.
#[sqlx::test(migrations = "../../migrations")]
async fn post_without_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool.clone());

    let resp = post_json_with_auth(&app, "/nutrition", None, valid_body()).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(count(&pool).await, 0, "unauthorized POST writes nothing");
}

// ===========================================================================
// AC3 / SAC3: GET /nutrition — list newest-first, own-only, empty array, 401.
// ===========================================================================

/// AC3: GET with no logs -> 200 with an empty array.
#[sqlx::test(migrations = "../../migrations")]
async fn list_when_empty_returns_empty_array(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "emptylist@b.com", "8charsmin").await;

    let resp = get_with_auth(&app, "/nutrition", Some(&bearer(&token))).await;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(body_json(resp).await, json!([]), "no logs -> empty array");
}

/// AC3: GET lists the caller's logs ordered by performed_on descending, each
/// carrying its derived calories.
#[sqlx::test(migrations = "../../migrations")]
async fn list_returns_caller_logs_newest_first_with_calories(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "order@b.com", "8charsmin").await;

    // Insert the older log FIRST, then the newer — so creation order differs
    // from the expected performed_on-descending result order.
    let older = post_json_with_auth(&app, "/nutrition", Some(&bearer(&token)), valid_body()).await;
    assert_eq!(older.status(), StatusCode::CREATED);
    let newer = post_json_with_auth(
        &app,
        "/nutrition",
        Some(&bearer(&token)),
        valid_body_newer(),
    )
    .await;
    assert_eq!(newer.status(), StatusCode::CREATED);

    let resp = get_with_auth(&app, "/nutrition", Some(&bearer(&token))).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let logs = body.as_array().unwrap();

    assert_eq!(logs.len(), 2);
    assert_eq!(
        logs[0]["performed_on"].as_str().unwrap(),
        "2026-05-15",
        "newest performed_on must come first"
    );
    assert_eq!(logs[1]["performed_on"].as_str().unwrap(), "2026-05-01");
    // Each element carries its own derived calories.
    // Newer: 4·200 + 4·250 + 9·60 = 800 + 1000 + 540 = 2340.
    assert_eq!(logs[0]["calories"].as_f64().unwrap(), 2340.0);
    assert_eq!(logs[1]["calories"].as_f64().unwrap(), 2520.0);
}

/// AC3: GET with no token -> 401.
#[sqlx::test(migrations = "../../migrations")]
async fn list_without_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool);

    let resp = get_with_auth(&app, "/nutrition", None).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// AC4 / SAC4 + AC10: GET /nutrition/:id — 200 owned, 404 missing/foreign, 401.
// ===========================================================================

/// AC4: GET /:id for an owned log -> 200 with the log and its derived calories.
#[sqlx::test(migrations = "../../migrations")]
async fn get_one_owned_returns_200(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "getone@b.com", "8charsmin").await;

    let created =
        post_json_with_auth(&app, "/nutrition", Some(&bearer(&token)), valid_body()).await;
    let id = body_json(created).await["id"].as_str().unwrap().to_string();

    let resp = get_with_auth(&app, &format!("/nutrition/{id}"), Some(&bearer(&token))).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["id"].as_str().unwrap(), id);
    assert_eq!(body["calories"].as_f64().unwrap(), 2520.0);
}

/// AC4: GET /:id for a non-existent id -> 404 (ownership never leaked).
#[sqlx::test(migrations = "../../migrations")]
async fn get_one_missing_is_not_found(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "getmissing@b.com", "8charsmin").await;

    let unknown = uuid::Uuid::new_v4();
    let resp = get_with_auth(
        &app,
        &format!("/nutrition/{unknown}"),
        Some(&bearer(&token)),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await, json!({ "error": "not_found" }));
}

/// AC4/AC10: GET /:id for another user's log -> 404 (never 403).
#[sqlx::test(migrations = "../../migrations")]
async fn get_one_foreign_log_is_not_found(pool: PgPool) {
    let app = build_app(pool);
    let (_id_a, token_a) = register_and_token(&app, "ownerA@b.com", "8charsmin").await;
    let (_id_b, token_b) = register_and_token(&app, "intruderB@b.com", "8charsmin").await;

    let created =
        post_json_with_auth(&app, "/nutrition", Some(&bearer(&token_a)), valid_body()).await;
    let id = body_json(created).await["id"].as_str().unwrap().to_string();

    let resp = get_with_auth(&app, &format!("/nutrition/{id}"), Some(&bearer(&token_b))).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// AC4: GET /:id with no token -> 401.
#[sqlx::test(migrations = "../../migrations")]
async fn get_one_without_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool);
    let id = uuid::Uuid::new_v4();

    let resp = get_with_auth(&app, &format!("/nutrition/{id}"), None).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// AC5 / SAC5 + AC10: PUT /nutrition/:id — full-replace 200 + updated_at bump +
// recomputed calories, 404 missing/foreign, 409 date-collision, 400 invalid,
// 401.
// ===========================================================================

/// AC5: PUT /:id full-replaces -> 200, bumps updated_at, recomputes calories.
#[sqlx::test(migrations = "../../migrations")]
async fn put_full_replace_returns_200_bumps_updated_at_and_recomputes_calories(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, "replace@b.com", "8charsmin").await;

    let created =
        post_json_with_auth(&app, "/nutrition", Some(&bearer(&token)), valid_body()).await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let first = body_json(created).await;
    let id = first["id"].as_str().unwrap().to_string();
    let created_at: DateTime<Utc> = first["created_at"].as_str().unwrap().parse().unwrap();
    let first_updated: DateTime<Utc> = first["updated_at"].as_str().unwrap().parse().unwrap();

    // Replace with new macros and a new date.
    let replacement = json!({
        "performed_on": "2026-05-20",
        "protein_g": 100.0,
        "carbs_g": 100.0,
        "fat_g": 10.0
    });
    let resp = put_json_with_auth(
        &app,
        &format!("/nutrition/{id}"),
        Some(&bearer(&token)),
        replacement,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "full-replace must return 200"
    );
    let second = body_json(resp).await;

    assert_eq!(second["id"].as_str().unwrap(), id, "id is stable");
    assert_eq!(
        second["performed_on"].as_str().unwrap(),
        "2026-05-20",
        "date updated"
    );
    // Recomputed: 4·100 + 4·100 + 9·10 = 400 + 400 + 90 = 890.
    assert_eq!(
        second["calories"].as_f64().unwrap(),
        890.0,
        "calories must be recomputed from the new macros"
    );

    let second_created: DateTime<Utc> = second["created_at"].as_str().unwrap().parse().unwrap();
    let second_updated: DateTime<Utc> = second["updated_at"].as_str().unwrap().parse().unwrap();
    assert_eq!(second_created, created_at, "created_at preserved");
    assert!(
        second_updated > first_updated,
        "updated_at must be bumped ({second_updated} !> {first_updated})"
    );

    assert_eq!(count(&pool).await, 1, "still one row (replaced, not added)");
}

/// AC5: PUT /:id whose new performed_on collides with another of the caller's
/// logs -> 409.
#[sqlx::test(migrations = "../../migrations")]
async fn put_date_collision_is_conflict(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, "collide@b.com", "8charsmin").await;

    // Two logs on two different days.
    let day1 = post_json_with_auth(&app, "/nutrition", Some(&bearer(&token)), valid_body()).await;
    assert_eq!(day1.status(), StatusCode::CREATED);
    let day1_id = body_json(day1).await["id"].as_str().unwrap().to_string();
    let day2 = post_json_with_auth(
        &app,
        "/nutrition",
        Some(&bearer(&token)),
        valid_body_newer(),
    )
    .await;
    assert_eq!(day2.status(), StatusCode::CREATED);

    // Edit day1 to land on day2's date (2026-05-15) — collides with the unique
    // (user_id, performed_on) constraint.
    let collide = json!({
        "performed_on": "2026-05-15",
        "protein_g": 10.0,
        "carbs_g": 10.0,
        "fat_g": 10.0
    });
    let resp = put_json_with_auth(
        &app,
        &format!("/nutrition/{day1_id}"),
        Some(&bearer(&token)),
        collide,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "editing onto another existing day's date must be 409"
    );
    assert_eq!(
        count(&pool).await,
        2,
        "no row added or destroyed by the conflict"
    );
}

/// AC5: PUT /:id for a non-existent id -> 404.
#[sqlx::test(migrations = "../../migrations")]
async fn put_missing_is_not_found(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "putmissing@b.com", "8charsmin").await;

    let unknown = uuid::Uuid::new_v4();
    let resp = put_json_with_auth(
        &app,
        &format!("/nutrition/{unknown}"),
        Some(&bearer(&token)),
        valid_body(),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// AC5/AC10: PUT /:id against another user's log -> 404 and leaves it untouched.
#[sqlx::test(migrations = "../../migrations")]
async fn put_foreign_log_is_not_found_and_untouched(pool: PgPool) {
    let app = build_app(pool);
    let (_id_a, token_a) = register_and_token(&app, "putownerA@b.com", "8charsmin").await;
    let (_id_b, token_b) = register_and_token(&app, "putintruderB@b.com", "8charsmin").await;

    let created =
        post_json_with_auth(&app, "/nutrition", Some(&bearer(&token_a)), valid_body()).await;
    let id = body_json(created).await["id"].as_str().unwrap().to_string();

    let resp = put_json_with_auth(
        &app,
        &format!("/nutrition/{id}"),
        Some(&bearer(&token_b)),
        valid_body_newer(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // A's log is unchanged.
    let get_a = get_with_auth(&app, &format!("/nutrition/{id}"), Some(&bearer(&token_a))).await;
    assert_eq!(get_a.status(), StatusCode::OK);
    let body = body_json(get_a).await;
    assert_eq!(
        body["performed_on"].as_str().unwrap(),
        "2026-05-01",
        "A's log must be untouched"
    );
    assert_eq!(body["calories"].as_f64().unwrap(), 2520.0);
}

/// AC5/AC8: PUT /:id with an invalid body -> 400 and writes nothing.
#[sqlx::test(migrations = "../../migrations")]
async fn put_invalid_body_is_rejected_and_writes_nothing(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, "putinvalid@b.com", "8charsmin").await;

    let created =
        post_json_with_auth(&app, "/nutrition", Some(&bearer(&token)), valid_body()).await;
    let id = body_json(created).await["id"].as_str().unwrap().to_string();

    // protein_g out of range -> 400 field "protein_g".
    let mut bad = valid_body();
    bad["protein_g"] = json!(-1.0);
    let resp = put_json_with_auth(
        &app,
        &format!("/nutrition/{id}"),
        Some(&bearer(&token)),
        bad,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "validation", "field": "protein_g" })
    );

    // The original row is intact (the rejected PUT replaced nothing).
    let get_one = get_with_auth(&app, &format!("/nutrition/{id}"), Some(&bearer(&token))).await;
    assert_eq!(
        body_json(get_one).await["protein_g"].as_f64().unwrap(),
        150.0
    );
}

/// AC5: PUT /:id with no token -> 401.
#[sqlx::test(migrations = "../../migrations")]
async fn put_without_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool);
    let id = uuid::Uuid::new_v4();

    let resp = put_json_with_auth(&app, &format!("/nutrition/{id}"), None, valid_body()).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// AC6 / SAC6 + AC10: DELETE /nutrition/:id — 204 then 404, foreign 404, 401.
// ===========================================================================

/// AC6: DELETE /:id of an owned log -> 204; a second DELETE -> 404.
#[sqlx::test(migrations = "../../migrations")]
async fn delete_owned_then_second_delete_is_not_found(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, "delete@b.com", "8charsmin").await;

    let created =
        post_json_with_auth(&app, "/nutrition", Some(&bearer(&token)), valid_body()).await;
    let id = body_json(created).await["id"].as_str().unwrap().to_string();

    let first = delete_with_auth(&app, &format!("/nutrition/{id}"), Some(&bearer(&token))).await;
    assert_eq!(first.status(), StatusCode::NO_CONTENT);
    assert_eq!(count(&pool).await, 0);

    let second = delete_with_auth(&app, &format!("/nutrition/{id}"), Some(&bearer(&token))).await;
    assert_eq!(second.status(), StatusCode::NOT_FOUND);
}

/// AC6/AC10: DELETE /:id of another user's log -> 404 and leaves it intact.
#[sqlx::test(migrations = "../../migrations")]
async fn delete_foreign_log_is_not_found_and_untouched(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_id_a, token_a) = register_and_token(&app, "delownerA@b.com", "8charsmin").await;
    let (_id_b, token_b) = register_and_token(&app, "delintruderB@b.com", "8charsmin").await;

    let created =
        post_json_with_auth(&app, "/nutrition", Some(&bearer(&token_a)), valid_body()).await;
    let id = body_json(created).await["id"].as_str().unwrap().to_string();

    let resp = delete_with_auth(&app, &format!("/nutrition/{id}"), Some(&bearer(&token_b))).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(count(&pool).await, 1, "B's delete must not touch A's log");
}

/// AC6: DELETE /:id with no token -> 401.
#[sqlx::test(migrations = "../../migrations")]
async fn delete_without_token_is_unauthorized(pool: PgPool) {
    let app = build_app(pool);
    let id = uuid::Uuid::new_v4();

    let resp = delete_with_auth(&app, &format!("/nutrition/{id}"), None).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// AC8 / SAC8: every validation branch -> 400, writes nothing. The field-label
// asymmetry (SPEC-0005 §6): SEMANTIC failures (present-but-invalid) report the
// leaf field; STRUCTURAL failures (missing / non-numeric) report "body".
// ===========================================================================

/// POST `mutate` as a fresh user; assert 400, optional `{error,field}` body, and
/// that nothing was written. `field == Some(f)` pins a SEMANTIC failure to leaf
/// `f`; `field == Some("body")` pins a STRUCTURAL failure routed to `"body"`.
async fn assert_rejected(pool: &PgPool, email: &str, mutate: Value, field: &str) {
    let app = build_app(pool.clone());
    let (_id, token) = register_and_token(&app, email, "8charsmin").await;

    let resp = post_json_with_auth(&app, "/nutrition", Some(&bearer(&token)), mutate).await;

    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "invalid body for {email} must be 400"
    );
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "validation", "field": field }),
        "validation error for {email} must name field `{field}`"
    );
    assert_eq!(
        count(pool).await,
        0,
        "rejected POST for {email} writes nothing"
    );
}

// --- Semantic failures: present-but-invalid value reports the leaf field. ---

/// AC8: performed_on in the future -> 400 field "performed_on".
#[sqlx::test(migrations = "../../migrations")]
async fn post_future_performed_on_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["performed_on"] = json!("2999-01-01");
    assert_rejected(&pool, "future@b.com", body, "performed_on").await;
}

/// AC8: protein_g < 0 -> 400 field "protein_g".
#[sqlx::test(migrations = "../../migrations")]
async fn post_negative_protein_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["protein_g"] = json!(-1.0);
    assert_rejected(&pool, "negprotein@b.com", body, "protein_g").await;
}

/// AC8: carbs_g > 2000 -> 400 field "carbs_g".
#[sqlx::test(migrations = "../../migrations")]
async fn post_carbs_above_maximum_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["carbs_g"] = json!(2000.1);
    assert_rejected(&pool, "bigcarbs@b.com", body, "carbs_g").await;
}

/// AC8: fat_g < 0 -> 400 field "fat_g".
#[sqlx::test(migrations = "../../migrations")]
async fn post_negative_fat_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["fat_g"] = json!(-5.0);
    assert_rejected(&pool, "negfat@b.com", body, "fat_g").await;
}

// --- Structural failures: missing / non-numeric report "body". ---

/// AC8: a missing required `performed_on` -> 400 field "body".
#[sqlx::test(migrations = "../../migrations")]
async fn post_missing_performed_on_is_rejected(pool: PgPool) {
    let body = json!({ "protein_g": 150.0, "carbs_g": 300.0, "fat_g": 80.0 });
    assert_rejected(&pool, "noperformed@b.com", body, "body").await;
}

/// AC8: a missing required macro (`protein_g` absent) -> 400 field "body".
#[sqlx::test(migrations = "../../migrations")]
async fn post_missing_protein_is_rejected(pool: PgPool) {
    let body = json!({ "performed_on": "2026-05-01", "carbs_g": 300.0, "fat_g": 80.0 });
    assert_rejected(&pool, "noprotein@b.com", body, "body").await;
}

/// AC8: a present-but-non-numeric macro (a string) -> 400 field "body".
/// Pins the structural-vs-semantic asymmetry for the SAME field: `protein_g:
/// -1` is semantic ("protein_g") whereas `protein_g: "x"` is structural ("body").
#[sqlx::test(migrations = "../../migrations")]
async fn post_non_numeric_protein_is_rejected(pool: PgPool) {
    let mut body = valid_body();
    body["protein_g"] = json!("x");
    assert_rejected(&pool, "strprotein@b.com", body, "body").await;
}

// ===========================================================================
// AC10 / SAC10: cross-user isolation — A's list never returns B's logs.
// ===========================================================================

/// AC10: two distinct users each see only their own logs in GET /nutrition.
#[sqlx::test(migrations = "../../migrations")]
async fn list_is_isolated_per_user(pool: PgPool) {
    let app = build_app(pool);
    let (id_a, token_a) = register_and_token(&app, "isoA@b.com", "8charsmin").await;
    let (id_b, token_b) = register_and_token(&app, "isoB@b.com", "8charsmin").await;
    assert_ne!(id_a, id_b);

    // B logs two days; A logs one.
    post_json_with_auth(&app, "/nutrition", Some(&bearer(&token_b)), valid_body()).await;
    post_json_with_auth(
        &app,
        "/nutrition",
        Some(&bearer(&token_b)),
        valid_body_newer(),
    )
    .await;
    post_json_with_auth(&app, "/nutrition", Some(&bearer(&token_a)), valid_body()).await;

    let list_a = get_with_auth(&app, "/nutrition", Some(&bearer(&token_a))).await;
    let a_arr = body_json(list_a).await;
    let a_arr = a_arr.as_array().unwrap();
    assert_eq!(a_arr.len(), 1, "A must see only its own log");
    assert_eq!(a_arr[0]["user_id"].as_str().unwrap(), id_a);

    let list_b = get_with_auth(&app, "/nutrition", Some(&bearer(&token_b))).await;
    let b_arr = body_json(list_b).await;
    let b_arr = b_arr.as_array().unwrap();
    assert_eq!(b_arr.len(), 2, "B must see only its own two logs");
    for log in b_arr {
        assert_eq!(
            log["user_id"].as_str().unwrap(),
            id_b,
            "no cross-user log leaks"
        );
    }
}
