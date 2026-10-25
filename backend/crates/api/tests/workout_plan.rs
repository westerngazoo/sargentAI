//! R-0061 / SPEC-0061 §2.4 — the plan link on `POST` and `PUT /workouts`
//! (slice 2c, AC6).
//!
//! A session may name the plan day it was seeded with (`user_program_id` +
//! `plan_day_index`). The checks run in the spec's order so an error never
//! reveals another user's data (N10): both-or-neither → ownership → currency
//! → cycle known → index and date valid.
//!
//! `heavy-duty-mass` is the pinned archetype: its cycle is [shoulders/arms,
//! back, rest, chest/arms, legs, rest], so index 0 trains and index 2 rests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::doc_markdown)]

mod common;

use axum::http::StatusCode;
use chrono::{Days, NaiveDate, Utc};
use common::{
    body_json, build_app, post_json_with_auth, put_json_with_auth, register_and_token,
    seed_program, workout_body,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const ARCHETYPE: &str = "heavy-duty-mass";
const TRAINING_0: i64 = 0;
const REST_2: i64 = 2;

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

fn yesterday() -> NaiveDate {
    today().checked_sub_days(Days::new(1)).unwrap()
}

async fn post_workout(
    app: &axum::Router,
    token: &str,
    body: Value,
) -> axum::response::Response<axum::body::Body> {
    post_json_with_auth(app, "/workouts", Some(&bearer(token)), body).await
}

/// The stored plan pair of a session.
async fn stored_plan(pool: &PgPool, session_id: &str) -> (Option<Uuid>, Option<i16>) {
    sqlx::query_as("SELECT user_program_id, plan_day_index FROM workout_sessions WHERE id = $1")
        .bind(Uuid::parse_str(session_id).unwrap())
        .fetch_one(pool)
        .await
        .unwrap()
}

// ---------------------------------------------------------------------------
// POST /workouts
// ---------------------------------------------------------------------------

/// A plan session persists both columns and still answers 201 with the
/// session the R-0004 clients expect.
#[sqlx::test(migrations = "../../migrations")]
async fn create_with_plan_fields_persists_them(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "plan-ok@b.com", "8charsmin").await;
    let program = seed_program(&pool, &user_id, ARCHETYPE, today(), true).await;

    let resp = post_workout(
        &app,
        &token,
        workout_body(today(), Some((program, TRAINING_0))),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert_eq!(body["performed_on"], today().to_string());

    let session_id = body["id"].as_str().unwrap();
    assert_eq!(
        stored_plan(&pool, session_id).await,
        (Some(program), Some(0)),
        "both plan columns are stored"
    );
}

/// A free workout sends neither field and keeps working: nothing is stored.
#[sqlx::test(migrations = "../../migrations")]
async fn create_without_plan_fields_stores_a_free_workout(pool: PgPool) {
    let app = build_app(pool.clone());
    let (_, token) = register_and_token(&app, "plan-free@b.com", "8charsmin").await;

    let resp = post_workout(&app, &token, workout_body(today(), None)).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let session_id = body_json(resp).await["id"].as_str().unwrap().to_string();
    assert_eq!(stored_plan(&pool, &session_id).await, (None, None));
}

/// One plan field without the other → 422, before anything is looked up.
#[sqlx::test(migrations = "../../migrations")]
async fn create_with_one_plan_field_is_422(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "plan-half@b.com", "8charsmin").await;
    let program = seed_program(&pool, &user_id, ARCHETYPE, today(), true).await;

    let mut only_program = workout_body(today(), None);
    only_program["user_program_id"] = Value::String(program.to_string());
    let mut only_index = workout_body(today(), None);
    only_index["plan_day_index"] = Value::from(TRAINING_0);

    for body in [only_program, only_index] {
        let resp = post_workout(&app, &token, body).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            body_json(resp).await,
            json!({ "error": "unprocessable", "reason": "plan_fields_both_or_neither" })
        );
    }
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workout_sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0, "nothing is written");
}

/// N10: another user's program → 404, even when the index is out of range,
/// so the error reveals nothing about that program's cycle.
#[sqlx::test(migrations = "../../migrations")]
async fn create_with_a_foreign_program_is_404_even_with_an_invalid_index(pool: PgPool) {
    let app = build_app(pool.clone());
    let (a_id, _) = register_and_token(&app, "plan-a@b.com", "8charsmin").await;
    let (_, b_token) = register_and_token(&app, "plan-b@b.com", "8charsmin").await;
    let a_program = seed_program(&pool, &a_id, ARCHETYPE, today(), true).await;

    let resp = post_workout(
        &app,
        &b_token,
        workout_body(today(), Some((a_program, 999))),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await, json!({ "error": "not_found" }));
}

/// The caller's own, superseded program → 409 `program_not_current`.
#[sqlx::test(migrations = "../../migrations")]
async fn create_with_an_own_non_current_program_is_409(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "plan-old@b.com", "8charsmin").await;
    let old = seed_program(&pool, &user_id, ARCHETYPE, today(), false).await;
    seed_program(&pool, &user_id, ARCHETYPE, today(), true).await;

    let resp = post_workout(&app, &token, workout_body(today(), Some((old, TRAINING_0)))).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "conflict", "reason": "program_not_current" })
    );
}

/// The current program's archetype is unknown to the library → 422
/// `plan_unavailable`, so the client saves the session as free.
#[sqlx::test(migrations = "../../migrations")]
async fn create_with_an_unknown_archetype_is_422_plan_unavailable(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "plan-unknown@b.com", "8charsmin").await;
    let program = seed_program(&pool, &user_id, "retired-slug", today(), true).await;

    let resp = post_workout(
        &app,
        &token,
        workout_body(today(), Some((program, TRAINING_0))),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "unprocessable", "reason": "plan_unavailable" })
    );
}

/// `plan_day_index` must name a training day: a rest day or an index past the
/// cycle → 422.
#[sqlx::test(migrations = "../../migrations")]
async fn create_with_a_rest_or_out_of_range_index_is_422(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "plan-rest@b.com", "8charsmin").await;
    let program = seed_program(&pool, &user_id, ARCHETYPE, today(), true).await;

    for index in [REST_2, 6, 999] {
        let resp = post_workout(&app, &token, workout_body(today(), Some((program, index)))).await;
        assert_eq!(
            resp.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "index {index}"
        );
        assert_eq!(
            body_json(resp).await,
            json!({ "error": "unprocessable", "reason": "plan_day_not_training" })
        );
    }
}

/// A plan session dated before the program started → 422.
#[sqlx::test(migrations = "../../migrations")]
async fn create_before_starts_on_is_422(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "plan-early@b.com", "8charsmin").await;
    let program = seed_program(&pool, &user_id, ARCHETYPE, today(), true).await;

    let resp = post_workout(
        &app,
        &token,
        workout_body(yesterday(), Some((program, TRAINING_0))),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "unprocessable", "reason": "performed_on_before_starts_on" })
    );
}

// ---------------------------------------------------------------------------
// PUT /workouts/{id}
// ---------------------------------------------------------------------------

/// The body carries no plan fields, so a replace keeps the stored pair, and
/// re-checks `performed_on ≥ starts_on` while they are set.
#[sqlx::test(migrations = "../../migrations")]
async fn replace_keeps_the_plan_fields_and_rechecks_starts_on(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "plan-put@b.com", "8charsmin").await;
    let program = seed_program(&pool, &user_id, ARCHETYPE, today(), true).await;
    let created = post_workout(
        &app,
        &token,
        workout_body(today(), Some((program, TRAINING_0))),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let session_id = body_json(created).await["id"].as_str().unwrap().to_string();
    let path = format!("/workouts/{session_id}");

    let resp = put_json_with_auth(
        &app,
        &path,
        Some(&bearer(&token)),
        workout_body(today(), None),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        stored_plan(&pool, &session_id).await,
        (Some(program), Some(0)),
        "a replace keeps the plan pair"
    );

    let resp = put_json_with_auth(
        &app,
        &path,
        Some(&bearer(&token)),
        workout_body(yesterday(), None),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "unprocessable", "reason": "performed_on_before_starts_on" })
    );
    let kept: NaiveDate =
        sqlx::query_scalar("SELECT performed_on FROM workout_sessions WHERE id = $1")
            .bind(Uuid::parse_str(&session_id).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(kept, today(), "a rejected replace writes nothing");
}

/// A free session has no plan pair, so a replace to any valid date passes.
#[sqlx::test(migrations = "../../migrations")]
async fn replace_of_a_free_session_has_no_starts_on_check(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "plan-put-free@b.com", "8charsmin").await;
    seed_program(&pool, &user_id, ARCHETYPE, today(), true).await;
    let created = post_workout(&app, &token, workout_body(today(), None)).await;
    let session_id = body_json(created).await["id"].as_str().unwrap().to_string();

    let resp = put_json_with_auth(
        &app,
        &format!("/workouts/{session_id}"),
        Some(&bearer(&token)),
        workout_body(yesterday(), None),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
}
