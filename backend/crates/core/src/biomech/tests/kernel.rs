//! SPEC-0066 slices 66-a1 and 66-a2 — the kernel glue, and the squat on it.
//!
//! Slice 66-a1 proved parity with the hand-written chain while both paths
//! were alive, to 1e-9 N·m on 1 916 postures and every placement corner, and
//! snapshotted the kernel's values in a generated fixture. Slice 66-a2
//! removed the chain's moment functions; the fixture is now the parity gate
//! (R-0045 AC17(c)). It is never regenerated: both paths are gone.

use std::path::PathBuf;

use serde::Deserialize;

use super::{bar, dorsi, pt, solved};
use crate::biomech::kernel::{agree, moment_of, moments, KernelError, PlanarTree, Side};
use crate::biomech::segments::Placement;
use crate::biomech::squat::{tree_with, Loads};
use crate::biomech::{
    body_moment, scale, squat_range, Body, FrameCount, Joint, MeasuredLengths, Metres,
    NewtonMetres, Posture, ResistingGroup,
};

/// The parity bound (R-0045 AC17(c)).
const PARITY_N_M: f64 = 1e-9;

/// The joints in the order the squat tree reports them.
const JOINTS: [Joint; 4] = [Joint::Ankle, Joint::Knee, Joint::Hip, Joint::Lumbar];

/// The squat's tree at the stated placement, bar included.
fn squat_tree(lifter: &Body, p: &Posture) -> PlanarTree {
    tree_with(lifter, p, Placement::STATED, Loads::All)
}

/// The squat tree's four moments in [`JOINTS`] order, computed once.
fn squat_moments(tree: &PlanarTree) -> [f64; 4] {
    let all = moments(tree).unwrap_or_else(|e| panic!("kernel refused: {e}"));
    JOINTS.map(|joint| {
        all.iter().find(|(j, _)| *j == joint).map_or_else(
            || panic!("{joint:?} not reported"),
            |&(_, NewtonMetres(m))| m,
        )
    })
}

fn kernel_moment(tree: &PlanarTree, joint: Joint) -> f64 {
    let all = moments(tree).unwrap_or_else(|e| panic!("kernel refused: {e}"));
    all.iter().find(|(j, _)| *j == joint).map_or_else(
        || panic!("{joint:?} not reported"),
        |&(_, NewtonMetres(m))| m,
    )
}

// ---------------------------------------------------------------------------
// The glue on its own
// ---------------------------------------------------------------------------

/// One weight on one link: `τ = m·g·d`, the arithmetic beside the literal.
/// A thigh held horizontal with 10 kg 0.30 m forward of the hip: gravity
/// pulls it down, toward hip extension, so the moment is negative and the hip
/// flexors resist. `10 · 9.81 · 0.30 = 29.43`.
#[test]
fn one_weight_on_one_link_is_its_weight_times_its_lever() {
    let mut tree = PlanarTree::new();
    let hip = tree.link(None, pt(0.0, 1.0));
    tree.report(hip, Joint::Hip, Side::Anatomical);
    tree.weight(hip, pt(0.30, 1.0), 10.0);
    assert!((kernel_moment(&tree, Joint::Hip) - (-29.43)).abs() < 1e-9);
}

/// A weight straight above or below the joint has no lever.
#[test]
fn a_weight_on_the_joint_s_vertical_has_no_moment() {
    let mut tree = PlanarTree::new();
    let knee = tree.link(None, pt(0.2, 0.5));
    tree.report(knee, Joint::Knee, Side::Anatomical);
    tree.weight(knee, pt(0.2, 0.1), 50.0);
    assert!(kernel_moment(&tree, Joint::Knee).abs() < 1e-9);
}

/// A load on a link that is not distal to the joint is carried elsewhere.
#[test]
fn a_load_proximal_to_the_joint_is_not_in_its_moment() {
    let mut tree = PlanarTree::new();
    let hip = tree.link(None, pt(0.0, 1.0));
    let knee = tree.link(Some(hip), pt(0.4, 1.0));
    tree.report(knee, Joint::Knee, Side::Anatomical);
    tree.weight(hip, pt(0.2, 1.0), 30.0);
    assert!(kernel_moment(&tree, Joint::Knee).abs() < 1e-9);
}

#[test]
fn an_empty_tree_has_no_moments() {
    assert_eq!(moments(&PlanarTree::new()), Ok(Vec::new()));
}

/// The cross-check's tolerance: `1e-4 + 1e-9·|τ|` (SPEC-0066 §2.1.2).
#[test]
fn the_cross_check_tolerance_is_absolute_plus_relative() {
    assert!(agree(100.0, 100.0 + 0.9e-4));
    assert!(!agree(100.0, 100.0 + 1.2e-4));
    // At 10 kN·m the relative term adds 1e-5.
    assert!(agree(1e4, 1e4 + 1.05e-4));
    assert!(!agree(1e4, 1e4 + 1.2e-4));
}

#[test]
fn a_disagreement_names_its_joint() {
    let e = KernelError::Disagree {
        joint: Joint::Knee,
        cross: 1.0,
        virtual_work: 2.0,
    };
    assert!(e.to_string().contains("Knee"));
}

// ---------------------------------------------------------------------------
// The squat tree
// ---------------------------------------------------------------------------

fn body(height_m: f64, mass_kg: f64) -> Body {
    scale(Metres(height_m), mass_kg, &MeasuredLengths::default())
}

/// The FK invariant: garust places every joint where the posture has it
/// (the spike's check was 6.7e-16 m).
#[test]
fn the_tree_puts_every_joint_where_the_posture_has_it() {
    let lifter = body(1.75, 80.0);
    let p = solved(&lifter, Some(100.0), 38.0);
    for (joint, at) in squat_tree(&lifter, &p).joint_centres() {
        let want = p.at(joint);
        let off = (at.x.0 - want.x.0).hypot(at.y.0 - want.y.0);
        assert!(off < 1e-12, "{joint:?} is {off} m off");
    }
}

/// Every squat joint's sign, by inspection (SPEC-0066 §2.1.3): at a loaded
/// parallel squat the bar and trunk sit ahead of the hip and L5/S1 and
/// behind the knee, and the system balances over the mid-foot, ahead of the
/// ankle. So the hip extensors, knee extensors, plantar flexors and erectors
/// all resist: every moment is positive.
#[test]
fn every_squat_joint_has_the_sign_its_resisting_group_says() {
    let lifter = body(1.75, 80.0);
    let p = solved(&lifter, Some(100.0), 38.0);
    let all = squat_moments(&squat_tree(&lifter, &p));
    for (joint, m) in JOINTS.iter().zip(all) {
        assert!(m > 0.0, "{joint:?}: {m}");
    }
}

/// The cross-check holds at the heaviest bar the squat accepts
/// (`LoadKg::MAX`, 1000 kg), where its relative term matters most
/// (SPEC-0066 §2.1.2): every frame of that range is served, none refused.
#[test]
fn the_cross_check_holds_at_the_heaviest_bar() {
    let case = Case {
        height_m: 1.75,
        mass_kg: 80.0,
        bar_kg: Some(1000.0),
        dorsiflexion_deg: 38.0,
        frames: Vec::new(),
    };
    let lifter = case.body();
    let postures = case.postures();
    assert!(!postures.is_empty(), "a 1000 kg bar still reaches depth");
    for p in postures {
        assert!(moments(&squat_tree(&lifter, &p)).is_ok());
    }
}

/// The parity grid: the spike's four bodies, four bars and three
/// dorsiflexions, every frame of each range.
fn grid() -> Vec<Case> {
    let mut cases = Vec::new();
    for &(height_m, mass_kg) in &[(1.60, 60.0), (1.75, 80.0), (1.85, 90.0), (1.95, 105.0)] {
        for bar_kg in [None, Some(60.0), Some(140.0), Some(250.0)] {
            for dorsiflexion_deg in [30.0, 38.0, 45.0] {
                cases.push(Case {
                    height_m,
                    mass_kg,
                    bar_kg,
                    dorsiflexion_deg,
                    frames: Vec::new(),
                });
            }
        }
    }
    cases
}

/// One grid case and, in the fixture, its frames' four moments in
/// [`JOINTS`] order.
#[derive(Debug, Deserialize)]
struct Case {
    height_m: f64,
    mass_kg: f64,
    bar_kg: Option<f64>,
    dorsiflexion_deg: f64,
    frames: Vec<[f64; 4]>,
}

impl Case {
    fn body(&self) -> Body {
        body(self.height_m, self.mass_kg)
    }

    /// The range's postures; empty when the lifter cannot reach depth.
    fn postures(&self) -> Vec<Posture> {
        let frames = FrameCount::new(41).unwrap_or_else(|| panic!("41 is a valid frame count"));
        squat_range(
            &self.body(),
            self.bar_kg.map(bar),
            dorsi(self.dorsiflexion_deg),
            frames,
        )
        .map(|r| r.frames.into_iter().map(|f| f.posture).collect())
        .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// The fixture that survives chain.rs (SPEC-0066 §2.2)
// ---------------------------------------------------------------------------

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/biomech/tests/fixtures/squat_kernel_parity.json")
}

fn kernel_frames(case: &Case) -> Vec<[f64; 4]> {
    let lifter = case.body();
    case.postures()
        .iter()
        .map(|p| squat_moments(&squat_tree(&lifter, p)))
        .collect()
}

#[test]
fn the_kernel_still_matches_the_parity_fixture() {
    let json = std::fs::read_to_string(fixture_path())
        .unwrap_or_else(|e| panic!("the fixture is missing ({e})"));
    let cases: Vec<Case> = serde_json::from_str(&json).unwrap_or_else(|e| panic!("parse: {e}"));
    assert_eq!(
        cases.len(),
        grid().len(),
        "the fixture covers the whole grid"
    );
    for case in &cases {
        let now = kernel_frames(case);
        assert_eq!(now.len(), case.frames.len(), "{case:?}: frame count");
        for (got, want) in now.iter().zip(&case.frames) {
            for (i, joint) in JOINTS.iter().enumerate() {
                assert!(
                    (got[i] - want[i]).abs() <= PARITY_N_M,
                    "{joint:?}: kernel {}, fixture {}",
                    got[i],
                    want[i]
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Slice 66-a2: one sign rule, the body's share, typed errors
// ---------------------------------------------------------------------------

/// `resisting` follows the sign (SPEC-0066 §2.1.3): positive, the loads tend
/// to flex the joint and its extensors resist; negative, its flexors do.
#[test]
fn the_resisting_group_follows_the_sign_of_the_moment() {
    let cases = [
        (
            Joint::Hip,
            ResistingGroup::HipExtensors,
            ResistingGroup::HipFlexors,
        ),
        (
            Joint::Knee,
            ResistingGroup::KneeExtensors,
            ResistingGroup::KneeFlexors,
        ),
        (
            Joint::Ankle,
            ResistingGroup::PlantarFlexors,
            ResistingGroup::Dorsiflexors,
        ),
        (
            Joint::Lumbar,
            ResistingGroup::SpinalErectors,
            ResistingGroup::Abdominals,
        ),
    ];
    for (joint, positive, negative) in cases {
        assert_eq!(joint.resisting(NewtonMetres(12.0)), positive, "{joint:?} +");
        assert_eq!(
            joint.resisting(NewtonMetres(-12.0)),
            negative,
            "{joint:?} −"
        );
    }
}

/// The SPEC-0059 example's upright frame has the knee moment negative (the
/// load sits ahead of the knee), so it names the knee flexors.
#[test]
fn an_upright_frame_with_a_negative_knee_moment_names_the_knee_flexors() {
    let body = scale(Metres(1.78), 77.0, &MeasuredLengths::default());
    let frames = FrameCount::new(41).unwrap_or_else(|| panic!("41 is valid"));
    let range = squat_range(&body, Some(bar(84.0)), dorsi(29.0), frames)
        .unwrap_or_else(|e| panic!("the example solves: {e:?}"));
    let knee = range.frames[0]
        .mechanics
        .joints
        .iter()
        .find(|j| j.joint == Joint::Knee)
        .unwrap_or_else(|| panic!("the knee is reported"));
    assert!(knee.moment.0 < 0.0, "{}", knee.moment.0);
    assert_eq!(knee.resisting, ResistingGroup::KneeFlexors);
}

/// The body's own share plus the bar's term is the joint's moment
/// (SPEC-0059 AC7, now from the kernel): the same tree without the bar.
#[test]
fn the_body_share_plus_the_bar_term_is_the_moment() {
    let lifter = body(1.75, 80.0);
    let p = solved(&lifter, Some(100.0), 38.0);
    let all = squat_moments(&squat_tree(&lifter, &p));
    let load = p.load.unwrap_or_else(|| panic!("a loaded posture"));
    for (joint, total) in JOINTS.into_iter().zip(all) {
        let share = body_moment(joint, &lifter, &p)
            .unwrap_or_else(|e| panic!("kernel refused: {e}"))
            .0;
        assert!((share - total).abs() > 1.0, "{joint:?}: the bar matters");
        let bar_only = total - share;
        // The bar's term by hand: m·g·|lever| (its sign is the joint's).
        let lever = load.at.x.0 - p.at(joint).x.0;
        let by_hand = load.kg.get() * 9.81 * lever.abs();
        assert!(
            (bar_only.abs() - by_hand).abs() < 1e-9,
            "{joint:?}: {bar_only} vs {by_hand}"
        );
    }
}

/// A joint the tree does not report is a typed error, never a silent zero.
#[test]
fn a_joint_the_tree_does_not_report_is_a_typed_error() {
    assert_eq!(
        moment_of(&[], Joint::Hip),
        Err(KernelError::NotReported { joint: Joint::Hip })
    );
}
