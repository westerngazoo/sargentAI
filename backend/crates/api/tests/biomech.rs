//! R-0059 / SPEC-0059 §2.2 — `POST /biomech/squat/range` (slice 59-a).
//!
//! AC1: a valid request's shape, bodyweight-only, every field at both bounds
//! and just outside, no session. AC2: `table_lengths` both ways. AC3: a
//! stopped range and a lifter the engine cannot place are both `200`. One
//! golden test pins the wire. AC8: the body's numbers never reach the log.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::float_cmp)]

mod common;

use axum::{http::StatusCode, Router};
use common::{body_json, build_app, post_json_with_auth, register_and_token};
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing_test::traced_test;

const PATH: &str = "/biomech/squat/range";

/// The SPEC-0059 §2.2 example request: reaches the bottom under the bar.
fn valid() -> Value {
    json!({
        "height_m": 1.78, "mass_kg": 77.0,
        "thigh_m": 0.45, "shank_m": 0.42, "trunk_m": null,
        "dorsiflexion_deg": 29.0,
        "bar_kg": 84.0,
        "frames": 41
    })
}

/// `valid()` with one field replaced.
fn with(field: &str, value: Value) -> Value {
    let mut body = valid();
    body[field] = value;
    body
}

async fn authed_app(pool: PgPool) -> (Router, String) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "squat@b.com", "8charsmin").await;
    (app, format!("Bearer {token}"))
}

async fn post(app: &Router, auth: &str, body: Value) -> (StatusCode, Value) {
    let res = post_json_with_auth(app, PATH, Some(auth), body).await;
    let status = res.status();
    (status, body_json(res).await)
}

fn f(v: &Value) -> f64 {
    v.as_f64()
        .unwrap_or_else(|| panic!("expected a number, got {v}"))
}

/// AC1 — a valid request returns frames with the four moments, the `hip` and
/// `ground_n` fields, `end`, and both assumption lists.
#[sqlx::test(migrations = "../../migrations")]
async fn a_valid_request_returns_frames_with_four_moments_and_the_lever(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let (status, body) = post(&app, &auth, valid()).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(body["end"]["kind"], "bottom");
    assert!(!body["assumptions"].as_array().unwrap().is_empty());
    let frames = body["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 41);
    for frame in frames {
        for key in [
            "progress",
            "shank_deg",
            "knee_flexion_deg",
            "hip_flexion_deg",
            "trunk_lean_deg",
        ] {
            assert!(frame[key].is_number(), "{key} missing: {frame}");
        }
        for point in ["ankle", "knee", "hip", "shoulder"] {
            assert!(frame["posture"][point]["x"].is_number());
            assert!(frame["posture"][point]["y"].is_number());
        }
        assert_eq!(frame["posture"]["load"]["kg"], 84.0);
        let joints: Vec<&str> = frame["mechanics"]["joints"]
            .as_array()
            .unwrap()
            .iter()
            .map(|j| j["joint"].as_str().unwrap())
            .collect();
        assert_eq!(joints, ["hip", "knee", "ankle", "lumbar"]);
        for j in frame["mechanics"]["joints"].as_array().unwrap() {
            assert!(j["moment"].is_number() && j["uncertainty"].is_number());
            assert!(j["resisting"].is_string());
        }
        assert_eq!(frame["mechanics"]["load"]["basis"], "bar");
        assert!(!frame["mechanics"]["assumptions"]
            .as_array()
            .unwrap()
            .is_empty());

        let hip = &frame["hip"];
        let (lever, bar_n, bar_nm, body_nm) = (
            f(&hip["lever_m"]),
            f(&hip["bar_n"]),
            f(&hip["bar_nm"]),
            f(&hip["body_nm"]),
        );
        assert!(
            (bar_nm - bar_n * lever).abs() < 1e-9,
            "bar_nm = bar_n × lever"
        );
        let hip_moment = f(&frame["mechanics"]["joints"][0]["moment"]);
        assert!(
            (bar_nm + body_nm - hip_moment).abs() < 1e-9,
            "the sum the check shows is the hip moment"
        );
        assert!(
            f(&frame["ground_n"]) > bar_n,
            "the ground carries bar and body"
        );
    }
}

/// AC1 — `bar_kg: null` is bodyweight only: `load.basis` says so and no
/// frame carries `hip`; `ground_n` is the body's weight.
#[sqlx::test(migrations = "../../migrations")]
async fn bodyweight_only_has_no_hip_lever(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let (status, body) = post(&app, &auth, with("bar_kg", Value::Null)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let frames = body["frames"].as_array().unwrap();
    assert!(!frames.is_empty());
    for frame in frames {
        assert_eq!(frame["mechanics"]["load"]["basis"], "bodyweight_only");
        assert!(frame.get("hip").is_none(), "no hip lever without a bar");
        assert!(frame["posture"]["load"].is_null());
        assert!(f(&frame["ground_n"]) > 0.0);
    }
}

/// AC1 — every field at both bounds is accepted; just outside is a `400`
/// naming it.
#[sqlx::test(migrations = "../../migrations")]
async fn every_field_is_bounded_and_named(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let cases: [(&str, Value, Value, Value, Value); 8] = [
        ("height_m", json!(0.5), json!(3.0), json!(0.49), json!(3.01)),
        (
            "mass_kg",
            json!(20.0),
            json!(500.0),
            json!(19.9),
            json!(500.1),
        ),
        ("thigh_m", json!(0.1), json!(1.5), json!(0.09), json!(1.51)),
        ("shank_m", json!(0.1), json!(1.5), json!(0.09), json!(1.51)),
        ("trunk_m", json!(0.1), json!(1.5), json!(0.09), json!(1.51)),
        (
            "dorsiflexion_deg",
            json!(0.0),
            json!(90.0),
            json!(-0.1),
            json!(90.1),
        ),
        (
            "bar_kg",
            json!(0.1),
            json!(1000.0),
            json!(0.0),
            json!(1000.1),
        ),
        ("frames", json!(2), json!(241), json!(1), json!(242)),
    ];
    for (field, low, high, below, above) in cases {
        for ok in [low, high] {
            let (status, body) = post(&app, &auth, with(field, ok.clone())).await;
            assert_eq!(status, StatusCode::OK, "{field} = {ok} accepted: {body}");
        }
        for bad in [below, above] {
            let (status, body) = post(&app, &auth, with(field, bad.clone())).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{field} = {bad}");
            assert_eq!(body, json!({"error": "validation", "field": field}));
        }
    }
}

/// AC1 — `frames` is optional and defaults to 41; dorsiflexion is required.
#[sqlx::test(migrations = "../../migrations")]
async fn frames_default_to_41_and_dorsiflexion_is_required(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let mut body = valid();
    body.as_object_mut().unwrap().remove("frames");
    let (status, got) = post(&app, &auth, body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["frames"].as_array().unwrap().len(), 41);

    let mut body = valid();
    body.as_object_mut().unwrap().remove("dorsiflexion_deg");
    let (status, got) = post(&app, &auth, body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(got["error"], "validation");
}

/// AC1 — no session is a `401`.
#[sqlx::test(migrations = "../../migrations")]
async fn no_session_is_401(pool: PgPool) {
    let app = build_app(pool);
    let res = post_json_with_auth(&app, PATH, None, valid()).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// AC2 — nulls in the lengths are listed in `table_lengths`; a full set
/// gives an empty list.
#[sqlx::test(migrations = "../../migrations")]
async fn table_lengths_lists_the_nulls(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let mut none = valid();
    for k in ["thigh_m", "shank_m", "trunk_m"] {
        none[k] = Value::Null;
    }
    let (_, got) = post(&app, &auth, none).await;
    assert_eq!(got["table_lengths"], json!(["thigh", "shank", "trunk"]));

    let (_, got) = post(&app, &auth, with("trunk_m", json!(0.52))).await;
    assert_eq!(got["table_lengths"], json!([]));

    let (_, got) = post(&app, &auth, valid()).await;
    assert_eq!(got["table_lengths"], json!(["trunk"]));
}

/// AC3 — the SPEC-0045 R7 lifter (1.75 m, 80 kg, 25°, bodyweight) stops
/// before the bottom: `200`, `kind: stopped`, `reached` and the bottom's own
/// rejection, and frames up to where the lifter got.
#[sqlx::test(migrations = "../../migrations")]
async fn a_lifter_who_stops_is_a_200_with_the_reason(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let req = json!({
        "height_m": 1.75, "mass_kg": 80.0,
        "dorsiflexion_deg": 25.0, "bar_kg": null, "frames": 41
    });
    let (status, body) = post(&app, &auth, req).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["end"]["kind"], "stopped");
    let reached = f(&body["end"]["reached"]);
    assert!((reached - 0.565).abs() < 0.001, "reached {reached}");
    assert_eq!(body["end"]["bottom"]["reason"], "depth_not_reachable");
    assert_eq!(body["end"]["bottom"]["limiting"], "ankle");
    let frames = body["frames"].as_array().unwrap();
    assert!(!frames.is_empty() && frames.len() < 41);
    assert!(frames.iter().all(|fr| f(&fr["progress"]) <= reached));
}

/// AC3 — a lifter the engine cannot place (2°, below the upright lean) is a
/// `200` with `kind: rejected`, the typed reason and `frames: []`.
#[sqlx::test(migrations = "../../migrations")]
async fn a_lifter_who_cannot_be_placed_is_a_200_rejected(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let (status, body) = post(&app, &auth, with("dorsiflexion_deg", json!(2.0))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["end"]["kind"], "rejected");
    assert_eq!(body["end"]["rejection"]["reason"], "depth_not_reachable");
    assert_eq!(body["end"]["rejection"]["limiting"], "ankle");
    assert_eq!(f(&body["end"]["rejection"]["available_deg"]), 2.0);
    assert_eq!(body["frames"], json!([]));
    assert_eq!(body["table_lengths"], json!(["trunk"]));
}

/// SPEC-0059 §2.2 — the golden: the example request at two frames, compared
/// as parsed JSON with a 1e-6 tolerance on floats, so a core refactor cannot
/// change the wire unnoticed.
#[sqlx::test(migrations = "../../migrations")]
async fn the_wire_matches_the_golden(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let (status, got) = post(&app, &auth, with("frames", json!(2))).await;
    assert_eq!(status, StatusCode::OK);
    let golden: Value =
        serde_json::from_str(include_str!("fixtures/biomech_squat_range_golden.json"))
            .expect("the golden fixture is JSON");
    assert_json_close(&got, &golden, "$");

    // Published in SPEC-0059 §2.2's example, not produced by this commit.
    let published = [33.52, -71.49, 109.56, 22.74];
    for (i, want) in published.into_iter().enumerate() {
        let moment = f(&got["frames"][0]["mechanics"]["joints"][i]["moment"]);
        assert!(
            (moment - want).abs() <= 0.01,
            "joint {i}: {moment} vs spec {want}"
        );
    }
}

/// Structural equality with a 1e-6 tolerance on numbers.
fn assert_json_close(got: &Value, want: &Value, path: &str) {
    match (got, want) {
        (Value::Number(g), Value::Number(w)) => {
            let (g, w) = (g.as_f64().unwrap(), w.as_f64().unwrap());
            assert!((g - w).abs() <= 1e-6, "{path}: {g} vs golden {w}");
        }
        (Value::Array(g), Value::Array(w)) => {
            assert_eq!(g.len(), w.len(), "{path}: array length");
            for (i, (g, w)) in g.iter().zip(w).enumerate() {
                assert_json_close(g, w, &format!("{path}[{i}]"));
            }
        }
        (Value::Object(g), Value::Object(w)) => {
            let mut gk: Vec<_> = g.keys().collect();
            let mut wk: Vec<_> = w.keys().collect();
            gk.sort();
            wk.sort();
            assert_eq!(gk, wk, "{path}: keys");
            for (k, w) in w {
                assert_json_close(&g[k], w, &format!("{path}.{k}"));
            }
        }
        _ => assert_eq!(got, want, "{path}"),
    }
}

/// Recognisable inputs: none of these digit strings occurs in a response
/// that would otherwise be logged, so a hit in the capture is a leak.
const SECRET_NUMBERS: [&str; 6] = ["1.7731", "77.123", "0.4567", "0.4231", "29.137", "84.321"];

/// Emitted by the test itself to prove the capture records anything at all.
const CAPTURE_MARKER: &str = "r0059-no-log-control-marker";

fn recognisable(dorsiflexion: f64) -> Value {
    json!({
        "height_m": 1.7731, "mass_kg": 77.123,
        "thigh_m": 0.4567, "shank_m": 0.4231, "trunk_m": null,
        "dorsiflexion_deg": dorsiflexion, "bar_kg": 84.321, "frames": 3
    })
}

fn assert_no_secret_in(lines: &[&str]) -> Result<(), String> {
    for line in lines {
        for secret in SECRET_NUMBERS {
            if line.contains(secret) {
                return Err(format!("the log leaked {secret}: {line}"));
            }
        }
    }
    Ok(())
}

/// AC8 — over the three paths (valid, `JsonRejection`, engine rejection) the
/// body's numbers never reach the tracing capture.
#[traced_test]
#[sqlx::test(migrations = "../../migrations")]
async fn the_body_is_never_logged(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;

    let (status, _) = post(&app, &auth, recognisable(29.137)).await;
    assert_eq!(status, StatusCode::OK, "valid path");

    let mut malformed = recognisable(29.137);
    malformed["height_m"] = json!("1.7731");
    let (status, body) = post(&app, &auth, malformed).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "JsonRejection path");
    assert_eq!(body, json!({"error": "validation", "field": "body"}));

    let (status, body) = post(&app, &auth, recognisable(2.0)).await;
    assert_eq!(status, StatusCode::OK, "engine rejection path");
    assert_eq!(body["end"]["kind"], "rejected");

    // Positive control: the capture is live, so a clean capture means something.
    tracing::info!("{CAPTURE_MARKER}");
    logs_assert(|lines: &[&str]| {
        if lines.iter().any(|line| line.contains(CAPTURE_MARKER)) {
            Ok(())
        } else {
            Err("the capture did not record the control marker".to_owned())
        }
    });
    logs_assert(assert_no_secret_in);
}
