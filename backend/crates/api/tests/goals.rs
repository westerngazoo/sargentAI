//! R-0042 goal-tracking integration suite — SPEC-0042 §7.
//!
//! Every test is `#[sqlx::test(migrations = "../../migrations")]`: sqlx
//! provisions a fresh per-test database, applies the migrations (including
//! `00010_goals.sql`), and hands a connected `PgPool` to the test.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Baselines land on exactly representable Epley values, so `==` is intended.
#![allow(clippy::float_cmp)]
// Test doc comments quote JSON literals and token names as prose, not code.
#![allow(clippy::doc_markdown)]

mod common;

use axum::http::StatusCode;
use chrono::{Duration, NaiveDate, Utc};
use common::{
    body_json, build_app, delete_with_auth, get_with_auth, post_json_with_auth, register_and_token,
};
use fitai_core::goals::{Baseline, Goal, GoalKind};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

/// A valid dated strength target ~13 weeks out.
fn strength_body(lift: &str, target: f64) -> Value {
    json!({
        "kind": { "kind": "strength", "lift": lift, "target_e1rm_kg": target },
        "target_date": (today() + Duration::days(90)).to_string(),
    })
}

/// Create a goal for `token`, asserting 201, and return the response body.
async fn create_goal(app: &axum::Router, token: &str, body: Value) -> Value {
    let res = post_json_with_auth(app, "/goals", Some(&bearer(token)), body).await;
    assert_eq!(
        res.status(),
        StatusCode::CREATED,
        "seed create expected 201"
    );
    body_json(res).await
}

/// Log one `lift` session on `on` for `token` through the real workout route.
async fn log_lift(
    app: &axum::Router,
    token: &str,
    lift: &str,
    on: NaiveDate,
    rep_count: u32,
    kg: f64,
) {
    let res = post_json_with_auth(
        app,
        "/workouts",
        Some(&bearer(token)),
        json!({
            "performed_on": on.to_string(),
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

/// Log one body measurement through the real measurements route.
async fn measure(app: &axum::Router, token: &str, on: NaiveDate, weight_kg: f64) {
    let res = post_json_with_auth(
        app,
        "/measurements",
        Some(&bearer(token)),
        json!({ "measured_on": on.to_string(), "weight_kg": weight_kg }),
    )
    .await;
    assert_eq!(
        res.status(),
        StatusCode::CREATED,
        "seed measurement expected 201"
    );
}

/// The overall status token of a response's report.
fn status_of(body: &Value) -> &str {
    body["report"]["status"]["status"].as_str().unwrap()
}

// ---------------------------------------------------------------------------
// §7 — auth is required on every route, including a malformed id
// ---------------------------------------------------------------------------

/// `AuthenticatedUser` runs before `Path`, so an unparseable UUID with no
/// token is 401, never axum's 400.
#[sqlx::test(migrations = "../../migrations")]
async fn every_route_requires_a_jwt(pool: PgPool) {
    let app = build_app(pool);
    let id = Uuid::new_v4();

    let res = post_json_with_auth(&app, "/goals", None, strength_body("Squat", 140.0)).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "POST without a JWT");

    for path in [
        "/goals".to_string(),
        format!("/goals/{id}"),
        "/goals/not-a-uuid".to_string(),
    ] {
        let res = get_with_auth(&app, &path, None).await;
        assert_eq!(
            res.status(),
            StatusCode::UNAUTHORIZED,
            "GET {path} without a JWT must be 401, never 400"
        );
    }
    for path in [format!("/goals/{id}"), "/goals/not-a-uuid".to_string()] {
        let res = delete_with_auth(&app, &path, None).await;
        assert_eq!(
            res.status(),
            StatusCode::UNAUTHORIZED,
            "DELETE {path} without a JWT must be 401, never 400"
        );
    }
}

// ---------------------------------------------------------------------------
// §7 — create returns 201 with the initial report (AC1, AC4, AC5)
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn create_returns_the_goal_with_an_honest_report(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;
    log_lift(&app, &token, "Squat", today(), 5, 100.0).await;

    let body = create_goal(&app, &token, strength_body("Squat", 140.0)).await;
    assert!(Uuid::parse_str(body["id"].as_str().unwrap()).is_ok());
    assert!(!body["created_at"].is_null());
    assert_eq!(body["goal"]["kind"]["kind"], "strength");
    assert_eq!(body["goal"]["set_on"], today().to_string());
    // One session sits below the strength floor: an explicit reason, never a
    // fabricated projection (AC5).
    assert_eq!(status_of(&body), "insufficient_data");
    assert_eq!(
        body["report"]["status"]["detail"]["reason"],
        "too_few_observations"
    );
    let metric = &body["report"]["metrics"][0];
    assert_eq!(metric["metric"]["metric"], "e1rm_kg");
    assert_eq!(metric["metric"]["lift"], "Squat");
    assert!(metric["projected_at_deadline"].is_null());
}

/// An already-met target is 201 + Achieved at creation (SPEC-0042 §2.2.2).
/// 15×100 kg is Epley e1RM 150.0 exactly, so the target can equal it.
#[sqlx::test(migrations = "../../migrations")]
async fn create_with_a_met_target_is_achieved(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;
    log_lift(&app, &token, "Squat", today(), 15, 100.0).await;

    let body = create_goal(&app, &token, strength_body("Squat", 150.0)).await;
    assert_eq!(status_of(&body), "achieved");
}

// ---------------------------------------------------------------------------
// §7 — table-driven GoalError token sweep (AC10 as amended)
// ---------------------------------------------------------------------------

/// Every rejection reachable through JSON has its own fixed token. The
/// eleventh variant, non_finite_target, cannot arrive over the wire (JSON has
/// no NaN/Infinity literal) and is pinned by the core unit suite instead.
#[sqlx::test(migrations = "../../migrations")]
async fn every_goal_rejection_has_its_own_token(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;

    let ok_date = (today() + Duration::days(90)).to_string();
    let strength = |lift: &str, target: f64, date: &str| {
        json!({
            "kind": { "kind": "strength", "lift": lift, "target_e1rm_kg": target },
            "target_date": date,
        })
    };
    let body = |w: Value, bf: Value| {
        json!({
            "kind": { "kind": "body", "target_weight_kg": w, "target_body_fat_pct": bf },
            "target_date": ok_date,
        })
    };
    let consistency = |rate: u32| {
        json!({
            "kind": { "kind": "consistency", "sessions_per_week": rate },
            "target_date": ok_date,
        })
    };

    let cases: Vec<(&str, Value)> = vec![
        (
            "target_date_not_after_creation",
            strength("Squat", 140.0, &today().to_string()),
        ),
        (
            "target_date_too_far",
            strength("Squat", 140.0, &(today() + Duration::days(731)).to_string()),
        ),
        ("non_positive_target", strength("Squat", 0.0, &ok_date)),
        ("target_too_large", strength("Squat", 1501.0, &ok_date)),
        ("body_fat_out_of_range", body(Value::Null, json!(80.0))),
        ("body_target_empty", body(Value::Null, Value::Null)),
        ("blank_lift", strength("   ", 140.0, &ok_date)),
        ("lift_too_long", strength(&"x".repeat(81), 140.0, &ok_date)),
        ("zero_sessions", consistency(0)),
        ("too_many_sessions", consistency(15)),
    ];
    assert_eq!(cases.len(), 10, "all ten wire-reachable tokens are covered");

    for (expected, bad) in cases {
        let res = post_json_with_auth(&app, "/goals", Some(&bearer(&token)), bad).await;
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
// §7 — the baseline comes from logged data, never from the request (AC1)
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn baseline_is_captured_server_side_not_taken_from_the_request(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;
    log_lift(&app, &token, "Squat", today(), 15, 100.0).await; // e1RM 150.0

    // A client-supplied "baseline" is not part of the contract and is ignored.
    let mut req = strength_body("Squat", 200.0);
    req["baseline"] = json!({ "kind": "strength", "e1rm_kg": 999.0 });
    let body = create_goal(&app, &token, req).await;
    assert_eq!(
        body["goal"]["baseline"]["e1rm_kg"].as_f64().unwrap(),
        150.0,
        "the measured e1RM, not the client's 999"
    );

    // A body goal captures the logged measurement the same way.
    measure(&app, &token, today(), 90.0).await;
    let body = create_goal(
        &app,
        &token,
        json!({
            "kind": { "kind": "body", "target_weight_kg": 80.0, "target_body_fat_pct": null },
            "target_date": (today() + Duration::days(90)).to_string(),
        }),
    )
    .await;
    assert_eq!(
        body["goal"]["baseline"]["weight_kg"].as_f64().unwrap(),
        90.0
    );
    assert!(body["goal"]["baseline"]["body_fat_pct"].is_null());
}

// ---------------------------------------------------------------------------
// §7 — ownership as absence (AC3) and cross-user signal isolation
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn another_users_goal_is_404_never_403(pool: PgPool) {
    let app = build_app(pool);
    let (_a, token_a) = register_and_token(&app, "a@b.com", "8charsmin").await;
    let (_b, token_b) = register_and_token(&app, "b@b.com", "8charsmin").await;
    let created = create_goal(&app, &token_a, strength_body("Squat", 140.0)).await;
    let id = created["id"].as_str().unwrap();

    let res = get_with_auth(&app, &format!("/goals/{id}"), Some(&bearer(&token_b))).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "GET as a non-owner");
    assert_eq!(body_json(res).await["error"], "not_found");

    let res = delete_with_auth(&app, &format!("/goals/{id}"), Some(&bearer(&token_b))).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "DELETE as a non-owner");

    // The owner still has the goal — the foreign delete removed nothing.
    let res = get_with_auth(&app, &format!("/goals/{id}"), Some(&bearer(&token_a))).await;
    assert_eq!(res.status(), StatusCode::OK);

    // An id that never existed is indistinguishable from a foreign one.
    let missing = Uuid::new_v4();
    let res = get_with_auth(&app, &format!("/goals/{missing}"), Some(&bearer(&token_a))).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

/// User B's training must not anchor user A's baseline or report.
#[sqlx::test(migrations = "../../migrations")]
async fn a_goal_never_reads_another_users_signal(pool: PgPool) {
    let app = build_app(pool);
    let (_a, token_a) = register_and_token(&app, "a@b.com", "8charsmin").await;
    let (_b, token_b) = register_and_token(&app, "b@b.com", "8charsmin").await;
    log_lift(&app, &token_b, "Squat", today(), 5, 200.0).await;

    let body = create_goal(&app, &token_a, strength_body("Squat", 140.0)).await;
    assert!(
        body["goal"]["baseline"]["e1rm_kg"].is_null(),
        "user A has logged nothing; B's squat must not leak in"
    );
    assert_eq!(status_of(&body), "insufficient_data");
    assert_eq!(body["report"]["status"]["detail"]["reason"], "no_signal");
}

// ---------------------------------------------------------------------------
// §7 — delete → 204 then 404 (AC11)
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn delete_removes_the_goal_then_404s(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;
    let created = create_goal(&app, &token, strength_body("Squat", 140.0)).await;
    let id = created["id"].as_str().unwrap();

    let res = delete_with_auth(&app, &format!("/goals/{id}"), Some(&bearer(&token))).await;
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = get_with_auth(&app, &format!("/goals/{id}"), Some(&bearer(&token))).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let res = delete_with_auth(&app, &format!("/goals/{id}"), Some(&bearer(&token))).await;
    assert_eq!(
        res.status(),
        StatusCode::NOT_FOUND,
        "a second delete is a 404"
    );
}

// ---------------------------------------------------------------------------
// §7 — list: caller's goals, newest first, each with its report
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn list_is_owner_scoped_and_newest_first(pool: PgPool) {
    let app = build_app(pool);
    let (_a, token_a) = register_and_token(&app, "a@b.com", "8charsmin").await;
    let (_b, token_b) = register_and_token(&app, "b@b.com", "8charsmin").await;

    for lift in ["Squat", "Bench Press", "Deadlift"] {
        create_goal(&app, &token_a, strength_body(lift, 140.0)).await;
    }
    create_goal(&app, &token_b, strength_body("Curl", 40.0)).await;

    let res = get_with_auth(&app, "/goals", Some(&bearer(&token_a))).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 3, "only the caller's own rows");
    // Membership is deterministic; order is asserted as a property because a
    // microsecond `created_at` tie falls back to a random UUIDv4.
    let lifts: std::collections::BTreeSet<&str> = rows
        .iter()
        .map(|r| r["goal"]["kind"]["lift"].as_str().unwrap())
        .collect();
    assert_eq!(
        lifts,
        ["Squat", "Bench Press", "Deadlift"].into_iter().collect()
    );
    let stamps: Vec<&str> = rows
        .iter()
        .map(|r| r["created_at"].as_str().unwrap())
        .collect();
    assert!(
        stamps.windows(2).all(|w| w[0] >= w[1]),
        "newest first: {stamps:?}"
    );
    for row in rows {
        assert!(
            row["report"]["status"]["status"].is_string(),
            "every listed goal carries its report"
        );
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_with_no_rows_is_an_empty_array_not_a_404(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;

    let res = get_with_auth(&app, "/goals", Some(&bearer(&token))).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await, json!([]));
}

// ---------------------------------------------------------------------------
// §7 — round-trip equality on the deserialized Goal (the R-0041 AC9 pattern)
// ---------------------------------------------------------------------------

/// Asserted on the **deserialized value** via the derived `PartialEq`, never
/// on JSON text: Postgres normalizes JSONB key order.
#[sqlx::test(migrations = "../../migrations")]
async fn a_goal_round_trips_as_the_same_domain_value(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;

    let created = create_goal(&app, &token, strength_body("Squat", 140.0)).await;
    let id = created["id"].as_str().unwrap();

    let expected = Goal {
        kind: GoalKind::Strength {
            lift: "Squat".into(),
            target_e1rm_kg: 140.0,
        },
        baseline: Baseline::Strength { e1rm_kg: None },
        set_on: today(),
        target_date: today() + Duration::days(90),
    };

    let res = get_with_auth(&app, &format!("/goals/{id}"), Some(&bearer(&token))).await;
    assert_eq!(res.status(), StatusCode::OK);
    let read_back: Goal = serde_json::from_value(body_json(res).await["goal"].clone()).unwrap();
    assert_eq!(read_back, expected);
}

// ---------------------------------------------------------------------------
// §7 — deadline anchoring: an expired goal is terminal and stable (AC6)
// ---------------------------------------------------------------------------

/// Insert an expired goal row directly (create validates dates, so the API
/// cannot mint one) and return its id.
async fn insert_expired_goal(pool: &PgPool, user_id: &str, goal: &Goal) -> Uuid {
    let (id,): (Uuid,) =
        sqlx::query_as("INSERT INTO goals (user_id, goal) VALUES ($1, $2) RETURNING id")
            .bind(Uuid::parse_str(user_id).unwrap())
            .bind(serde_json::to_value(goal).unwrap())
            .fetch_one(pool)
            .await
            .unwrap();
    id
}

/// Today's session lies AFTER the expired deadline, so the deadline-anchored
/// signal must not see it: the goal is Missed even though the user's current
/// e1RM meets the target. The unanchored reading would say Achieved — the
/// difference is exactly SPEC-0042 §2.2.4. The active goal on the same list
/// reads the same session and achieves, so one request serves two anchors.
#[sqlx::test(migrations = "../../migrations")]
async fn an_expired_goal_is_anchored_at_its_deadline(pool: PgPool) {
    let app = build_app(pool.clone());
    let (user_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;
    log_lift(&app, &token, "Squat", today(), 15, 100.0).await; // e1RM 150.0

    let expired = Goal {
        kind: GoalKind::Strength {
            lift: "Squat".into(),
            target_e1rm_kg: 140.0,
        },
        baseline: Baseline::Strength {
            e1rm_kg: Some(100.0),
        },
        set_on: today() - Duration::days(120),
        target_date: today() - Duration::days(10),
    };
    let expired_id = insert_expired_goal(&pool, &user_id, &expired).await;
    create_goal(&app, &token, strength_body("Squat", 150.0)).await; // Achieved now

    let res = get_with_auth(&app, &format!("/goals/{expired_id}"), Some(&bearer(&token))).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(
        status_of(&body),
        "missed",
        "today's 150 e1RM is after the deadline and must not count"
    );
    let metric = &body["report"]["metrics"][0];
    assert!(
        metric["required_per_week"].is_null(),
        "terminal: no required pace"
    );
    assert!(
        metric["projected_at_deadline"].is_null(),
        "terminal: no projection"
    );

    let res = get_with_auth(&app, "/goals", Some(&bearer(&token))).await;
    assert_eq!(res.status(), StatusCode::OK);
    let rows = body_json(res).await;
    let statuses: std::collections::BTreeSet<&str> =
        rows.as_array().unwrap().iter().map(status_of).collect();
    assert_eq!(
        statuses,
        ["achieved", "missed"].into_iter().collect(),
        "two goals, two anchor dates, one honest answer each"
    );
}

// ---------------------------------------------------------------------------
// §2.6 — the per-user goal cap (AC12; the R-0041 DoS lesson, self-inflicted
// variant: every stored goal is an assess per list read, every expired goal
// its own summarize anchor)
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn the_twenty_sixth_goal_is_rejected_with_its_own_token(pool: PgPool) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "a@b.com", "8charsmin").await;

    for i in 0..25 {
        create_goal(&app, &token, strength_body(&format!("Lift {i}"), 100.0)).await;
    }

    let res = post_json_with_auth(
        &app,
        "/goals",
        Some(&bearer(&token)),
        strength_body("One Too Many", 100.0),
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(res).await;
    assert_eq!(body["reason"], "too_many_goals");

    // The cap is per-user, not global: a second user still creates freely.
    let (_id, other) = register_and_token(&app, "b@b.com", "8charsmin").await;
    create_goal(&app, &other, strength_body("Squat", 100.0)).await;
}
