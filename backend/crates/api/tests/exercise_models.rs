//! R-0066 / SPEC-0066 §2.4 — the exercise model library over HTTP (slice
//! 66-b2b): `GET /exercise-models`, `GET /exercise-models/{id}` and
//! `POST /exercise-models/{id}/range`.
//!
//! AC8 and AC9: the list, a description and a range, each behind a session.
//! The status codes of §2.4: `400` naming the field, `401`, `404` (before the
//! body is validated), `200` for a stopped or rejected end. The squat through the library equals the squat
//! through `POST /biomech/squat/range` within 1e-9 N·m at every frame and
//! joint. SPEC-0059 AC8: no body value reaches the log.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::float_cmp)]

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use common::{body_json, build_app, get_with_auth, post_json_with_auth, register_and_token};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use tracing_test::traced_test;

const LIST: &str = "/exercise-models";
const SQUAT: &str = "/exercise-models/back_squat";
const SQUAT_RANGE: &str = "/exercise-models/back_squat/range";
const OLD_RANGE: &str = "/biomech/squat/range";

/// SPEC-0059 §2.2's example lifter, in the library's request shape.
fn valid() -> Value {
    json!({
        "height_m": 1.78, "mass_kg": 77.0,
        "body": { "thigh": 0.45, "shank": 0.42, "dorsiflexion_deg": 29.0 },
        "values": { "bar_kg": 84.0 },
        "frames": 41
    })
}

async fn authed_app(pool: PgPool) -> (Router, String) {
    let app = build_app(pool);
    let (_id, token) = register_and_token(&app, "library@b.com", "8charsmin").await;
    (app, format!("Bearer {token}"))
}

async fn get(app: &Router, path: &str, auth: Option<&str>) -> (StatusCode, Value) {
    let res = get_with_auth(app, path, auth).await;
    let status = res.status();
    (status, body_json(res).await)
}

async fn post(app: &Router, path: &str, auth: Option<&str>, body: Value) -> (StatusCode, Value) {
    let res = post_json_with_auth(app, path, auth, body).await;
    let status = res.status();
    (status, body_json(res).await)
}

/// POST a raw body, for JSON a `Value` cannot hold (a repeated key).
async fn post_raw(app: &Router, path: &str, auth: &str, body: &str) -> (StatusCode, Value) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .header("authorization", auth)
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    (status, body_json(res).await)
}

fn f(v: &Value) -> f64 {
    v.as_f64()
        .unwrap_or_else(|| panic!("expected a number, got {v}"))
}

/// The standard body's bodyweight squat ends at the bottom (1.75 m, 75 kg,
/// the table, 38.8°).
const STANDARD_END: &str = "complete";

fn validation(field: &str) -> Value {
    json!({"error": "validation", "field": field})
}

/// AC9 — the list names every modelled exercise with its equipment and joints.
#[sqlx::test(migrations = "../../migrations")]
async fn the_list_names_the_back_squat(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let (status, body) = get(&app, LIST, Some(&auth)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let squat = body
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == "back_squat")
        .unwrap_or_else(|| panic!("no back_squat in {body}"));
    assert_eq!(
        squat,
        &json!({
            "id": "back_squat",
            "name_es": "Sentadilla trasera",
            "name_en": "Back squat",
            "equipment": "free_weight",
            "joints": ["hip", "knee", "ankle", "lumbar"]
        })
    );
}

/// AC9 — a description carries the body measures and the parameters the app
/// builds its controls from, with their bounds and defaults.
#[sqlx::test(migrations = "../../migrations")]
async fn a_description_carries_its_body_measures_and_parameters(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let (status, body) = get(&app, SQUAT, Some(&auth)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["id"], "back_squat");
    assert_eq!(body["equipment"], "free_weight");
    let keys: Vec<&str> = body["body"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, ["thigh", "shank", "trunk", "dorsiflexion_deg"]);
    let dorsiflexion = &body["body"][3];
    assert_eq!(
        dorsiflexion,
        &json!({"key": "dorsiflexion_deg", "unit": "deg", "min": 0.0, "max": 90.0,
                "step": 0.5, "standard": 38.8})
    );
    assert_eq!(body["body"][0]["standard"], Value::Null);
    assert_eq!(
        body["parameters"],
        json!([{"key": "bar_kg", "group": "load", "kind": {
            "kind": "number", "unit": "kg", "min": 0.0, "max": 1000.0,
            "step": 2.5, "default": 0.0
        }}])
    );
    assert_eq!(body["pairs"], json!([]));
    assert!(body.get("in_plan_as").is_none(), "{body}");
}

/// §2.4 — an unknown id is `404` on the description and on the range.
#[sqlx::test(migrations = "../../migrations")]
async fn an_unknown_id_is_404(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let (status, _) = get(&app, "/exercise-models/front_lever", Some(&auth)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = post(
        &app,
        "/exercise-models/front_lever/range",
        Some(&auth),
        valid(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = post_raw(&app, "/exercise-models/front_lever/range", &auth, "{").await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "404 wins over a body that does not parse"
    );
}

/// §2.4 — every route needs a session, and `401` comes before `404`.
#[sqlx::test(migrations = "../../migrations")]
async fn no_session_is_401_everywhere(pool: PgPool) {
    let app = build_app(pool);
    for path in [LIST, SQUAT, "/exercise-models/front_lever"] {
        let (status, _) = get(&app, path, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "GET {path}");
    }
    let (status, _) = post(&app, SQUAT_RANGE, None, valid()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

/// AC8, AC2 — a valid request returns every frame's points, angles and four
/// moments, the settings with defaults filled in, the spreads, and the body
/// it ran on with each measure's source.
#[sqlx::test(migrations = "../../migrations")]
async fn a_valid_range_restates_what_it_ran_on(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let (status, body) = post(&app, SQUAT_RANGE, Some(&auth), valid()).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(body["end"], json!({"kind": "complete"}));
    assert_eq!(body["settings"], json!({"bar_kg": 84.0}));
    assert_eq!(
        body["spreads"],
        json!([{"name": "l5s1_placement", "metres": 0.03},
               {"name": "com_placement", "metres": 0.02}])
    );
    let used = body["body"].as_array().unwrap();
    let source = |key: &str| {
        used.iter()
            .find(|m| m["key"] == key)
            .map(|m| m["source"].as_str().unwrap().to_owned())
    };
    assert_eq!(source("thigh").as_deref(), Some("measured"));
    assert_eq!(source("shank").as_deref(), Some("measured"));
    assert_eq!(source("trunk").as_deref(), Some("table"));
    assert_eq!(source("dorsiflexion_deg").as_deref(), Some("measured"));

    let frames = body["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 41);
    for frame in frames {
        let names = |key: &str| -> Vec<String> {
            frame[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p["name"].as_str().unwrap().to_owned())
                .collect()
        };
        assert_eq!(names("points"), ["ankle", "knee", "hip", "shoulder", "bar"]);
        assert_eq!(
            names("angles"),
            ["shank", "knee_flexion", "hip_flexion", "trunk_lean"]
        );
        assert_eq!(frame["moments"].as_array().unwrap().len(), 4);
    }
    assert!(!body["assumptions"]
        .as_array()
        .unwrap()
        .contains(&json!({"assumption": "standard_dorsiflexion"})));
}

/// §2.3.1 — with nothing but height and mass, the range runs on the table and
/// the standard body, says so, and samples 41 frames.
#[sqlx::test(migrations = "../../migrations")]
async fn the_standard_body_runs_and_is_stated(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let (status, body) = post(
        &app,
        SQUAT_RANGE,
        Some(&auth),
        json!({"height_m": 1.75, "mass_kg": 75.0}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["settings"], json!({"bar_kg": 0.0}));
    assert!(body["assumptions"]
        .as_array()
        .unwrap()
        .contains(&json!({"assumption": "standard_dorsiflexion"})));
    let dorsiflexion = body["body"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["key"] == "dorsiflexion_deg")
        .unwrap();
    assert_eq!(
        dorsiflexion,
        &json!({"key": "dorsiflexion_deg", "value": 38.8, "source": "standard"})
    );
    assert_eq!(body["end"]["kind"], STANDARD_END, "{}", body["end"]);
    assert_eq!(body["frames"].as_array().unwrap().len(), 41);
}

/// §2.4 — SPEC-0059's bounds on height, mass and frames, and the library's
/// validation of `values` and `body`, each naming its field.
#[sqlx::test(migrations = "../../migrations")]
async fn every_field_is_bounded_and_named(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let cases: Vec<(Value, &str)> = vec![
        (json!({"height_m": 0.49}), "height_m"),
        (json!({"height_m": 3.01}), "height_m"),
        (json!({"mass_kg": 19.9}), "mass_kg"),
        (json!({"mass_kg": 500.1}), "mass_kg"),
        (json!({"frames": 1}), "frames"),
        (json!({"frames": 242}), "frames"),
        (json!({"frames": 100_000}), "frames"),
        (json!({"values": {"bar_kg": -1.0}}), "bar_kg"),
        (json!({"values": {"bar_kg": 1000.5}}), "bar_kg"),
        (json!({"values": {"bar_kg": "heavy"}}), "bar_kg"),
        (json!({"values": {"tempo": 3.0}}), "values"),
        (json!({"body": {"thigh": 0.05}}), "thigh"),
        (json!({"body": {"trunk": 1.6}}), "trunk"),
        (
            json!({"body": {"dorsiflexion_deg": 91.0}}),
            "dorsiflexion_deg",
        ),
        (json!({"body": {"upper_arm": 0.3}}), "upper_arm"),
        (json!({"body": {"neck": 0.1}}), "body"),
        (json!({"body": {"thigh": "long"}}), "body"),
        (json!({"values": {"bar_kg": true}}), "body"),
        (json!({"height_m": "1.78"}), "body"),
    ];
    for (patch, field) in cases {
        let mut request = valid();
        for (key, value) in patch.as_object().unwrap() {
            request[key] = value.clone();
        }
        let (status, body) = post(&app, SQUAT_RANGE, Some(&auth), request.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{request} → {body}");
        assert_eq!(body, validation(field), "{request}");
    }
}

/// §2.3 — a key given twice is ambiguous, so the request is invalid, never
/// "the last one wins".
#[sqlx::test(migrations = "../../migrations")]
async fn a_repeated_key_is_rejected(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let values = r#"{"height_m": 1.78, "mass_kg": 77.0,
                     "values": {"bar_kg": 60.0, "bar_kg": 100.0}}"#;
    let (status, body) = post_raw(&app, SQUAT_RANGE, &auth, values).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body, validation("values"));

    let measures = r#"{"height_m": 1.78, "mass_kg": 77.0,
                       "body": {"thigh": 0.45, "thigh": 0.50}}"#;
    let (status, body) = post_raw(&app, SQUAT_RANGE, &auth, measures).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body, validation("body"));
}

/// §2.4, SPEC-0059 AC3 — the SPEC-0045 R7 lifter (1.75 m, 80 kg, 25°,
/// bodyweight) stops before the bottom: `200`, a stopped end with `reached`
/// and the squat's reason, and frames only up to where the lifter got.
#[sqlx::test(migrations = "../../migrations")]
async fn a_lifter_who_stops_is_a_200_with_the_reason(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let request = json!({
        "height_m": 1.75, "mass_kg": 80.0,
        "body": { "dorsiflexion_deg": 25.0 }, "frames": 41
    });
    let (status, body) = post(&app, SQUAT_RANGE, Some(&auth), request).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["end"]["kind"], "stopped");
    let reached = f(&body["end"]["reached"]);
    assert!((reached - 0.565).abs() < 0.001, "reached {reached}");
    assert_eq!(body["end"]["reason"]["source"], "squat");
    assert_eq!(body["end"]["reason"]["reason"], "depth_not_reachable");
    assert_eq!(body["end"]["reason"]["limiting"], "ankle");
    let frames = body["frames"].as_array().unwrap();
    assert!(!frames.is_empty() && frames.len() < 41);
    assert!(frames.iter().all(|fr| f(&fr["progress"]) <= reached));
}

/// SPEC-0059 accepted `null` for every optional field; so does this request.
#[sqlx::test(migrations = "../../migrations")]
async fn null_optional_fields_are_absent_fields(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let request = json!({
        "height_m": 1.75, "mass_kg": 75.0,
        "body": null, "values": null, "frames": null
    });
    let (status, body) = post(&app, SQUAT_RANGE, Some(&auth), request).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["settings"], json!({"bar_kg": 0.0}));
}

/// §2.4, SPEC-0059 AC3 — a lifter the squat cannot place is a `200` with a
/// rejected end and no frames.
#[sqlx::test(migrations = "../../migrations")]
async fn a_lifter_who_cannot_be_placed_is_a_200_rejected(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let mut request = valid();
    request["body"]["dorsiflexion_deg"] = json!(2.0);
    let (status, body) = post(&app, SQUAT_RANGE, Some(&auth), request).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["end"]["kind"], "rejected");
    assert_eq!(body["end"]["reason"]["source"], "squat");
    assert!(body["frames"].as_array().unwrap().is_empty());
}

/// SPEC-0066 §2.4 — the squat through the library is the squat through
/// SPEC-0059's endpoint: the same frames, and every joint's moment and
/// uncertainty within 1e-9 N·m, for a loaded lifter, a bodyweight one and a
/// table-only one. The old endpoint always gets the dorsiflexion it requires.
#[sqlx::test(migrations = "../../migrations")]
async fn the_library_squat_equals_the_old_endpoint(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;
    let lifters = [
        (1.78, 77.0, Some((0.45, 0.42)), 29.0, 84.0),
        (1.75, 80.0, None, 38.8, 0.0),
        (1.62, 58.0, Some((0.38, 0.39)), 41.0, 40.0),
        // Stops part-way: the prefix and its end are compared too.
        (1.75, 80.0, None, 25.0, 0.0),
    ];
    for (height, mass, lengths, dorsiflexion, bar) in lifters {
        let (thigh, shank) =
            lengths.map_or((Value::Null, Value::Null), |(t, s)| (json!(t), json!(s)));
        let old = json!({
            "height_m": height, "mass_kg": mass,
            "thigh_m": thigh, "shank_m": shank, "trunk_m": null,
            "dorsiflexion_deg": dorsiflexion,
            "bar_kg": if bar > 0.0 { json!(bar) } else { Value::Null },
            "frames": 41
        });
        let mut body = json!({ "dorsiflexion_deg": dorsiflexion });
        if let Some((t, s)) = lengths {
            body["thigh"] = json!(t);
            body["shank"] = json!(s);
        }
        let new = json!({
            "height_m": height, "mass_kg": mass, "body": body,
            "values": { "bar_kg": bar }, "frames": 41
        });

        let (status, old) = post(&app, OLD_RANGE, Some(&auth), old).await;
        assert_eq!(status, StatusCode::OK, "{old}");
        let (status, new) = post(&app, SQUAT_RANGE, Some(&auth), new).await;
        assert_eq!(status, StatusCode::OK, "{new}");

        let old_frames = old["frames"].as_array().unwrap();
        let new_frames = new["frames"].as_array().unwrap();
        assert!(!old_frames.is_empty(), "lifter {height} m has no frames");
        assert_eq!(old_frames.len(), new_frames.len(), "lifter {height} m");
        match old["end"]["kind"].as_str().unwrap() {
            "bottom" => assert_eq!(new["end"]["kind"], "complete"),
            "stopped" => {
                assert_eq!(new["end"]["kind"], "stopped");
                assert_eq!(f(&old["end"]["reached"]), f(&new["end"]["reached"]));
            }
            other => panic!("lifter {height} m: unexpected old end {other}"),
        }
        for (o, n) in old_frames.iter().zip(new_frames) {
            assert_eq!(f(&o["progress"]), f(&n["progress"]));
            for point in n["points"].as_array().unwrap() {
                let name = point["name"].as_str().unwrap();
                let at = if name == "bar" {
                    &o["posture"]["load"]["at"]
                } else {
                    &o["posture"][name]
                };
                for axis in ["x", "y"] {
                    let (a, b) = (f(&at[axis]), f(&point["at"][axis]));
                    assert!(
                        (a - b).abs() <= 1e-9,
                        "{height} m, {name}.{axis}: {a} vs {b}"
                    );
                }
            }
            let old_joints = o["mechanics"]["joints"].as_array().unwrap();
            let new_joints = n["moments"].as_array().unwrap();
            assert_eq!(old_joints.len(), new_joints.len());
            for (oj, nj) in old_joints.iter().zip(new_joints) {
                assert_eq!(oj["joint"], nj["joint"]);
                assert_eq!(oj["resisting"], nj["resisting"]);
                for key in ["moment", "uncertainty"] {
                    let (a, b) = (f(&oj[key]), f(&nj[key]));
                    assert!(
                        (a - b).abs() <= 1e-9,
                        "{height} m, s = {}, {} {key}: {a} vs {b}",
                        o["progress"],
                        oj["joint"]
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SPEC-0059 AC8 — no body value reaches the log.
// ---------------------------------------------------------------------------

/// Distinctive values: a line containing one of them leaked the body.
const SECRET_NUMBERS: [&str; 5] = ["1.7731", "77.123", "0.4567", "29.137", "84.321"];

/// Emitted by the test itself to prove the capture records anything at all.
const CAPTURE_MARKER: &str = "r0066-no-log-control-marker";

fn recognisable(dorsiflexion: f64) -> Value {
    json!({
        "height_m": 1.7731, "mass_kg": 77.123,
        "body": { "thigh": 0.4567, "dorsiflexion_deg": dorsiflexion },
        "values": { "bar_kg": 84.321 },
        "frames": 3
    })
}

/// Over the valid, the unparseable, the invalid and the rejected paths, the
/// body's numbers never reach the tracing capture.
#[traced_test]
#[sqlx::test(migrations = "../../migrations")]
async fn the_body_is_never_logged(pool: PgPool) {
    let (app, auth) = authed_app(pool).await;

    let (status, _) = post(&app, SQUAT_RANGE, Some(&auth), recognisable(29.137)).await;
    assert_eq!(status, StatusCode::OK, "valid path");

    let mut malformed = recognisable(29.137);
    malformed["height_m"] = json!("1.7731");
    let (status, body) = post(&app, SQUAT_RANGE, Some(&auth), malformed).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "JsonRejection path");
    assert_eq!(body, validation("body"));

    let mut invalid = recognisable(29.137);
    invalid["body"]["upper_arm"] = json!(0.4567);
    let (status, body) = post(&app, SQUAT_RANGE, Some(&auth), invalid).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "validation path");
    assert_eq!(body, validation("upper_arm"));

    let (status, body) = post(&app, SQUAT_RANGE, Some(&auth), recognisable(2.0)).await;
    assert_eq!(status, StatusCode::OK, "rejected path");
    assert_eq!(body["end"]["kind"], "rejected");

    tracing::info!("{CAPTURE_MARKER}");
    logs_assert(|lines: &[&str]| {
        if lines.iter().any(|line| line.contains(CAPTURE_MARKER)) {
            Ok(())
        } else {
            Err("the capture did not record the control marker".to_owned())
        }
    });
    logs_assert(|lines: &[&str]| {
        for line in lines {
            for secret in SECRET_NUMBERS {
                if line.contains(secret) {
                    return Err(format!("the log leaked {secret}: {line}"));
                }
            }
        }
        Ok(())
    });
}
