//! SPEC-0059 §2.2 — the response envelope: core's `Serialize` output
//! unchanged, plus the per-frame `hip` lever and `ground_n`.

use fitai_core::biomech::{Assumption, RangeEnd, RangeFrame, Rejection, SquatRange};
use serde::Serialize;

use super::{
    lever::{ground_n, hip_lever, HipLever},
    request::Squat,
};

/// How the range ended. Api-owned: `Rejected` is this endpoint's answer to
/// the engine's `Err(Rejection)`, so core's `RangeEnd` keeps its promise of
/// "the largest balanced prefix".
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum End {
    /// Reached the bottom.
    Bottom,
    /// Stopped at `reached`; `bottom` is the bottom's own rejection, if any.
    Stopped {
        /// The progress `s*` the lifter reached, `0..1`.
        reached: f64,
        /// Why the bottom itself fails, or `None` when only the convention stops it.
        bottom: Option<Rejection>,
    },
    /// The engine could not place the lifter at all; there are no frames.
    Rejected { rejection: Rejection },
}

impl From<RangeEnd> for End {
    fn from(end: RangeEnd) -> Self {
        match end {
            RangeEnd::Bottom => Self::Bottom,
            RangeEnd::Stopped { reached, bottom } => Self::Stopped { reached, bottom },
        }
    }
}

/// One frame: core's `RangeFrame` with the two fields the check needs.
#[derive(Serialize)]
pub(crate) struct Frame {
    #[serde(flatten)]
    core: RangeFrame,
    /// Absent for a bodyweight-only frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    hip: Option<HipLever>,
    ground_n: f64,
}

/// The `200` body of `POST /biomech/squat/range`.
#[derive(Serialize)]
pub(crate) struct RangeResponse {
    end: End,
    table_lengths: Vec<&'static str>,
    frames: Vec<Frame>,
    assumptions: Vec<Assumption>,
}

impl RangeResponse {
    /// Wrap a computed range, adding each frame's lever and ground force.
    pub(crate) fn from_range(squat: Squat, range: SquatRange) -> Self {
        let ground = ground_n(
            squat.bar.map(fitai_core::workout::LoadKg::get),
            squat.mass_kg,
        );
        let frames = range
            .frames
            .into_iter()
            .map(|frame| Frame {
                hip: hip_lever(&squat.body, &frame),
                ground_n: ground,
                core: frame,
            })
            .collect();
        Self {
            end: range.end.into(),
            table_lengths: squat.table_lengths,
            frames,
            assumptions: range.assumptions,
        }
    }

    /// The engine could not place the lifter: the typed reason, no frames.
    pub(crate) fn rejected(rejection: Rejection, table_lengths: Vec<&'static str>) -> Self {
        Self {
            end: End::Rejected { rejection },
            table_lengths,
            frames: Vec::new(),
            assumptions: Vec::new(),
        }
    }
}
