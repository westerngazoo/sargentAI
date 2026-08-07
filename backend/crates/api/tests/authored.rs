//! R-0041 authored-program integration suite — SPEC-0041 §7 items 1–13.
//!
//! Every test is `#[sqlx::test(migrations = "../../migrations")]`: sqlx
//! provisions a fresh per-test database, applies the migrations (including
//! `00009_authored_programs.sql`), and hands a connected `PgPool` to the test.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Loads are exact multiples of the plate increment, so `==` is correct here.
#![allow(clippy::float_cmp)]
// Test doc comments quote JSON literals and token names as prose, not code.
#![allow(clippy::doc_markdown)]

mod common;

use axum::http::StatusCode;
use chrono::Utc;
use common::{body_json, build_app, get_with_auth, post_json_with_auth, register_and_token};
use fitai_core::AuthoredProgram;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

/// An exercise whose three classes are simple single work-set lines.
fn ex(name: &str) -> Value {
    json!({
        "name": name,
        "low":    {"warmup_sets": 2, "work": [{"sets": 3, "reps": 8, "load_pct": 0.70}]},
        "medium": {"warmup_sets": 2, "work": [{"sets": 4, "reps": 5, "load_pct": 0.80}]},
        "high":   {"warmup_sets": 3, "work": [{"sets": 5, "reps": 3, "load_pct": 0.88}]}
    })
}

/// A valid three-day program: Squat High on day 1, rest on day 2, Bench Medium
/// on day 3. Curl is authored but unscheduled (still validated).
fn program() -> Value {
    json!({
        "name": "Push Pull Legs",
        "core": [ex("Squat"), ex("Bench Press")],
        "accessories": [ex("Curl")],
        "schedule": {"days": [
            [{"exercise": "Squat", "class": "high"}],
            [],
            [{"exercise": "Bench Press", "class": "medium"}]
        ]}
    })
}

fn create_body(program: &Value) -> Value {
    json!({ "program": program })
}

/// Create a program for `token`, asserting 201, and return its id.
async fn create_program(app: &axum::Router, token: &str, program: &Value) -> String {
    let res = post_json_with_auth(
        app,
        "/authored-programs",
        Some(&bearer(token)),
        create_body(program),
    )
    .await;
    assert_eq!(
        res.status(),
        StatusCode::CREATED,
        "seed create expected 201"
    );
    body_json(res).await["id"]
        .as_str()
        .expect("create response must carry a string id")
        .to_string()
}

/// Log one `lift` session today (inside the 8-week e1RM window) for `token`.
async fn log_lift(app: &axum::Router, token: &str, lift: &str, rep_count: u32, kg: f64) {
    let res = post_json_with_auth(
        app,
        "/workouts",
        Some(&bearer(token)),
        json!({
            "performed_on": Utc::now().date_naive().to_string(),
            "exercises": [{
                "name": lift,
                "muscle_group": "legs",
                "sets": [{ "reps": rep_count, "weight_kg": kg }]
            }]
        }),
    )
    .await;
    assert_eq!(
        res.status(),
        StatusCode::CREATED,
        "seed workout expected 201"
    );
}

/// The `target_load_kg` of the first work set of day `day`'s first entry.
fn first_load(cycle: &Value, day: usize) -> &Value {
    &cycle["days"][day]["entries"][0]["work_sets"][0]["target_load_kg"]
}

// ---------------------------------------------------------------------------
// §7.1 — auth is required on every route (AC8)
// ---------------------------------------------------------------------------

/// Includes a **malformed** id with no token: `AuthenticatedUser` runs before
/// `Path`, so the answer is 401, not axum's 400 for an unparseable UUID.
#[sqlx::test(migrations = "../../migrations")]
async fn every_route_requires_a_jwt(pool: PgPool) {
    let app = build_app(pool);
    let id = Uuid::new_v4();

    let res = post_json_with_auth(&app, "/authored-programs", None, create_body(&program())).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "POST without a JWT");

    for path in [
        "/authored-programs".to_string(),
        format!("/authored-programs/{id}"),
        format!("/authored-programs/{id}/materialized"),
        "/authored-programs/not-a-uuid".to_string(),
        "/authored-programs/not-a-uuid/materialized".to_string(),
    ] {
        let res = get_with_auth(&app, &path, None).await;
        assert_eq!(
            res.status(),
            StatusCode::UNAUTHORIZED,
            "GET {path} without a JWT must be 401, never 400"
        );
    }
}

// ---------------------------------------------------------------------------
// §7.2 — create happy path (AC2)
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn create_stores_and_returns_the_program(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;

    let res = post_json_with_auth(
        &app,
        "/authored-programs",
        Some(&bearer(&token)),
        create_body(&program()),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    let body = body_json(res).await;
    assert!(Uuid::parse_str(body["id"].as_str().unwrap()).is_ok());
    assert_eq!(body["program"]["name"], "Push Pull Legs");
    assert!(!body["created_at"].is_null());
    assert!(!body["updated_at"].is_null());
    // The denormalized column is storage detail, not payload (SPEC-0041 §2.7).
    assert!(body.get("name").is_none(), "no top-level `name` field");
}

// ---------------------------------------------------------------------------
// §7.3 — table-driven rejection sweep over all ten program-level tokens (AC3)
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn every_program_level_rejection_has_its_own_token(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;

    let line =
        |sets: u32, reps: u32, pct: f64| json!([{"sets": sets, "reps": reps, "load_pct": pct}]);
    let cases: Vec<(&str, Value)> = vec![
        ("no_exercises", {
            let mut p = program();
            p["core"] = json!([]);
            p["accessories"] = json!([]);
            p
        }),
        ("blank_exercise", {
            let mut p = program();
            p["accessories"] = json!([ex("   ")]);
            p
        }),
        ("duplicate_exercise", {
            let mut p = program();
            p["accessories"] = json!([ex("SQUAT")]);
            p
        }),
        ("no_schedule_days", {
            let mut p = program();
            p["schedule"]["days"] = json!([]);
            p
        }),
        ("no_scheduled_entries", {
            let mut p = program();
            p["schedule"]["days"] = json!([[], [], []]);
            p
        }),
        ("unknown_exercise", {
            let mut p = program();
            p["schedule"]["days"][0] = json!([{"exercise": "Ghost Lift", "class": "low"}]);
            p
        }),
        ("empty_work_lines", {
            let mut p = program();
            p["core"][0]["low"]["work"] = json!([]);
            p
        }),
        ("bad_intensity", {
            let mut p = program();
            p["core"][0]["low"]["work"] = line(3, 5, 1.5);
            p
        }),
        ("bad_reps", {
            let mut p = program();
            p["core"][0]["low"]["work"] = line(3, 0, 0.70);
            p
        }),
        ("bad_sets", {
            let mut p = program();
            p["core"][0]["low"]["work"] = line(0, 5, 0.70);
            p
        }),
    ];
    assert_eq!(cases.len(), 10, "all ten program-level tokens are covered");

    for (expected, bad) in cases {
        let res = post_json_with_auth(
            &app,
            "/authored-programs",
            Some(&bearer(&token)),
            create_body(&bad),
        )
        .await;
        assert_eq!(
            res.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{expected} must be a 422"
        );
        let body = body_json(res).await;
        assert_eq!(body["error"], "unprocessable");
        assert_eq!(body["reason"], expected);
    }
}

// ---------------------------------------------------------------------------
// §7.4 — `bad_plate`, the eleventh token, reachable only from /materialized
// ---------------------------------------------------------------------------

/// `?plate_kg=` must actually change the rounding, not merely be accepted.
/// A 5x100 kg squat gives Epley e1RM 116.667; High is 88% of that = 102.667.
/// Rounding is to the *nearest* increment, so that is 102.5 on the 2.5 kg
/// default and 105.0 on 5 kg plates — the disagreement is what proves the
/// parameter is applied rather than merely parsed.
#[sqlx::test(migrations = "../../migrations")]
async fn plate_kg_changes_the_rounding(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;
    log_lift(&app, &token, "Squat", 5, 100.0).await;
    let id = create_program(&app, &token, &program()).await;

    let res = get_with_auth(
        &app,
        &format!("/authored-programs/{id}/materialized?plate_kg=5"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let coarse = body_json(res).await;
    assert_eq!(
        first_load(&coarse, 0).as_f64().unwrap(),
        105.0,
        "5 kg plates: 102.667 rounds to the nearest 5, which is 105"
    );

    let res = get_with_auth(
        &app,
        &format!("/authored-programs/{id}/materialized?plate_kg=2.5"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let fine = body_json(res).await;
    assert_eq!(
        first_load(&fine, 0).as_f64().unwrap(),
        102.5,
        "2.5 kg plates land on 102.5 — so the query parameter is really applied"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn zero_plate_is_the_only_materialize_only_rejection(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;
    let id = create_program(&app, &token, &program()).await;

    let res = get_with_auth(
        &app,
        &format!("/authored-programs/{id}/materialized?plate_kg=0"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body_json(res).await["reason"], "bad_plate");
}

// ---------------------------------------------------------------------------
// §7.5 — program name is request-level validation → 400 (not an AuthorError)
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn bad_program_names_are_a_400_on_field_name(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;

    for bad in ["", "   ", &"x".repeat(121)] {
        let mut p = program();
        p["name"] = json!(bad);
        let res = post_json_with_auth(
            &app,
            "/authored-programs",
            Some(&bearer(&token)),
            create_body(&p),
        )
        .await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST, "name {bad:?}");
        let body = body_json(res).await;
        assert_eq!(body["error"], "validation");
        assert_eq!(body["field"], "name");
    }
}

// ---------------------------------------------------------------------------
// §7.6 / §7.7 — list (AC4)
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn list_is_owner_scoped_and_newest_first(pool: PgPool) {
    let app = build_app(pool);
    let (_a, token_a) = register_and_token(&app, "a@b.com", "8charsmin").await;
    let (_b, token_b) = register_and_token(&app, "b@b.com", "8charsmin").await;

    for name in ["first", "second", "third"] {
        let mut p = program();
        p["name"] = json!(name);
        create_program(&app, &token_a, &p).await;
    }
    let mut theirs = program();
    theirs["name"] = json!("not mine");
    create_program(&app, &token_b, &theirs).await;

    let res = get_with_auth(&app, "/authored-programs", Some(&bearer(&token_a))).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 3, "only the caller's own rows");
    // Membership is deterministic; order is asserted as a property rather than a
    // fixed sequence, because `created_at DESC, id DESC` falls back to a random
    // UUIDv4 on a microsecond tie and the exact sequence would then be arbitrary.
    let names: std::collections::BTreeSet<&str> =
        rows.iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        ["first", "second", "third"].into_iter().collect(),
        "exactly the caller's three programs"
    );
    let stamps: Vec<&str> = rows
        .iter()
        .map(|r| r["created_at"].as_str().unwrap())
        .collect();
    assert!(
        stamps.windows(2).all(|w| w[0] >= w[1]),
        "newest first: {stamps:?}"
    );
    assert_eq!(rows[0]["name"], "third", "newest of the three");
    // The list is a pure projection: no program body on the wire.
    assert!(rows[0].get("program").is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_with_no_rows_is_an_empty_array_not_a_404(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;

    let res = get_with_auth(&app, "/authored-programs", Some(&bearer(&token))).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await, json!([]));
}

// ---------------------------------------------------------------------------
// §7.8 / §7.9 — fetch one (AC5) and ownership as absence (AC6)
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn fetch_one_returns_the_stored_program(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;
    let id = create_program(&app, &token, &program()).await;

    let res = get_with_auth(
        &app,
        &format!("/authored-programs/{id}"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["id"], id);
    assert_eq!(body["program"]["name"], "Push Pull Legs");
    assert_eq!(body["program"]["core"][0]["name"], "Squat");
}

#[sqlx::test(migrations = "../../migrations")]
async fn another_users_program_is_404_never_403(pool: PgPool) {
    let app = build_app(pool);
    let (_a, token_a) = register_and_token(&app, "a@b.com", "8charsmin").await;
    let (_b, token_b) = register_and_token(&app, "b@b.com", "8charsmin").await;
    let id = create_program(&app, &token_a, &program()).await;

    for path in [
        format!("/authored-programs/{id}"),
        format!("/authored-programs/{id}/materialized"),
    ] {
        let res = get_with_auth(&app, &path, Some(&bearer(&token_b))).await;
        assert_eq!(
            res.status(),
            StatusCode::NOT_FOUND,
            "{path} for a non-owner"
        );
        assert_eq!(body_json(res).await["error"], "not_found");
    }

    // An id that never existed is indistinguishable from a foreign one.
    let missing = Uuid::new_v4();
    let res = get_with_auth(
        &app,
        &format!("/authored-programs/{missing}"),
        Some(&bearer(&token_b)),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// §7.10 / §7.11 / §7.12 — materialize against the caller's own e1RM (AC7)
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn materialized_uses_the_callers_e1rm_and_rounds_to_plates(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;
    log_lift(&app, &token, "Squat", 5, 100.0).await;
    let id = create_program(&app, &token, &program()).await;

    let res = get_with_auth(
        &app,
        &format!("/authored-programs/{id}/materialized"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let cycle = body_json(res).await;

    assert_eq!(cycle["cycle_days"], 3);
    assert_eq!(cycle["days"][0]["day_index"], 1);
    assert_eq!(cycle["days"][0]["entries"][0]["lift"], "Squat");
    assert_eq!(cycle["days"][0]["entries"][0]["class"], "high");
    // Epley e1RM = 100 × (1 + 5/30) = 116.667; High = 88% → 102.667 → 102.5.
    assert_eq!(first_load(&cycle, 0).as_f64().unwrap(), 102.5);
    // Day 2 is a rest day; the schedule shape survives materialization.
    assert_eq!(cycle["days"][1]["entries"].as_array().unwrap().len(), 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn an_untrained_lift_gets_a_null_load_not_an_error(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;
    log_lift(&app, &token, "Squat", 5, 100.0).await;
    let id = create_program(&app, &token, &program()).await;

    let res = get_with_auth(
        &app,
        &format!("/authored-programs/{id}/materialized"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let cycle = body_json(res).await;

    // Bench Press (day 3) has never been logged: reps and % still prescribed.
    let bench = &cycle["days"][2]["entries"][0];
    assert_eq!(bench["lift"], "Bench Press");
    assert!(
        first_load(&cycle, 2).is_null(),
        "an untrained lift must yield a null load"
    );
    assert_eq!(bench["work_sets"][0]["reps"], 5);
    assert_eq!(bench["work_sets"][0]["intensity_pct"], 0.80);
}

/// The cross-user leak AC6 does not cover: user B's training must not anchor
/// user A's loads.
#[sqlx::test(migrations = "../../migrations")]
async fn materialized_never_borrows_another_users_e1rm(pool: PgPool) {
    let app = build_app(pool);
    let (_a, token_a) = register_and_token(&app, "a@b.com", "8charsmin").await;
    let (_b, token_b) = register_and_token(&app, "b@b.com", "8charsmin").await;

    log_lift(&app, &token_b, "Squat", 5, 200.0).await;
    let id = create_program(&app, &token_a, &program()).await;

    let res = get_with_auth(
        &app,
        &format!("/authored-programs/{id}/materialized"),
        Some(&bearer(&token_a)),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let cycle = body_json(res).await;
    assert!(
        first_load(&cycle, 0).is_null(),
        "user A has logged nothing; B's squat must not leak in"
    );
}

// ---------------------------------------------------------------------------
// §7.13 — round-trip fidelity (AC9)
// ---------------------------------------------------------------------------

/// Asserted on the **deserialized value** via the derived `PartialEq`, never on
/// JSON text: Postgres normalizes JSONB key order, so byte-for-byte equality is
/// not the guarantee and testing it would be flaky.
#[sqlx::test(migrations = "../../migrations")]
async fn a_program_round_trips_as_the_same_domain_value(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;

    let sent: AuthoredProgram = serde_json::from_value(program()).unwrap();
    let id = create_program(&app, &token, &program()).await;

    let res = get_with_auth(
        &app,
        &format!("/authored-programs/{id}"),
        Some(&bearer(&token)),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let read_back: AuthoredProgram =
        serde_json::from_value(body_json(res).await["program"].clone()).unwrap();

    assert_eq!(sent, read_back);
}
