//! SPEC-0045 §2.8 — the result: four joint moments, what they were computed
//! under, and how much the stated conventions could be worth.
//!
//! **Why the entry point is named for its source.** `SolvedBaseline` and
//! `SinglePosture` are facts about how a posture was *made*, and §2.1's whole
//! point is that the model cannot see that: it takes a [`Posture`] in metres
//! and does not know whether a solver or a camera produced it. A single
//! source-agnostic `analyse` would therefore have to assert both — true today,
//! and a lie the day the R-0044 measured source feeds it. So [`mechanics`] is
//! private and each source names its own door: [`analyse_baseline`] now,
//! `analyse_measured` with the measured slice.
//!
//! The alternative — a `Provenance` argument — was rejected twice over: it
//! would let a caller mislabel a posture, and `Provenance` is already taken at
//! the crate root by `archetype::Provenance` (architect finding 10's
//! collision, again).

use serde::{Deserialize, Serialize};

use super::chain::{moment_about_with, Placement};
use super::range::{RangeAt, RangePosture};
use super::{Baseline, Body, Joint, Metres, NewtonMetres, Posture};

/// SPEC-0045 §2.8 — the L5/S1 placement spread, metres. A **stated
/// convention**: the §2.5 placement and the L4-L5 segment boundary are taken
/// as one point, and this is the room that mismatch is given.
pub const L5S1_PLACEMENT_SPREAD_M: f64 = 0.03;

/// SPEC-0045 §2.8 — the head & neck and arm COM placement spread, metres. A
/// **stated convention**, each applied along the line its §2.4.2 rule places
/// the point on.
pub const COM_PLACEMENT_SPREAD_M: f64 = 0.02;

/// The joints in the result's fixed order (SPEC-0045 §2.8, §5.3).
const JOINT_ORDER: [Joint; 4] = [Joint::Hip, Joint::Knee, Joint::Ankle, Joint::Lumbar];

/// What resists a joint's moment — the group, and nothing finer.
///
/// Named for the function rather than the anatomy: `fitai_core::MuscleGroup`
/// already exists with an anatomical meaning (`Chest | Back | …`), and this is
/// not that (architect finding 10). There is no `Muscle` type and none may be
/// added: splitting a joint moment across the muscles that cross the joint is
/// indeterminate without a musculoskeletal model — `r₁F₁ + … + rₙFₙ = τ` is
/// one equation in `n` unknowns, and naming more heads makes it *less*
/// determined, not more (R-0045 AC5; RFC-002 §7.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResistingGroup {
    /// Resists the hip moment.
    HipExtensors,
    /// Resists the knee moment.
    KneeExtensors,
    /// Resists the ankle moment.
    PlantarFlexors,
    /// Resists the L5/S1 moment.
    SpinalErectors,
}

/// Where in the lift the single analysed posture sits (SPEC-0045 §2.1, §2.6.1,
/// §2.6.6).
///
/// The baseline is at parallel, the depth standard R-0044 AC8 also uses; the
/// range adds its upright and the frames between. A unit enum on purpose: the
/// progress `s` is a coordinate of the range's convention and lives on the
/// frame, so [`Assumption`] keeps `Eq` and `Hash`. Depth becomes a knob in
/// slice B.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Position {
    /// Femur horizontal.
    Parallel,
    /// The solved straight-body upright at the start of the range (`s = 0`).
    Upright,
    /// Strictly between the upright and the bottom, under the §2.6.6 leg
    /// convention; the frame's own progress says where.
    Partial,
}

/// What the external load was, and whether there was one (R-0045 AC3).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "basis")]
pub enum LoadBasis {
    /// A bar of this mass.
    Bar {
        /// Kilograms.
        kg: f64,
    },
    /// No load was supplied, so none was assumed. The moments are the
    /// lifter's own weight, and the result says so rather than inventing a bar.
    BodyweightOnly,
}

/// Something the result is only true under. Non-empty on every result, and
/// carried *with* the numbers so a consumer cannot show one without the other
/// (R-0045 AC12).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "assumption")]
pub enum Assumption {
    /// Everything happens in one plane. A frontal-plane moment — an abducted
    /// femur's, say — is not computed and not implied.
    SagittalPlane,
    /// Every joint in moment equilibrium, no acceleration terms: exact at the
    /// sticking point, an understatement during fast phases (OQ-2).
    QuasiStatic,
    /// Segments are rigid. The trunk cannot be seen to round.
    RigidSegments,
    /// One posture, not a rep. There is no φ axis, no peak and no "per rep"
    /// in v1 — a descent kinematic nobody has validated would be fabricated.
    SinglePosture {
        /// Which posture.
        at: Position,
    },
    /// The posture was **solved** from this lifter's lengths under a stated
    /// convention — the §2.6.1 baseline at the bottom, the §2.6.6 range
    /// elsewhere — not a posture they were observed in.
    SolvedBaseline,
    /// The lumbar row treats the trunk as one rigid link through a single
    /// L5/S1 joint placed by convention (§2.5).
    RigidTrunkLumbar,
    /// Segment **masses** are table fractions of body mass. Unconditional:
    /// masses have no measurement path at all (§2.3).
    TableDerivedMasses,
    /// At least one segment **length** came from the table rather than a
    /// measurement.
    TableDerivedLengths,
    /// The stance knob's sagittal projection assumes the knee tracks over the
    /// foot in the frontal plane, and the abducted femur carries a
    /// frontal-plane hip moment this model does not compute (§2.6.2).
    ///
    /// **Unreachable in slice A**, like `Rejection::AnatomicallyImplausible`:
    /// there are no knobs yet, so no result can differ from the baseline
    /// stance. Present so the closed set does not change shape when they
    /// arrive.
    KneesTrackFeet,
    /// Body fat is not in the model: the cited table does not vary by it, and
    /// inventing an adjustment would be a constant with no source (§2.3).
    BodyFatIgnored,
    /// The range's upright is a straight body, knee and hip extended, leaning
    /// forward from the ankle until the system centre of mass is over the
    /// mid-foot: the ankle strategy of quiet standing (§2.6.6). Not observed.
    StraightBodyUpright,
    /// Between the upright and the bottom, the shank and the femur advance
    /// together by the §2.6.6 convention: each angle interpolated in degrees
    /// from the solved upright lean to the bottom (the lifter's dorsiflexion,
    /// the femur horizontal). Where a moment peaks across the range, and how
    /// far it reaches, hold under this convention only.
    JointsAdvanceTogether,
}

/// What the `±` on each joint moment was propagated from — the stated
/// placement conventions, so the bound can be audited instead of trusted
/// (SPEC-0045 §2.8).
///
/// The ±5% segment-length spread is deliberately **not** here. At a fixed
/// posture the joint centres are given, so a different femur length changes
/// nothing; its real effect is that the lifter's *posture* would be different,
/// which only a re-solve can show. That is the slice-B stability claim
/// (§2.6.5), and it belongs to the source rather than to the model.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Uncertainty {
    /// L5/S1 placement along the trunk line.
    pub l5s1_placement: Metres,
    /// Head & neck and arm COM placement, along the line each is placed on.
    pub com_placement: Metres,
}

/// One joint's share of the load.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct JointMoment {
    /// Which joint.
    pub joint: Joint,
    /// The group that resists it.
    pub resisting: ResistingGroup,
    /// N·m resisting extension, **signed**: negative means the load is
    /// assisting extension, which is a real state and never flattened.
    pub moment: NewtonMetres,
    /// Half-width from [`Uncertainty`]'s conventions — a bound from stated
    /// inputs, not a confidence interval. No distribution is assumed.
    pub uncertainty: NewtonMetres,
}

/// The analysis of one posture: hip, knee, ankle and lumbar, what the load
/// was, and everything the numbers are only true under.
///
/// Nothing here says *risk*, *danger*, *safe* or *injury*, and no field may be
/// added that does (R-0045 AC14). The result describes how a load is
/// distributed; what to do about it is coaching, not mechanics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mechanics {
    /// Hip, knee, ankle, lumbar — fixed order.
    pub joints: [JointMoment; 4],
    /// The load basis, which never invents a bar.
    pub load: LoadBasis,
    /// Non-empty, and it varies: `TableDerivedLengths` is conditional.
    pub assumptions: Vec<Assumption>,
    /// The spreads the per-joint `uncertainty` came from.
    pub uncertainty: Uncertainty,
}

/// Analyse a posture produced by the v1 parametric solve (SPEC-0045 §2.6.1).
///
/// Total: a posture that exists has moments. Every rejection happens in the
/// solve, before there is a posture to analyse.
///
/// It takes a [`Baseline`] rather than a [`Posture`] so the two source
/// assumptions below cannot be asserted about a posture that was measured or
/// hand-built — only `squat::baseline` can make one.
#[must_use]
pub fn analyse_baseline(body: &Body, solved: &Baseline) -> Mechanics {
    mechanics(
        body,
        solved.posture(),
        &[
            Assumption::SinglePosture {
                at: Position::Parallel,
            },
            Assumption::SolvedBaseline,
        ],
    )
}

/// Analyse a posture the §2.6.6 range solved: the upright or a frame between
/// it and the bottom. The bottom itself goes through [`analyse_baseline`].
///
/// Takes a [`RangePosture`], which only the range can build, for the same
/// reason [`analyse_baseline`] takes a [`Baseline`]: the assumptions below are
/// claims about where the posture came from.
pub(super) fn analyse_range(body: &Body, solved: &RangePosture) -> Mechanics {
    let from_source = match solved.at() {
        RangeAt::Upright => [
            Assumption::SinglePosture {
                at: Position::Upright,
            },
            Assumption::SolvedBaseline,
            Assumption::StraightBodyUpright,
        ],
        RangeAt::Partial => [
            Assumption::SinglePosture {
                at: Position::Partial,
            },
            Assumption::SolvedBaseline,
            Assumption::JointsAdvanceTogether,
        ],
    };
    mechanics(body, solved.posture(), &from_source)
}

/// The model, shared by every source. Private: `from_source` is the half a
/// [`Posture`] cannot carry, and only the source knows it.
fn mechanics(body: &Body, p: &Posture, from_source: &[Assumption]) -> Mechanics {
    let mut assumptions = vec![
        Assumption::SagittalPlane,
        Assumption::QuasiStatic,
        Assumption::RigidSegments,
    ];
    assumptions.extend_from_slice(from_source);
    // v1's `joints` is a fixed four, so this is always emitted — but it is
    // written against the row's presence, because the assumption is about the
    // row and not about the lift.
    if JOINT_ORDER.contains(&Joint::Lumbar) {
        assumptions.push(Assumption::RigidTrunkLumbar);
    }
    assumptions.push(Assumption::TableDerivedMasses);
    if body.any_length_table_derived() {
        assumptions.push(Assumption::TableDerivedLengths);
    }
    assumptions.push(Assumption::BodyFatIgnored);

    Mechanics {
        joints: JOINT_ORDER.map(|joint| JointMoment {
            joint,
            resisting: joint.resisting(),
            moment: moment_about_with(joint, body, p, Placement::STATED),
            uncertainty: placement_half_width(joint, body, p),
        }),
        load: p
            .load
            .map_or(LoadBasis::BodyweightOnly, |l| LoadBasis::Bar {
                kg: l.kg.get(),
            }),
        assumptions,
        uncertainty: Uncertainty {
            l5s1_placement: Metres(L5S1_PLACEMENT_SPREAD_M),
            com_placement: Metres(COM_PLACEMENT_SPREAD_M),
        },
    }
}

/// The largest the stated placement conventions could move this joint's
/// moment: the moment evaluated at all eight corners of the three spreads,
/// against the stated placement (SPEC-0045 §2.8).
///
/// Eight corners rather than a gradient because the bound is meant to be a
/// bound — the moment is affine in each offset, so the extreme is at a corner,
/// and walking them needs no claim about which way each one leans.
fn placement_half_width(joint: Joint, body: &Body, p: &Posture) -> NewtonMetres {
    let stated = moment_about_with(joint, body, p, Placement::STATED).0;
    let mut worst: f64 = 0.0;
    for head in [-COM_PLACEMENT_SPREAD_M, COM_PLACEMENT_SPREAD_M] {
        for arms in [-COM_PLACEMENT_SPREAD_M, COM_PLACEMENT_SPREAD_M] {
            for l5s1 in [-L5S1_PLACEMENT_SPREAD_M, L5S1_PLACEMENT_SPREAD_M] {
                let corner = Placement { head, arms, l5s1 };
                let deviation = (moment_about_with(joint, body, p, corner).0 - stated).abs();
                worst = worst.max(deviation);
            }
        }
    }
    NewtonMetres(worst)
}
