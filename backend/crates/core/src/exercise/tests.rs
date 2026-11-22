//! SPEC-0066 slice 66-b1 — the exercise library: its one list, the generic
//! validation of a request against a description, and the back squat served
//! through the library with the same numbers as `squat_range`.

use super::{
    back_squat::end_of, find, library, validate, Description, End, Equipment, Group, Invalid,
    Ordered, Parameter, ParameterKind, Reason, Value,
};
use crate::biomech::{
    scale, squat_range, Dorsiflexion, FrameCount, Joint, MeasuredLengths, Metres, RangeEnd,
    Rejection,
};
use crate::workout::LoadKg;

// ---------------------------------------------------------------------------
// The list
// ---------------------------------------------------------------------------

#[test]
fn the_library_serves_the_back_squat_under_a_stable_id() {
    let squat = find("back_squat").unwrap_or_else(|| panic!("back_squat is in the library"));
    let d = squat.describe();
    assert_eq!(d.id, "back_squat");
    assert_eq!(d.equipment, Equipment::FreeWeight);
    assert_eq!(
        d.joints,
        &[Joint::Hip, Joint::Knee, Joint::Ankle, Joint::Lumbar]
    );
}

#[test]
fn every_id_in_the_library_is_unique() {
    let mut ids: Vec<&str> = library().iter().map(|m| m.describe().id).collect();
    let n = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), n, "duplicate ids");
}

#[test]
fn an_unknown_id_is_not_found() {
    assert!(find("bench_press").is_none());
}

// ---------------------------------------------------------------------------
// Generic validation (SPEC-0066 §2.3)
// ---------------------------------------------------------------------------

/// A description no real exercise has, so every rule is exercised: a
/// required number, a defaulted number, a choice and an ordered pair.
fn fixture() -> Description {
    Description {
        id: "fixture",
        name_es: "Prueba",
        name_en: "Fixture",
        aliases: &[],
        equipment: Equipment::Cable,
        joints: &[Joint::Hip],
        lengths: &["thigh", "upper_arm"],
        parameters: vec![
            Parameter {
                key: "angle_deg",
                group: Group::Motion,
                kind: ParameterKind::Number {
                    unit: "deg",
                    min: 0.0,
                    max: 90.0,
                    step: 1.0,
                    default: None,
                },
            },
            Parameter {
                key: "low_deg",
                group: Group::Motion,
                kind: ParameterKind::Number {
                    unit: "deg",
                    min: 0.0,
                    max: 60.0,
                    step: 1.0,
                    default: Some(10.0),
                },
            },
            Parameter {
                key: "high_deg",
                group: Group::Motion,
                kind: ParameterKind::Number {
                    unit: "deg",
                    min: 30.0,
                    max: 150.0,
                    step: 1.0,
                    default: Some(100.0),
                },
            },
            Parameter {
                key: "cop",
                group: Group::Machine,
                kind: ParameterKind::Choice {
                    options: &["heel", "mid", "toes"],
                    default: Some("mid"),
                },
            },
        ],
        pairs: &[Ordered {
            low: "low_deg",
            high: "high_deg",
        }],
        in_plan_as: None,
    }
}

fn number(key: &str, n: f64) -> (String, Value) {
    (key.to_owned(), Value::Number(n))
}

#[test]
fn missing_keys_take_their_defaults() {
    let v = validate(&fixture(), &[number("angle_deg", 45.0)], &[])
        .unwrap_or_else(|e| panic!("valid: {e:?}"));
    assert_eq!(v.number("angle_deg"), Some(45.0));
    assert_eq!(v.number("low_deg"), Some(10.0));
    assert_eq!(v.number("high_deg"), Some(100.0));
    assert_eq!(v.choice("cop"), Some("mid"));
}

/// A parameter with no default is required (dorsiflexion is one: SPEC-0045
/// commits no uncited population default).
#[test]
fn a_missing_required_key_is_invalid() {
    assert_eq!(
        validate(&fixture(), &[], &[]).err(),
        Some(Invalid { field: "angle_deg" })
    );
}

/// An unknown key is invalid. It is reported as `values`, never echoed: the
/// API's field is a fixed token, and a request's own text never comes back.
#[test]
fn an_unknown_key_is_invalid() {
    let err = validate(
        &fixture(),
        &[number("angle_deg", 45.0), number("tilt_deg", 3.0)],
        &[],
    )
    .err();
    assert_eq!(err, Some(Invalid { field: "values" }));
}

#[test]
fn an_unknown_length_key_is_invalid() {
    let err = validate(
        &fixture(),
        &[number("angle_deg", 45.0)],
        &[("wingspan".to_owned(), 1.8)],
    )
    .err();
    assert_eq!(err, Some(Invalid { field: "lengths" }));
}

#[test]
fn a_number_outside_its_range_is_invalid() {
    for bad in [-0.1, 90.1, f64::NAN] {
        assert_eq!(
            validate(&fixture(), &[number("angle_deg", bad)], &[]).err(),
            Some(Invalid { field: "angle_deg" }),
            "{bad}"
        );
    }
}

#[test]
fn a_choice_not_in_its_options_is_invalid() {
    let values = [
        number("angle_deg", 45.0),
        ("cop".to_owned(), Value::Choice("arch".to_owned())),
    ];
    assert_eq!(
        validate(&fixture(), &values, &[]).err(),
        Some(Invalid { field: "cop" })
    );
}

#[test]
fn a_value_of_the_wrong_kind_is_invalid() {
    let values = [("angle_deg".to_owned(), Value::Choice("forty".to_owned()))];
    assert_eq!(
        validate(&fixture(), &values, &[]).err(),
        Some(Invalid { field: "angle_deg" })
    );
}

/// `low < high`, else the `high` key is named.
#[test]
fn a_broken_pair_names_its_high_key() {
    let values = [
        number("angle_deg", 45.0),
        number("low_deg", 50.0),
        number("high_deg", 40.0),
    ];
    assert_eq!(
        validate(&fixture(), &values, &[]).err(),
        Some(Invalid { field: "high_deg" })
    );
}

#[test]
fn a_length_the_exercise_uses_is_accepted_and_bounded() {
    let ok = validate(
        &fixture(),
        &[number("angle_deg", 45.0)],
        &[("upper_arm".to_owned(), 0.31)],
    )
    .unwrap_or_else(|e| panic!("valid: {e:?}"));
    assert_eq!(ok.lengths().upper_arm, Some(Metres(0.31)));
    for bad in [0.05, 1.6] {
        assert_eq!(
            validate(
                &fixture(),
                &[number("angle_deg", 45.0)],
                &[("thigh".to_owned(), bad)]
            )
            .err(),
            Some(Invalid { field: "thigh" }),
            "{bad}"
        );
    }
}

#[test]
fn a_length_the_exercise_does_not_use_is_invalid() {
    assert_eq!(
        validate(
            &fixture(),
            &[number("angle_deg", 45.0)],
            &[("shank".to_owned(), 0.4)]
        )
        .err(),
        Some(Invalid { field: "shank" })
    );
}

// ---------------------------------------------------------------------------
// The back squat through the library
// ---------------------------------------------------------------------------

fn frames() -> FrameCount {
    FrameCount::new(21).unwrap_or_else(|| panic!("21 is valid"))
}

fn squat_values(dorsi: f64, bar_kg: f64) -> Vec<(String, Value)> {
    vec![number("dorsiflexion_deg", dorsi), number("bar_kg", bar_kg)]
}

/// The library's squat has `squat_range`'s moments, to 1e-9, frame by frame.
#[test]
fn the_back_squat_through_the_library_matches_squat_range() {
    let body = scale(Metres(1.78), 77.0, &MeasuredLengths::default());
    let squat = find("back_squat").unwrap_or_else(|| panic!("in the library"));
    for (dorsi, bar_kg) in [(29.0, 84.0), (38.0, 0.0), (45.0, 140.0)] {
        let values = validate(&squat.describe(), &squat_values(dorsi, bar_kg), &[])
            .unwrap_or_else(|e| panic!("valid: {e:?}"));
        let got = squat
            .range(&body, &values, frames())
            .unwrap_or_else(|e| panic!("kernel: {e}"));
        let bar = (bar_kg > 0.0)
            .then(|| LoadKg::try_new(bar_kg).unwrap_or_else(|e| panic!("valid bar: {e:?}")));
        let ankle = Dorsiflexion::from_degrees(dorsi).unwrap_or_else(|| panic!("valid"));
        let want =
            squat_range(&body, bar, ankle, frames()).unwrap_or_else(|e| panic!("solves: {e:?}"));
        assert_eq!(got.frames.len(), want.frames.len(), "{dorsi}° {bar_kg} kg");
        for (g, w) in got.frames.iter().zip(&want.frames) {
            assert!((g.progress - w.progress).abs() < 1e-12);
            for (gm, wm) in g.moments.iter().zip(&w.mechanics.joints) {
                assert_eq!(gm.joint, wm.joint);
                assert!((gm.moment.0 - wm.moment.0).abs() <= 1e-9);
                assert_eq!(gm.resisting, wm.resisting);
            }
        }
        assert_eq!(
            got.end,
            End::Complete,
            "{dorsi}° {bar_kg} kg reach the bottom"
        );
    }
}

/// A bar of 0 kg is the bodyweight squat (`LoadKg` is `(0, 1000]`).
#[test]
fn a_zero_bar_is_bodyweight_only() {
    let squat = find("back_squat").unwrap_or_else(|| panic!("in the library"));
    let v = validate(&squat.describe(), &squat_values(38.0, 0.0), &[])
        .unwrap_or_else(|e| panic!("valid: {e:?}"));
    let body = scale(Metres(1.75), 80.0, &MeasuredLengths::default());
    let r = squat
        .range(&body, &v, frames())
        .unwrap_or_else(|e| panic!("kernel: {e}"));
    assert!(r
        .frames
        .iter()
        .all(|f| f.points.iter().all(|(name, _)| *name != "bar")));
}

/// The squat requires its dorsiflexion: no uncited default (SPEC-0045).
#[test]
fn the_back_squat_requires_a_dorsiflexion() {
    let squat = find("back_squat").unwrap_or_else(|| panic!("in the library"));
    assert_eq!(
        validate(&squat.describe(), &[number("bar_kg", 60.0)], &[]).err(),
        Some(Invalid {
            field: "dorsiflexion_deg"
        })
    );
}

#[test]
fn a_lifter_who_cannot_reach_depth_is_a_rejected_end() {
    let squat = find("back_squat").unwrap_or_else(|| panic!("in the library"));
    let v = validate(&squat.describe(), &squat_values(0.0, 0.0), &[])
        .unwrap_or_else(|e| panic!("valid: {e:?}"));
    let body = scale(Metres(1.75), 80.0, &MeasuredLengths::default());
    let r = squat
        .range(&body, &v, frames())
        .unwrap_or_else(|e| panic!("kernel: {e}"));
    assert!(r.frames.is_empty());
    assert!(matches!(
        r.end,
        End::Rejected {
            reason: Reason::Squat(Rejection::DepthNotReachable { .. })
        }
    ));
}

/// Every `RangeEnd` maps: `Stopped { bottom: None }` (the bottom balances,
/// the convention passes horizontal on the way) has a reason of its own.
#[test]
fn every_squat_range_end_maps_to_a_library_end() {
    assert_eq!(end_of(RangeEnd::Bottom), End::Complete);
    assert_eq!(
        end_of(RangeEnd::Stopped {
            reached: 0.5,
            bottom: None
        }),
        End::Stopped {
            reached: 0.5,
            reason: Reason::ConventionPassesHorizontal
        }
    );
    let r = Rejection::DoesNotBalance {
        unknown: crate::biomech::Unknown::TrunkAngle,
    };
    assert_eq!(
        end_of(RangeEnd::Stopped {
            reached: 0.7,
            bottom: Some(r)
        }),
        End::Stopped {
            reached: 0.7,
            reason: Reason::Squat(r)
        }
    );
}
