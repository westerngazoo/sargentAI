//! SPEC-0066 slice 66-a1 — the kernel glue, and the squat on it with the
//! hand-written chain still alive as the parity reference (§2.2).
//!
//! The parity claim is the gate for moving the squat onto garust: every
//! posture of a dense grid, and every placement corner of a subset, has the
//! same four moments on the kernel as on `chain.rs`, to 1e-9 N·m (R-0045
//! AC17(c)). The fixture written from a green run is what survives `chain.rs`
//! in slice 66-a2.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{bar, dorsi, pt, solved};
use crate::biomech::chain::{moment_about_with, Placement};
use crate::biomech::kernel::{agree, moments, KernelError, PlanarTree, Side};
use crate::biomech::squat::tree_with;
use crate::biomech::{
    moment_about, scale, squat_range, squat_tree, Body, FrameCount, Joint, MeasuredLengths, Metres,
    NewtonMetres, Posture, COM_PLACEMENT_SPREAD_M, L5S1_PLACEMENT_SPREAD_M,
};

/// The parity bound (R-0045 AC17(c)).
const PARITY_N_M: f64 = 1e-9;

/// The joints in the order the squat tree reports them.
const JOINTS: [Joint; 4] = [Joint::Ankle, Joint::Knee, Joint::Hip, Joint::Lumbar];

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
#[derive(Debug, Serialize, Deserialize)]
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

#[test]
fn every_grid_posture_has_the_same_moments_on_the_kernel_as_on_the_chain() {
    let mut postures = 0;
    for case in grid() {
        let lifter = case.body();
        for p in case.postures() {
            let kernel_all = squat_moments(&squat_tree(&lifter, &p));
            for (joint, kernel) in JOINTS.into_iter().zip(kernel_all) {
                let chain = moment_about(joint, &lifter, &p).0;
                assert!(
                    (kernel - chain).abs() <= PARITY_N_M,
                    "{case:?} {joint:?}: kernel {kernel}, chain {chain}"
                );
            }
            postures += 1;
        }
    }
    assert_eq!(postures, 1916, "the grid's posture count (SPEC-0066 §2.2)");
}

/// Every placement corner the uncertainty walk evaluates (SPEC-0045 §2.8),
/// on one lifter's whole range.
#[test]
fn every_placement_corner_has_the_same_moments_on_the_kernel_as_on_the_chain() {
    let case = Case {
        height_m: 1.75,
        mass_kg: 80.0,
        bar_kg: Some(140.0),
        dorsiflexion_deg: 38.0,
        frames: Vec::new(),
    };
    let lifter = case.body();
    let com = [-COM_PLACEMENT_SPREAD_M, COM_PLACEMENT_SPREAD_M];
    let l5s1 = [-L5S1_PLACEMENT_SPREAD_M, L5S1_PLACEMENT_SPREAD_M];
    for p in case.postures() {
        for head in com {
            for arms in com {
                for l in l5s1 {
                    let corner = Placement {
                        head,
                        arms,
                        l5s1: l,
                    };
                    let kernel_all = squat_moments(&tree_with(&lifter, &p, corner));
                    for (joint, kernel) in JOINTS.into_iter().zip(kernel_all) {
                        let chain = moment_about_with(joint, &lifter, &p, corner).0;
                        assert!(
                            (kernel - chain).abs() <= PARITY_N_M,
                            "{corner:?} {joint:?}: kernel {kernel}, chain {chain}"
                        );
                    }
                }
            }
        }
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

/// Writes the fixture. **Generated; regenerate only with both paths alive**
/// (SPEC-0066 §2.2), i.e. never after slice 66-a2. Run with
/// `cargo test -p fitai-core write_the_squat_kernel_parity_fixture -- --ignored`.
#[test]
#[ignore = "writes the generated parity fixture; run by hand, only in slice 66-a1"]
fn write_the_squat_kernel_parity_fixture() {
    let cases: Vec<Case> = grid()
        .into_iter()
        .map(|mut case| {
            case.frames = kernel_frames(&case);
            case
        })
        .collect();
    let json = serde_json::to_string(&cases).unwrap_or_else(|e| panic!("serialize: {e}"));
    std::fs::write(fixture_path(), json).unwrap_or_else(|e| panic!("write: {e}"));
}

#[test]
fn the_kernel_still_matches_the_parity_fixture() {
    let json = std::fs::read_to_string(fixture_path())
        .unwrap_or_else(|e| panic!("the fixture is missing ({e}); write it in 66-a1"));
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
