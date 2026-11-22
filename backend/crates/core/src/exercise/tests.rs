//! SPEC-0066 slice 66-b1 — the exercise library: its one list, the generic
//! validation of a request against a description, and the back squat served
//! through the library with the same numbers as `squat_range`.

use super::{
    back_squat::end_of, find, input::LENGTH_KEYS, library, validate, Description, End, Equipment,
    Group, Invalid, Ordered, Parameter, ParameterKind, Reason, Value,
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
                    min: 0.0,
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
    assert_eq!(v.settings.number("angle_deg"), Some(45.0));
    assert_eq!(v.settings.number("low_deg"), Some(10.0));
    assert_eq!(v.settings.number("high_deg"), Some(100.0));
    assert_eq!(v.settings.choice("cop"), Some("mid"));
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
    assert_eq!(ok.lengths.upper_arm, Some(Metres(0.31)));
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
            .range(&body, &values.settings, frames())
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
        .range(&body, &v.settings, frames())
        .unwrap_or_else(|e| panic!("kernel: {e}"));
    assert!(r
        .frames
        .iter()
        .all(|f| f.points.iter().all(|p| p.name != "bar")));
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
        .range(&body, &v.settings, frames())
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

// ---------------------------------------------------------------------------
// Architect review of PR #196
// ---------------------------------------------------------------------------

#[test]
fn a_repeated_key_is_invalid() {
    let values = [number("angle_deg", 45.0), number("angle_deg", 50.0)];
    assert_eq!(
        validate(&fixture(), &values, &[]).err(),
        Some(Invalid { field: "values" })
    );
    let lengths = [("thigh".to_owned(), 0.4), ("thigh".to_owned(), 0.45)];
    assert_eq!(
        validate(&fixture(), &[number("angle_deg", 45.0)], &lengths).err(),
        Some(Invalid { field: "lengths" })
    );
}

#[test]
fn a_number_given_to_a_choice_is_invalid() {
    let values = [number("angle_deg", 45.0), number("cop", 1.0)];
    assert_eq!(
        validate(&fixture(), &values, &[]).err(),
        Some(Invalid { field: "cop" })
    );
}

/// A pair broken by the request against a default still names the `high`
/// key, the control R-0067 AC8 tells the lifter to move back.
#[test]
fn a_pair_broken_against_a_default_names_its_high_key() {
    // low_deg defaults to 10; a high of 5 breaks the pair against it.
    let values = [number("angle_deg", 45.0), number("high_deg", 5.0)];
    assert_eq!(
        validate(&fixture(), &values, &[]).err(),
        Some(Invalid { field: "high_deg" })
    );
}

/// Every description in the library is well formed (SPEC-0066 §2.3): unique
/// keys, `min <= max`, defaults in bounds and among the options, pairs that
/// name number parameters and hold for the defaults, lengths the request can
/// carry. A typo in a pair key would otherwise switch its check off.
#[test]
fn every_library_description_is_well_formed() {
    for model in library() {
        let d = model.describe();
        let mut keys: Vec<&str> = d.parameters.iter().map(|p| p.key).collect();
        let n = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), n, "{}: duplicate parameter keys", d.id);
        for p in &d.parameters {
            match &p.kind {
                ParameterKind::Number {
                    min,
                    max,
                    default,
                    step,
                    ..
                } => {
                    assert!(min <= max && *step > 0.0, "{}.{}: bounds", d.id, p.key);
                    if let Some(v) = default {
                        assert!((min..=max).contains(&v), "{}.{}: default", d.id, p.key);
                    }
                }
                ParameterKind::Choice { options, default } => {
                    assert!(!options.is_empty(), "{}.{}: options", d.id, p.key);
                    if let Some(v) = default {
                        assert!(options.contains(v), "{}.{}: default", d.id, p.key);
                    }
                }
            }
        }
        let number_default =
            |key: &str| d.parameters.iter().find(|p| p.key == key).map(|p| &p.kind);
        for pair in d.pairs {
            for key in [pair.low, pair.high] {
                assert!(
                    matches!(number_default(key), Some(ParameterKind::Number { .. })),
                    "{}: pair key {key} is not a number parameter",
                    d.id
                );
            }
            if let (
                Some(ParameterKind::Number {
                    default: Some(lo), ..
                }),
                Some(ParameterKind::Number {
                    default: Some(hi), ..
                }),
            ) = (number_default(pair.low), number_default(pair.high))
            {
                assert!(lo < hi, "{}: defaults run {} backwards", d.id, pair.high);
            }
        }
        for length in d.lengths {
            assert!(LENGTH_KEYS.contains(length), "{}: length {length}", d.id);
        }
    }
}

/// One `reason` key per object on the wire: the library's `Reason` is
/// tagged `source`, the squat's `Rejection` keeps `reason`.
#[test]
fn a_rejected_end_serializes_without_a_repeated_key() {
    let end = End::Rejected {
        reason: Reason::Squat(Rejection::DepthNotReachable {
            limiting: Joint::Ankle,
            needed_deg: 3.0,
            available_deg: 0.0,
        }),
    };
    let json = serde_json::to_string(&end).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(json.matches("\"reason\"").count(), 2, "{json}");
    assert_eq!(json.matches("\"source\"").count(), 1, "{json}");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(value["reason"]["source"], "squat");
    assert_eq!(value["reason"]["reason"], "depth_not_reachable");
}

/// The squat through the library states every assumption `squat_range` does,
/// frame by frame (R-0066 §1).
#[test]
fn the_squat_through_the_library_keeps_every_frame_s_assumptions() {
    let body = scale(Metres(1.78), 77.0, &MeasuredLengths::default());
    let squat = find("back_squat").unwrap_or_else(|| panic!("in the library"));
    let v = validate(&squat.describe(), &squat_values(29.0, 84.0), &[])
        .unwrap_or_else(|e| panic!("valid: {e:?}"));
    let got = squat
        .range(&body, &v.settings, frames())
        .unwrap_or_else(|e| panic!("kernel: {e}"));
    let ankle = Dorsiflexion::from_degrees(29.0).unwrap_or_else(|| panic!("valid"));
    let bar = LoadKg::try_new(84.0).unwrap_or_else(|e| panic!("{e:?}"));
    let want = squat_range(&body, Some(bar), ankle, frames()).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(got.assumptions, want.assumptions);
    for (g, w) in got.frames.iter().zip(&want.frames) {
        assert_eq!(g.assumptions, w.mechanics.assumptions);
    }
}

/// The settings serialize as one flat object the client can read by key.
#[test]
fn the_settings_serialize_as_one_flat_object() {
    let v = validate(&fixture(), &[number("angle_deg", 45.0)], &[])
        .unwrap_or_else(|e| panic!("valid: {e:?}"));
    let json: serde_json::Value =
        serde_json::to_value(&v.settings).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(json["angle_deg"], 45.0);
    assert_eq!(json["cop"], "mid");
}
