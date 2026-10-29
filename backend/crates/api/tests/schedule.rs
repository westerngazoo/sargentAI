//! R-0061 / SPEC-0061 §2.3–§2.4 — `GET /programs/me/schedule` and migration
//! `00012_plan_projection.sql` (slice 2c, AC6).
//!
//! The projection itself is unit-tested in `core::plan`; this suite checks the
//! plumbing: the caller's sessions reach the projection as `DayLog`s, the plan
//! fields of the current program count and nothing else does, the validation
//! of `today` and the range, and owner scoping.
//!
//! Dates are relative to the server's UTC date because `today` must lie within
//! a day of it (§2.4). `heavy-duty-mass` is the pinned archetype: its cycle is
//! [shoulders/arms, back, rest, chest/arms, legs, rest].

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::doc_markdown)]

mod common;

use std::borrow::Cow;

use axum::http::StatusCode;
use chrono::{Days, NaiveDate, Utc};
use common::{
    body_json, build_app, get_with_auth, post_json_with_auth, register_and_token, seed_program,
    workout_body,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const ARCHETYPE: &str = "heavy-duty-mass";
const TRAINING_0: i64 = 0;

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

/// `today` shifted by `n` days (negative for the past).
fn day(n: i64) -> NaiveDate {
    let t = today();
    if n >= 0 {
        t.checked_add_days(Days::new(n.unsigned_abs())).unwrap()
    } else {
        t.checked_sub_days(Days::new(n.unsigned_abs())).unwrap()
    }
}

async fn schedule(
    app: &axum::Router,
    token: &str,
    from: NaiveDate,
    to: NaiveDate,
    today: NaiveDate,
) -> axum::response::Response<axum::body::Body> {
    get_with_auth(
        app,
        &format!("/programs/me/schedule?from={from}&to={to}&today={today}"),
        Some(&bearer(token)),
    )
    .await
}

/// `POST /workouts`, asserting 201, and return the new session id.
async fn log_workout(
    app: &axum::Router,
    token: &str,
    performed_on: NaiveDate,
    plan: Option<(Uuid, i64)>,
) -> String {
    let resp = post_json_with_auth(
        app,
        "/workouts",
        Some(&bearer(token)),
        workout_body(performed_on, plan),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "seed workout expected 201"
    );
    body_json(resp).await["id"].as_str().unwrap().to_string()
}

fn day_entry(days: &Value, date: NaiveDate) -> &Value {
    days.as_array()
        .unwrap()
        .iter()
        .find(|d| d["date"] == date.to_string())
        .unwrap_or_else(|| panic!("no entry for {date}"))
}

fn state_of(days: &Value, date: NaiveDate) -> Value {
    let mut entry = day_entry(days, date).clone();
    entry.as_object_mut().unwrap().remove("date");
    entry
}

// ---------------------------------------------------------------------------
// GET /programs/me/schedule
// ---------------------------------------------------------------------------

/// Happy path: a plan session completes its day, a free workout leaves the next
/// training day pending, an unlogged past day shifts it, and the future is the
/// projected cycle. `session_ids` names every session in the range.
#[sqlx::test(migrations = "../../migrations")]
async fn schedule_projects_plan_and_free_sessions(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "sched-ok@b.com", "8charsmin").await;
    let program_id = seed_program(&pool, &user_id, ARCHETYPE, day(-3), true).await;
    let plan_session = log_workout(&app, &token, day(-3), Some((program_id, TRAINING_0))).await;
    let free_session = log_workout(&app, &token, day(-2), None).await;

    let resp = schedule(&app, &token, day(-4), day(1), today()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    assert_eq!(body["program_id"], program_id.to_string());
    assert_eq!(body["starts_on"], day(-3).to_string());
    let cycle = body["cycle"].as_array().unwrap();
    assert_eq!(cycle.len(), 6, "the pinned archetype has a six-day cycle");
    assert_eq!(cycle[0]["kind"], "training");
    assert_eq!(cycle[2], json!({ "kind": "rest" }));

    let days = &body["days"];
    assert_eq!(
        days.as_array().unwrap().len(),
        6,
        "one entry per day of the range"
    );
    assert_eq!(
        state_of(days, day(-4)),
        json!({ "state": "nothing", "pending": null }),
        "before starts_on"
    );
    assert_eq!(
        state_of(days, day(-3)),
        json!({ "state": "done", "completed": [0], "pending": null })
    );
    assert_eq!(
        state_of(days, day(-2)),
        json!({ "state": "done", "completed": [], "pending": 1 }),
        "a free workout leaves the training day pending"
    );
    assert_eq!(
        state_of(days, day(-1)),
        json!({ "state": "nothing", "pending": 1 }),
        "an unlogged past day shifts the training day"
    );
    assert_eq!(
        state_of(days, day(0)),
        json!({ "state": "planned", "cycle_index": 1 })
    );
    assert_eq!(
        state_of(days, day(1)),
        json!({ "state": "rest", "cycle_index": 2 })
    );

    assert_eq!(
        body["session_ids"],
        json!({
            day(-3).to_string(): [plan_session],
            day(-2).to_string(): [free_session],
        })
    );
}

/// SPEC-0059 §2.5: each training day of the cycle carries `physics`, one
/// entry per exercise, decided by the server. `powerbuilder-leverage` opens
/// with legs: Back Squat (modelled), then Front Squat, Leg Press, Leg Curl
/// and Standing Calf Raise (not); rest days carry no `physics` at all.
#[sqlx::test(migrations = "../../migrations")]
async fn schedule_cycle_marks_each_exercise_with_its_physics(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "sched-phys@b.com", "8charsmin").await;
    seed_program(&pool, &user_id, "powerbuilder-leverage", day(0), true).await;

    let resp = schedule(&app, &token, day(0), day(0), today()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    let cycle = body["cycle"].as_array().unwrap();
    assert_eq!(cycle[0]["exercises"][0], "Back Squat");
    assert_eq!(
        cycle[0]["physics"],
        json!(["squat", null, null, null, null])
    );
    for entry in cycle {
        match entry["kind"].as_str().unwrap() {
            "training" => assert_eq!(
                entry["physics"].as_array().unwrap().len(),
                entry["exercises"].as_array().unwrap().len(),
                "one physics entry per exercise: {entry}"
            ),
            _ => assert_eq!(entry, &json!({ "kind": "rest" })),
        }
    }
}

/// No current program: `program_id` is null, the cycle is empty and the days
/// come from the logs alone.
#[sqlx::test(migrations = "../../migrations")]
async fn schedule_without_a_program_shows_logs_only(pool: PgPool) {
    let app = build_app(pool);
    let (_, token) = register_and_token(&app, "sched-none@b.com", "8charsmin").await;
    let session = log_workout(&app, &token, day(-1), None).await;

    let resp = schedule(&app, &token, day(-2), day(0), today()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    assert_eq!(body["program_id"], Value::Null);
    assert_eq!(body["starts_on"], Value::Null);
    assert_eq!(body["cycle"], json!([]));
    assert_eq!(
        body["days"],
        json!([
            { "date": day(-2).to_string(), "state": "nothing", "pending": null },
            { "date": day(-1).to_string(), "state": "done", "completed": [], "pending": null },
            { "date": day(0).to_string(), "state": "nothing", "pending": null },
        ])
    );
    assert_eq!(
        body["session_ids"],
        json!({ day(-1).to_string(): [session] })
    );
}

/// A current program whose `archetype_id` the library no longer knows reads as
/// no program (§2.1): never a 500.
#[sqlx::test(migrations = "../../migrations")]
async fn schedule_with_an_unknown_archetype_shows_logs_only(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "sched-unknown@b.com", "8charsmin").await;
    seed_program(&pool, &user_id, "retired-slug", day(-1), true).await;
    log_workout(&app, &token, day(-1), None).await;

    let resp = schedule(&app, &token, day(-1), day(0), today()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    assert_eq!(body["program_id"], Value::Null);
    assert_eq!(body["cycle"], json!([]));
    assert_eq!(
        state_of(&body["days"], day(-1)),
        json!({ "state": "done", "completed": [], "pending": null })
    );
    assert_eq!(
        state_of(&body["days"], day(0)),
        json!({ "state": "nothing", "pending": null })
    );
}

/// `from > to` → 422.
#[sqlx::test(migrations = "../../migrations")]
async fn schedule_rejects_an_inverted_range(pool: PgPool) {
    let app = build_app(pool);
    let (_, token) = register_and_token(&app, "sched-inv@b.com", "8charsmin").await;

    let resp = schedule(&app, &token, day(1), day(0), today()).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "unprocessable", "reason": "range_inverted" })
    );
}

/// 62 days is the most a range may span, inclusive; 63 → 422.
#[sqlx::test(migrations = "../../migrations")]
async fn schedule_rejects_a_range_longer_than_62_days(pool: PgPool) {
    let app = build_app(pool);
    let (_, token) = register_and_token(&app, "sched-long@b.com", "8charsmin").await;

    let ok = schedule(&app, &token, day(0), day(61), today()).await;
    assert_eq!(ok.status(), StatusCode::OK, "62 days inclusive is allowed");

    let resp = schedule(&app, &token, day(0), day(62), today()).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "unprocessable", "reason": "range_too_long" })
    );
}

/// `today` is the client's local date: a day either side of the server's UTC
/// date is accepted, two days is not.
#[sqlx::test(migrations = "../../migrations")]
async fn schedule_bounds_today_to_a_day_of_the_server_date(pool: PgPool) {
    let app = build_app(pool);
    let (_, token) = register_and_token(&app, "sched-today@b.com", "8charsmin").await;

    for offset in [-1, 0, 1] {
        let resp = schedule(&app, &token, day(0), day(0), day(offset)).await;
        assert_eq!(resp.status(), StatusCode::OK, "today at {offset:+} days");
    }
    for offset in [-2, 2] {
        let resp = schedule(&app, &token, day(0), day(0), day(offset)).await;
        assert_eq!(
            resp.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "today at {offset:+} days"
        );
        assert_eq!(
            body_json(resp).await,
            json!({ "error": "unprocessable", "reason": "today_out_of_bounds" })
        );
    }
}

/// The client's `today` drives the projection, not the server's clock: with
/// `today` one day ahead, the server's date is already a past day with no log
/// (`nothing`, the pending index shifted) and the client's day is the one
/// `planned`.
#[sqlx::test(migrations = "../../migrations")]
async fn schedule_projects_from_the_clients_today(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "sched-client@b.com", "8charsmin").await;
    seed_program(&pool, &user_id, ARCHETYPE, day(0), true).await;

    let resp = schedule(&app, &token, day(0), day(1), day(1)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let days = body_json(resp).await["days"].clone();

    assert_eq!(
        state_of(&days, day(0)),
        json!({ "state": "nothing", "pending": 0 }),
        "the server's date is yesterday to the client"
    );
    assert_eq!(
        state_of(&days, day(1)),
        json!({ "state": "planned", "cycle_index": 0 }),
        "the client's today holds the shifted first training day"
    );
}

/// N9: a range ending more than 366 days after today is refused before any
/// walk happens.
#[sqlx::test(migrations = "../../migrations")]
async fn schedule_rejects_a_range_far_ahead(pool: PgPool) {
    let app = build_app(pool);
    let (_, token) = register_and_token(&app, "sched-far@b.com", "8charsmin").await;

    let resp = get_with_auth(
        &app,
        &format!(
            "/programs/me/schedule?from=9999-01-01&to=9999-01-31&today={}",
            today()
        ),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body_json(resp).await,
        json!({ "error": "unprocessable", "reason": "range_too_far_ahead" })
    );
}

/// Owner scoping (finding 12): user A's current program and A's plan session
/// never appear in B's schedule.
#[sqlx::test(migrations = "../../migrations")]
async fn schedule_never_shows_another_users_program_or_sessions(pool: PgPool) {
    let app = build_app(pool.clone());
    let (a_id, a_token) = register_and_token(&app, "sched-a@b.com", "8charsmin").await;
    let (_, b_token) = register_and_token(&app, "sched-b@b.com", "8charsmin").await;
    let a_program = seed_program(&pool, &a_id, ARCHETYPE, day(-1), true).await;
    let a_session = log_workout(&app, &a_token, day(-1), Some((a_program, TRAINING_0))).await;

    let resp = schedule(&app, &b_token, day(-1), day(0), today()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    assert_eq!(body["program_id"], Value::Null);
    assert_eq!(body["cycle"], json!([]));
    assert_eq!(body["session_ids"], json!({}));
    for date in [day(-1), day(0)] {
        assert_eq!(
            state_of(&body["days"], date),
            json!({ "state": "nothing", "pending": null })
        );
    }
    let serialized = body.to_string();
    assert!(!serialized.contains(&a_program.to_string()));
    assert!(!serialized.contains(&a_session));
}

/// No token → 401.
#[sqlx::test(migrations = "../../migrations")]
async fn schedule_unauthenticated_is_401(pool: PgPool) {
    let app = build_app(pool);
    let resp = get_with_auth(
        &app,
        &format!(
            "/programs/me/schedule?from={}&to={}&today={}",
            day(0),
            day(0),
            today()
        ),
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// Migration 00012
// ---------------------------------------------------------------------------

/// The backfill: a program chosen before the migration gets `starts_on` from
/// its `chosen_at` (UTC date). The default: a row inserted without naming the
/// column gets the server's UTC date, so a rolled-back image still inserts.
#[sqlx::test(migrations = false)]
async fn migration_backfills_starts_on_from_chosen_at_and_defaults_it(pool: PgPool) {
    let mut migrator = sqlx::migrate!("../../migrations");
    let all = migrator.migrations.clone();
    migrator.migrations = Cow::Owned(all.iter().filter(|m| m.version < 12).cloned().collect());
    migrator.run(&pool).await.expect("migrations up to 00011");

    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (email, password_hash) VALUES ('m12@b.com', 'x') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let old: Uuid = sqlx::query_scalar(
        "INSERT INTO user_programs (user_id, archetype_id, program, diet, chosen_at) \
         VALUES ($1, 'heavy-duty-mass', '{}'::jsonb, '{}'::jsonb, '2026-01-15 23:30:00+00') \
         RETURNING id",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    migrator.migrations = all;
    migrator.run(&pool).await.expect("migration 00012");

    let backfilled: NaiveDate =
        sqlx::query_scalar("SELECT starts_on FROM user_programs WHERE id = $1")
            .bind(old)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(backfilled, NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());

    let defaulted: NaiveDate = sqlx::query_scalar(
        "INSERT INTO user_programs (user_id, archetype_id, program, diet) \
         VALUES ($1, 'heavy-duty-mass', '{}'::jsonb, '{}'::jsonb) RETURNING starts_on",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(defaulted, today());

    let columns: Vec<(String, String)> = sqlx::query_as(
        "SELECT column_name, is_nullable FROM information_schema.columns \
         WHERE table_name = 'workout_sessions' \
           AND column_name IN ('user_program_id', 'plan_day_index') \
         ORDER BY column_name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        columns,
        vec![
            ("plan_day_index".to_string(), "YES".to_string()),
            ("user_program_id".to_string(), "YES".to_string()),
        ],
        "both plan columns exist and are nullable (no CHECK pairs them)"
    );
}
