//! SPEC-0066 §2.3 — the back squat in the library: an adapter over
//! `biomech::squat_range`, with its numbers unchanged.

use super::body::{BodyMeasures, STANDARD_DORSIFLEXION_DEG};
use super::description::{Description, Equipment, Group, Parameter, ParameterKind};
use super::input::{Invalid, Values};
use super::range::{End, ExerciseRange, Frame, NamedAngle, NamedPoint, NamedSpread, Reason};
use super::{ExerciseModel, LibraryError};
use crate::biomech::{
    squat_range, Assumption, Body, Dorsiflexion, FrameCount, Joint, Metres, ModelError, RangeEnd,
    RangeFrame, COM_PLACEMENT_SPREAD_M, L5S1_PLACEMENT_SPREAD_M,
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
            // Dorsiflexion is the lifter's, a body measure (SPEC-0066 §2.3.1).
            body: &["thigh", "shank", "trunk", "dorsiflexion_deg"],
            parameters: vec![Parameter {
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
            }],
            pairs: &[],
            in_plan_as: Some(Lift::Squat),
        }
    }

    fn range(
        &self,
        body: &Body,
        measures: &BodyMeasures,
        values: &Values,
        frames: FrameCount,
    ) -> Result<ExerciseRange, LibraryError> {
        // The standard body's cited dorsiflexion when the lifter gave none,
        // stated in the result (SPEC-0066 §2.3.1). The model itself always
        // takes one (SPEC-0045 §2.6.1.1).
        let ankle = match measures.dorsiflexion {
            Some(lifter_s) => lifter_s,
            None => Dorsiflexion::from_degrees(STANDARD_DORSIFLEXION_DEG).ok_or(Invalid {
                field: "dorsiflexion_deg",
            })?,
        };
        let bar_kg = values.number("bar_kg").ok_or(Invalid { field: "bar_kg" })?;
        let bar = if bar_kg > 0.0 {
            Some(LoadKg::try_new(bar_kg).map_err(|_| Invalid { field: "bar_kg" })?)
        } else {
            None
        };
        let settings = values.clone();
        let table_lengths: Vec<&'static str> = self
            .describe()
            .body
            .iter()
            .copied()
            .filter(|key| !measures.has(key))
            .collect();
        let standard = measures.dorsiflexion.is_none();
        let with_standard = |mut assumptions: Vec<Assumption>| {
            if standard {
                assumptions.push(Assumption::StandardDorsiflexion);
            }
            assumptions
        };
        let (end, frames, assumptions) = match squat_range(body, bar, ankle, frames) {
            Ok(range) => (
                end_of(range.end),
                range.frames.iter().map(frame_of).collect(),
                range.assumptions,
            ),
            Err(ModelError::Rejected(rejection)) => (
                End::Rejected {
                    reason: Reason::Squat(rejection),
                },
                Vec::new(),
                Vec::new(),
            ),
            Err(ModelError::Kernel(e)) => return Err(e.into()),
        };
        Ok(ExerciseRange {
            end,
            frames,
            settings,
            spreads: spreads(),
            table_lengths,
            assumptions: with_standard(assumptions),
        })
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

/// A squat frame's points, moments and assumptions, unchanged.
fn frame_of(f: &RangeFrame) -> Frame {
    let p = &f.posture;
    let named = |name, at| NamedPoint { name, at };
    let mut points = vec![
        named("ankle", p.ankle),
        named("knee", p.knee),
        named("hip", p.hip),
        named("shoulder", p.shoulder),
    ];
    if let Some(load) = p.load {
        points.push(named("bar", load.at));
    }
    let angle = |name, deg| NamedAngle { name, deg };
    Frame {
        progress: f.progress,
        points,
        angles: vec![
            angle("shank", f.shank_deg),
            angle("knee_flexion", f.knee_flexion_deg),
            angle("hip_flexion", f.hip_flexion_deg),
            angle("trunk_lean", f.trunk_lean_deg),
        ],
        moments: f.mechanics.joints.to_vec(),
        assumptions: f.mechanics.assumptions.clone(),
    }
}

/// The squat's placement spreads, SPEC-0045 §2.8's `Uncertainty`.
fn spreads() -> Vec<NamedSpread> {
    vec![
        NamedSpread {
            name: "l5s1_placement",
            metres: Metres(L5S1_PLACEMENT_SPREAD_M),
        },
        NamedSpread {
            name: "com_placement",
            metres: Metres(COM_PLACEMENT_SPREAD_M),
        },
    ]
}
