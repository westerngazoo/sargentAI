//! SPEC-0066 §2.3 — the back squat in the library: an adapter over
//! `biomech::squat_range`, with its numbers unchanged.

use super::description::{Description, Equipment, Group, Parameter, ParameterKind};
use super::input::{Invalid, Values};
use super::range::{End, ExerciseRange, Frame, Reason};
use super::{ExerciseModel, LibraryError};
use crate::biomech::{
    squat_range, Body, Dorsiflexion, FrameCount, Joint, ModelError, RangeEnd, RangeFrame,
};
use crate::technique::Lift;
use crate::workout::LoadKg;

/// The high-bar back squat of SPEC-0045 and SPEC-0059.
pub(super) struct BackSquat;

impl ExerciseModel for BackSquat {
    fn describe(&self) -> Description {
        Description {
            id: "back_squat",
            name_es: "Sentadilla trasera",
            name_en: "Back squat",
            aliases: &["back squat", "sentadilla", "sentadilla trasera"],
            equipment: Equipment::FreeWeight,
            joints: &[Joint::Hip, Joint::Knee, Joint::Ankle, Joint::Lumbar],
            lengths: &["thigh", "shank", "trunk"],
            parameters: vec![
                Parameter {
                    key: "dorsiflexion_deg",
                    group: Group::Motion,
                    // Required: SPEC-0045 commits no uncited population default.
                    kind: ParameterKind::Number {
                        unit: "deg",
                        min: 0.0,
                        max: 90.0,
                        step: 0.5,
                        default: None,
                    },
                },
                Parameter {
                    key: "bar_kg",
                    group: Group::Load,
                    // 0 is the bodyweight squat.
                    kind: ParameterKind::Number {
                        unit: "kg",
                        min: 0.0,
                        max: 1000.0,
                        step: 2.5,
                        default: Some(0.0),
                    },
                },
            ],
            pairs: &[],
            in_plan_as: Some(Lift::Squat),
        }
    }

    fn range(
        &self,
        body: &Body,
        values: &Values,
        frames: FrameCount,
    ) -> Result<ExerciseRange, LibraryError> {
        let ankle = values
            .number("dorsiflexion_deg")
            .and_then(Dorsiflexion::from_degrees)
            .ok_or(Invalid {
                field: "dorsiflexion_deg",
            })?;
        let bar_kg = values.number("bar_kg").ok_or(Invalid { field: "bar_kg" })?;
        let bar = if bar_kg > 0.0 {
            Some(LoadKg::try_new(bar_kg).map_err(|_| Invalid { field: "bar_kg" })?)
        } else {
            None
        };
        let settings = values.clone();
        match squat_range(body, bar, ankle, frames) {
            Ok(range) => Ok(ExerciseRange {
                end: end_of(range.end),
                frames: range.frames.iter().map(frame_of).collect(),
                settings,
                assumptions: range.assumptions,
            }),
            Err(ModelError::Rejected(rejection)) => Ok(ExerciseRange {
                end: End::Rejected {
                    reason: Reason::Squat(rejection),
                },
                frames: Vec::new(),
                settings,
                assumptions: Vec::new(),
            }),
            Err(ModelError::Kernel(e)) => Err(e.into()),
        }
    }
}

/// The squat's end in the library's terms. Pure, so every case is testable,
/// including `Stopped { bottom: None }`, which no fixture input reaches.
pub(super) fn end_of(end: RangeEnd) -> End {
    match end {
        RangeEnd::Bottom => End::Complete,
        RangeEnd::Stopped {
            reached,
            bottom: None,
        } => End::Stopped {
            reached,
            reason: Reason::ConventionPassesHorizontal,
        },
        RangeEnd::Stopped {
            reached,
            bottom: Some(rejection),
        } => End::Stopped {
            reached,
            reason: Reason::Squat(rejection),
        },
    }
}

/// A squat frame's points and moments, unchanged.
fn frame_of(f: &RangeFrame) -> Frame {
    let p = &f.posture;
    let mut points = vec![
        ("ankle", p.ankle),
        ("knee", p.knee),
        ("hip", p.hip),
        ("shoulder", p.shoulder),
    ];
    if let Some(load) = p.load {
        points.push(("bar", load.at));
    }
    Frame {
        progress: f.progress,
        points,
        moments: f.mechanics.joints.to_vec(),
    }
}
