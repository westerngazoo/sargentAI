//! R-0045 slice A claims (SPEC-0045 §7: B0–B15 less the slice-B B7/B8/B12)
//! — **no video, no database, no model**: a scaled body, a posture in metres,
//! and moments a reader can check by hand.
//!
//! These tests were written **before** the model they exercise (constitution
//! §4): the red state is the `todo!()` in each submodule. The B0 table claims
//! are the exception by design — they check constants and must hold whatever
//! the values are (SPEC-0045 §2.3).
//!
//! Every hand computation below re-derives the §2.4.2 placement conventions
//! independently of `chain.rs`: masses from [`WINTER`] fractions, levers from
//! the posture's own geometry. Two readings the spec leaves implicit are made
//! explicit in [`hand_com`]: the posture's `shoulder` stands for C7-T1 (the
//! head convention already extends the trunk line from it), and the L4-L5
//! segment boundary is the L5/S1 point of §2.5.
//!
//! B13, B14 and the `LoadBasis` half of B5 read a `Mechanics` result; §3
//! names no function that produces one, so they are written once it does.

#![allow(clippy::doc_markdown)]

use crate::biomech::ChainSegment::{Arms, HeadNeck, Pelvis, Shank, Thigh, ThoraxAbdomen};
use crate::biomech::{
    above, baseline, moment_about, scale, Body, ChainSegment, Joint, Load, MeasuredLengths, Metres,
    Point, Posture, Rejection, Segment, Unknown, Verification, WINTER, WINTER_VERIFICATION,
};

// ===========================================================================
// Fixture conventions
// ===========================================================================

/// `g`, as SPEC-0045 §2.2 fixes it.
const G: f64 = 9.81;

/// The fixture lifter: table proportions at 1.75 m, 80 kg — the body
/// SPEC-0045 §2.4.3 scales the reel against.
const HEIGHT_M: f64 = 1.75;
const MASS_KG: f64 = 80.0;

/// SPEC-0045 §2.5: L5/S1 sits this far up the trunk line from the hip. A
/// stated convention pending citation; the L4-L5 segment boundary is taken as
/// the same point, the mismatch absorbed into the stated error.
const L5S1_UP_THE_TRUNK: f64 = 0.10;

/// SPEC-0045 §2.6.1: the bar attaches this far up the trunk line (high-bar).
const HIGH_BAR: f64 = 0.95;

/// SPEC-0045 §2.6.1: the ankle sits this fraction of a foot length behind the
/// mid-foot line — the ankle at the heel-side quarter, mid-foot at half.
const ANKLE_BEHIND_MID_FOOT: f64 = 0.26;

/// The four joints, in the result's fixed order.
const JOINTS: [Joint; 4] = [Joint::Hip, Joint::Knee, Joint::Ankle, Joint::Lumbar];

// ===========================================================================
// Scenario builders
// ===========================================================================

fn pt(x: f64, y: f64) -> Point {
    Point {
        x: Metres(x),
        y: Metres(y),
    }
}

/// The point `f` of the way from `a` to `b`.
fn along(a: Point, b: Point, f: f64) -> Point {
    pt(a.x.0 + f * (b.x.0 - a.x.0), a.y.0 + f * (b.y.0 - a.y.0))
}

/// A table length at the fixture height (Figure 4.1).
fn table_length(row: Segment) -> f64 {
    row.length_fraction_of_height.expect("a row with a length") * HEIGHT_M
}

/// Segment lengths a posture is built from, metres.
#[derive(Clone, Copy)]
struct Lengths {
    shank: f64,
    thigh: f64,
    trunk: f64,
}

impl Lengths {
    /// Table proportions at the fixture height.
    fn table() -> Self {
        Self {
            shank: table_length(WINTER.shank),
            thigh: table_length(WINTER.thigh),
            trunk: table_length(WINTER.trunk),
        }
    }

    /// MECÁNICA 01's lifter (SPEC-0045 §2.4.3).
    fn reel() -> Self {
        Self {
            shank: 0.43,
            thigh: 0.40,
            trunk: 0.53,
        }
    }

    /// The same lengths as measured overrides.
    fn measured(self) -> MeasuredLengths {
        MeasuredLengths {
            thigh: Some(Metres(self.thigh)),
            shank: Some(Metres(self.shank)),
            trunk: Some(Metres(self.trunk)),
        }
    }
}

/// The fixture lifter, every length from the table.
fn lifter() -> Body {
    scale(Metres(HEIGHT_M), MASS_KG, &MeasuredLengths::default())
}

/// The §2.6.1 ankle for the fixture height: `(−0.26 × foot, 0.039 H)`.
fn table_ankle() -> Point {
    pt(
        -ANKLE_BEHIND_MID_FOOT * table_length(WINTER.foot),
        WINTER.ankle_height.fraction_of_height * HEIGHT_M,
    )
}

/// A bottom-position chain built by hand from the §2.6.1 conventions: `ankle`
/// placed, the shank `shank_deg` forward of vertical, the thigh parallel with
/// the hip behind the knee, the trunk `trunk_deg` forward of vertical, and the
/// bar — if any — at [`HIGH_BAR`] along the trunk line.
fn bottom(
    lengths: Lengths,
    ankle: Point,
    shank_deg: f64,
    trunk_deg: f64,
    bar_kg: Option<f64>,
) -> Posture {
    let (sin_shank, cos_shank) = shank_deg.to_radians().sin_cos();
    let (sin_trunk, cos_trunk) = trunk_deg.to_radians().sin_cos();
    let knee = pt(
        ankle.x.0 + lengths.shank * sin_shank,
        ankle.y.0 + lengths.shank * cos_shank,
    );
    let hip = pt(knee.x.0 - lengths.thigh, knee.y.0);
    let shoulder = pt(
        hip.x.0 + lengths.trunk * sin_trunk,
        hip.y.0 + lengths.trunk * cos_trunk,
    );
    let load = bar_kg.map(|kg| Load {
        kg,
        at: along(hip, shoulder, HIGH_BAR),
    });
    Posture {
        ankle,
        knee,
        hip,
        shoulder,
        load,
    }
}

/// The B2 posture: table lengths, the shank and the trunk both 30° forward of
/// vertical.
fn thirty_degrees(bar_kg: Option<f64>) -> Posture {
    bottom(Lengths::table(), table_ankle(), 30.0, 30.0, bar_kg)
}

/// `p` reflected in the mid-foot line: the lifter facing the other way.
fn mirrored(p: Posture) -> Posture {
    let flip = |q: Point| pt(-q.x.0, q.y.0);
    Posture {
        ankle: flip(p.ankle),
        knee: flip(p.knee),
        hip: flip(p.hip),
        shoulder: flip(p.shoulder),
        load: p.load.map(|l| Load {
            kg: l.kg,
            at: flip(l.at),
        }),
    }
}

/// A solve that must succeed, naming the rejection if it does not.
fn solved(body: &Body, bar_kg: Option<f64>) -> Posture {
    baseline(body, bar_kg).unwrap_or_else(|r| panic!("baseline (bar {bar_kg:?}) rejected: {r:?}"))
}

// ===========================================================================
// The model by hand — §2.3 masses, §2.4.2 placement, §2.4.1 raw sum
// ===========================================================================

/// Mass of a lumped segment: table fraction × the fixture mass, both legs and
/// both arms.
fn hand_mass(seg: ChainSegment) -> f64 {
    let fraction = match seg {
        Shank => 2.0 * WINTER.shank.mass_fraction,
        Thigh => 2.0 * WINTER.thigh.mass_fraction,
        Pelvis => WINTER.pelvis.mass_fraction,
        ThoraxAbdomen => WINTER.thorax_abdomen.mass_fraction,
        HeadNeck => WINTER.head_neck.mass_fraction,
        Arms => {
            2.0 * (WINTER.upper_arm.mass_fraction
                + WINTER.forearm.mass_fraction
                + WINTER.hand.mass_fraction)
        }
    };
    fraction * MASS_KG
}

/// Where a lumped segment's COM sits, by the §2.4.2 conventions.
fn hand_com(seg: ChainSegment, p: &Posture) -> Point {
    let l4l5 = along(p.hip, p.shoulder, L5S1_UP_THE_TRUNK);
    match seg {
        Shank => along(p.knee, p.ankle, WINTER.shank.com_from_proximal),
        Thigh => along(p.hip, p.knee, WINTER.thigh.com_from_proximal),
        Pelvis => along(l4l5, p.hip, WINTER.pelvis.com_from_proximal),
        ThoraxAbdomen => along(p.shoulder, l4l5, WINTER.thorax_abdomen.com_from_proximal),
        HeadNeck => {
            let trunk = (p.shoulder.x.0 - p.hip.x.0).hypot(p.shoulder.y.0 - p.hip.y.0);
            let beyond = WINTER.head_neck.com_from_proximal * table_length(WINTER.head_neck);
            along(p.hip, p.shoulder, 1.0 + beyond / trunk)
        }
        Arms => match p.load {
            Some(load) => along(p.shoulder, load.at, 0.5),
            None => pt(
                p.shoulder.x.0,
                p.shoulder.y.0
                    - WINTER.upper_arm.com_from_proximal * table_length(WINTER.upper_arm),
            ),
        },
    }
}

/// `Σ m·g·(x_COM − x_joint)` over `segments`, plus the load if any: the raw
/// coordinate sum of §2.4.1, before the per-joint sign.
fn hand_raw_sum(p: &Posture, joint_x: f64, segments: &[ChainSegment]) -> f64 {
    let segments: f64 = segments
        .iter()
        .map(|&s| hand_mass(s) * G * (hand_com(s, p).x.0 - joint_x))
        .sum();
    let load = p.load.map_or(0.0, |l| l.kg * G * (l.at.x.0 - joint_x));
    segments + load
}

/// The model's own system COM `x`: every segment above the ankle plus the
/// load, weighted by the model's masses and placed by the model's conventions.
fn system_com_x(body: &Body, p: &Posture) -> f64 {
    let (mut first_moment, mut mass) = (0.0, 0.0);
    for &s in above(Joint::Ankle) {
        let m = body.mass_kg(s);
        first_moment += m * s.com(body, p).x.0;
        mass += m;
    }
    if let Some(l) = p.load {
        first_moment += l.kg * l.at.x.0;
        mass += l.kg;
    }
    first_moment / mass
}

fn assert_close(actual: f64, expected: f64, tol: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= tol,
        "{what}: got {actual}, expected {expected} (±{tol})"
    );
}

fn assert_point_close(actual: Point, expected: Point, tol: f64, what: &str) {
    assert_close(actual.x.0, expected.x.0, tol, &format!("{what} x"));
    assert_close(actual.y.0, expected.y.0, tol, &format!("{what} y"));
}

// ===========================================================================
// B0 · The table (AC2)
// ===========================================================================

/// B0 — `HAT + 2·(thigh + shank + foot) = 1.000`: the table accounts for the
/// whole body. Holds whatever the values are (AC2).
#[test]
fn b0_hat_and_two_legs_sum_to_the_whole_body() {
    let leg = WINTER.thigh.mass_fraction + WINTER.shank.mass_fraction + WINTER.foot.mass_fraction;
    let whole = WINTER.hat.mass_fraction + 2.0 * leg;
    assert_close(whole, 1.0, 1e-9, "HAT + 2·(thigh + shank + foot)");
}

/// B0 — `trunk + head&neck + 2·(upper arm + forearm + hand) = HAT` (AC2).
#[test]
fn b0_trunk_head_and_arms_sum_to_hat() {
    let arm =
        WINTER.upper_arm.mass_fraction + WINTER.forearm.mass_fraction + WINTER.hand.mass_fraction;
    let hat = WINTER.trunk.mass_fraction + WINTER.head_neck.mass_fraction + 2.0 * arm;
    assert_close(
        hat,
        WINTER.hat.mass_fraction,
        1e-9,
        "trunk + head&neck + 2·arm",
    );
}

/// B0 — `thorax&abdomen + pelvis = trunk`: the split the lumbar row needs
/// loses no mass (AC2; SPEC-0045 §2.5).
#[test]
fn b0_thorax_abdomen_and_pelvis_sum_to_the_trunk() {
    let split = WINTER.thorax_abdomen.mass_fraction + WINTER.pelvis.mass_fraction;
    assert_close(
        split,
        WINTER.trunk.mass_fraction,
        1e-9,
        "thorax&abdomen + pelvis",
    );
}

/// B0 — every constant in `WINTER` carries a non-empty citation (AC2: the
/// table and its source are cited; no invented constants).
#[test]
fn b0_every_winter_constant_carries_a_citation() {
    let rows = [
        ("foot", WINTER.foot),
        ("shank", WINTER.shank),
        ("thigh", WINTER.thigh),
        ("pelvis", WINTER.pelvis),
        ("thorax & abdomen", WINTER.thorax_abdomen),
        ("trunk", WINTER.trunk),
        ("head & neck", WINTER.head_neck),
        ("upper arm", WINTER.upper_arm),
        ("forearm", WINTER.forearm),
        ("hand", WINTER.hand),
        ("HAT", WINTER.hat),
    ];
    for (name, row) in rows {
        assert!(!row.citation.trim().is_empty(), "{name} has no citation");
    }
    let lengths = [
        ("hip breadth", WINTER.hip_breadth),
        ("shoulder breadth", WINTER.shoulder_breadth),
        ("ankle height", WINTER.ankle_height),
    ];
    for (name, length) in lengths {
        assert!(!length.citation.trim().is_empty(), "{name} has no citation");
    }
}

/// AC2 (SPEC-0045 §6) — the verification record is filled in before the
/// constants ship: edition, pages, date, who. Red until someone opens the
/// book; a recollection is not the control.
#[test]
fn winter_verification_record_is_filled_in() {
    assert!(
        matches!(WINTER_VERIFICATION, Verification::Verified { .. }),
        "WINTER is {WINTER_VERIFICATION:?}: check Table 4.1 / Figure 4.1 against the printed edition and record it"
    );
}

/// SPEC-0045 §2.3 (AC2) — masses are table fractions of body mass and have no
/// measurement path: a body with measured lengths weighs the same as one
/// without.
#[test]
fn masses_are_table_fractions_and_ignore_measured_lengths() {
    let table = lifter();
    let measured = scale(Metres(HEIGHT_M), MASS_KG, &Lengths::reel().measured());
    for seg in [Shank, Thigh, Pelvis, ThoraxAbdomen, HeadNeck, Arms] {
        assert_close(
            table.mass_kg(seg),
            hand_mass(seg),
            1e-9,
            &format!("{seg:?} mass, table lengths"),
        );
        assert_close(
            measured.mass_kg(seg),
            hand_mass(seg),
            1e-9,
            &format!("{seg:?} mass, measured lengths"),
        );
    }
}

// ===========================================================================
// B1–B4 · Hand-computed statics (AC1, AC4, AC16)
// ===========================================================================

/// B1 — trunk vertical over the hip, no bar: every load-side COM is over the
/// hip, so the hip moment is zero. The sign bench (AC16).
#[test]
fn b1_a_vertical_trunk_over_the_hip_has_no_hip_moment() {
    let p = bottom(Lengths::table(), table_ankle(), 30.0, 0.0, None);
    let m = moment_about(Joint::Hip, &lifter(), &p).0;
    assert_close(m, 0.0, 1e-9, "hip moment, trunk vertical");
}

/// B2 — trunk 30° from vertical, no bar: the hip moment is `Σ m·g·lever` over
/// the §2.4.2 load side — pelvis, thorax & abdomen, head, arms — worked by
/// hand, and **positive**: the extensors are loaded (AC1, AC4, AC16).
///
/// By hand at 1.75 m / 80 kg (trunk 0.504 m, `sin 30° = 0.5`): levers 0.0226
/// (pelvis), 0.1091 (thorax & abdomen), 0.3089 (head), 0.252 (arms); masses
/// 11.36, 28.4, 6.48, 8.0 kg; `Σ m·lever = 7.373 kg·m`, × 9.81 ≈ **72.3 N·m**.
#[test]
fn b2_hip_moment_at_thirty_degrees_is_the_hand_sum_and_positive() {
    let p = thirty_degrees(None);
    let expected = hand_raw_sum(&p, p.hip.x.0, &[Pelvis, ThoraxAbdomen, HeadNeck, Arms]);
    assert_close(
        expected,
        72.3,
        0.1,
        "the hand arithmetic in the doc comment",
    );
    let m = moment_about(Joint::Hip, &lifter(), &p).0;
    assert_close(m, expected, 1e-6, "hip moment");
    assert!(m > 0.0, "hip extensors must be loaded: {m} N·m");
}

/// B2k — the same posture: the **knee** moment is positive too, although the
/// raw coordinate sum about the knee is negative — every load-side COM sits
/// behind the knee on a zig-zag chain. The test architect finding 3 exists
/// for (AC4, AC16).
///
/// By hand: levers from the knee −0.243 (thigh), −0.406 (pelvis), −0.320
/// (thorax & abdomen), −0.120 (head), −0.177 (arms); `Σ m·lever = −19.77
/// kg·m`, × 9.81 ≈ −194 N·m raw, so **+194 N·m** resisting extension.
#[test]
fn b2k_knee_moment_is_positive_although_its_raw_sum_is_negative() {
    let p = thirty_degrees(None);
    let raw = hand_raw_sum(
        &p,
        p.knee.x.0,
        &[Thigh, Pelvis, ThoraxAbdomen, HeadNeck, Arms],
    );
    assert!(
        raw < 0.0,
        "the geometry must put the load side behind the knee; raw sum {raw}"
    );
    assert_close(raw, -194.0, 0.5, "the hand arithmetic in the doc comment");
    let m = moment_about(Joint::Knee, &lifter(), &p).0;
    assert_close(m, -raw, 1e-6, "knee moment");
    assert!(m > 0.0, "knee extensors must be loaded: {m} N·m");
}

/// The lumbar row by hand (SPEC-0045 §2.5; AC4): L5/S1 on the trunk line 0.10
/// of the way to the shoulder; load side thorax & abdomen, head, arms —
/// ≈ 0.536 M, not the HAT lump; the sign is the hip's.
///
/// By hand: levers from L5/S1 0.0839, 0.2837, 0.2268; `Σ m·lever = 6.036
/// kg·m`, × 9.81 ≈ **59.2 N·m**.
#[test]
fn lumbar_moment_is_the_hand_sum_about_l5s1() {
    let p = thirty_degrees(None);
    let l5s1_x = along(p.hip, p.shoulder, L5S1_UP_THE_TRUNK).x.0;
    let expected = hand_raw_sum(&p, l5s1_x, &[ThoraxAbdomen, HeadNeck, Arms]);
    assert_close(
        expected,
        59.2,
        0.1,
        "the hand arithmetic in the doc comment",
    );
    let m = moment_about(Joint::Lumbar, &lifter(), &p).0;
    assert_close(m, expected, 1e-6, "lumbar moment");
    assert!(m > 0.0, "erectors must be loaded: {m} N·m");
}

/// The ankle row by hand (AC4): everything above the ankle, sign +1 — a load
/// ahead of the joint dorsiflexes it. This hand-built posture is not
/// balanced: its mass sits behind the ankle, so the moment is **negative** —
/// the load assists plantar flexion — and must be reported signed, never
/// `.abs()` (SPEC-0045 §2.4.1).
///
/// By hand: `Σ m·lever = −3.745 kg·m`, × 9.81 ≈ **−36.7 N·m**.
#[test]
fn ankle_moment_is_the_hand_sum_and_keeps_its_sign() {
    let p = thirty_degrees(None);
    let expected = hand_raw_sum(
        &p,
        p.ankle.x.0,
        &[Shank, Thigh, Pelvis, ThoraxAbdomen, HeadNeck, Arms],
    );
    assert_close(
        expected,
        -36.7,
        0.1,
        "the hand arithmetic in the doc comment",
    );
    let m = moment_about(Joint::Ankle, &lifter(), &p).0;
    assert_close(m, expected, 1e-6, "ankle moment");
    assert!(m < 0.0, "the sign must survive: {m} N·m");
}

/// B3 — MECÁNICA 01's setup: the reel's lengths as measured overrides (femur
/// 0.40, shank 0.43, trunk 0.53), its ankle and 22° shank, an 80 kg lifter, a
/// 100 kg bar high-bar over the mid-foot. The bar component alone — the same
/// posture on a massless body — reproduces the reel's **274 N·m**; the whole
/// model puts the hip at **≈ 366 N·m**. The reel's number is recovered as a
/// component, so the correction is derived, not asserted (AC2, AC16;
/// SPEC-0045 §2.4.3).
///
/// A hand-built posture, not the baseline solve: the setup *is* the reel's
/// geometry. The spec's 366 uses the HAT lump (92.9 N·m); the §2.4.2 split
/// places the trunk mass lower and lands near 357, so ≈ is read as ±5%.
#[test]
fn b3_the_reel_hip_moment_is_the_bar_component_of_a_larger_total() {
    let reel = Lengths::reel();
    let ankle = pt(-0.04, 0.09);
    let hip_x = ankle.x.0 + reel.shank * 22.0_f64.to_radians().sin() - reel.thigh;
    let trunk_deg = (-hip_x / (HIGH_BAR * reel.trunk)).asin().to_degrees();
    let p = bottom(reel, ankle, 22.0, trunk_deg, Some(100.0));
    let bar = p.load.expect("the reel has a bar").at;
    assert_close(bar.x.0, 0.0, 1e-9, "the bar over the mid-foot");

    let massless = scale(Metres(HEIGHT_M), 0.0, &reel.measured());
    let bar_component = moment_about(Joint::Hip, &massless, &p).0;
    assert_close(bar_component, 274.0, 1.0, "the reel's published hip moment");

    let reel_lifter = scale(Metres(HEIGHT_M), MASS_KG, &reel.measured());
    let total = moment_about(Joint::Hip, &reel_lifter, &p).0;
    assert_close(total, 366.0, 0.05 * 366.0, "the honest hip moment");
    assert!(
        total > bar_component,
        "segment weight must add to the reel's number: total {total}, bar {bar_component}"
    );
}

/// B4 — the knee moment is invariant to femur length at fixed shank geometry
/// (RFC-002 C2), now with segment weight: under the system-COM balance the
/// bar and the whole load side move with the hip, not the knee (AC16).
#[test]
fn b4_knee_moment_does_not_depend_on_femur_length_at_fixed_shank_geometry() {
    let solve = |thigh: f64| {
        let measured = MeasuredLengths {
            thigh: Some(Metres(thigh)),
            ..MeasuredLengths::default()
        };
        let body = scale(Metres(HEIGHT_M), MASS_KG, &measured);
        let p = solved(&body, Some(100.0));
        (p, moment_about(Joint::Knee, &body, &p).0)
    };
    let (short, knee_short) = solve(0.38);
    let (long, knee_long) = solve(0.44);
    assert_point_close(
        short.knee,
        long.knee,
        1e-9,
        "fixed shank geometry: the knee",
    );
    assert_point_close(
        short.ankle,
        long.ankle,
        1e-9,
        "fixed shank geometry: the ankle",
    );
    assert_close(
        knee_long,
        knee_short,
        1e-3,
        "knee moment across femur lengths",
    );
}

// ===========================================================================
// B5 · Balance (AC3, AC6; architect finding 4)
// ===========================================================================

/// B5 — bodyweight-only solves: a posture exists, it carries no load the
/// model invented, and the combined COM of everything above the ankle is
/// over the mid-foot within 1 mm (AC3, AC9). The result's
/// `LoadBasis::BodyweightOnly` is the same claim's other half, on the
/// `Mechanics` producer.
#[test]
fn b5_bodyweight_only_solves_with_the_system_com_over_the_mid_foot() {
    let body = lifter();
    let p = solved(&body, None);
    assert_eq!(
        p.load, None,
        "no load was supplied; the model must not assume one"
    );
    assert_close(
        system_com_x(&body, &p),
        0.0,
        1e-3,
        "system COM x, bodyweight-only",
    );
}

/// SPEC-0045 §2.6.1 (AC6) — the baseline is a **stated** posture: the ankle
/// at `(−0.26 × foot, 0.039 H)`, the thigh parallel with the hip behind the
/// knee, the bar at 0.95 of the trunk line carrying the supplied mass — and
/// it balances with the bar included.
#[test]
fn baseline_with_a_bar_is_the_stated_posture_and_balances() {
    let body = lifter();
    let p = solved(&body, Some(100.0));
    assert_point_close(p.ankle, table_ankle(), 1e-3, "the ankle");
    assert_close(
        p.hip.y.0,
        p.knee.y.0,
        1e-9,
        "thigh parallel: hip height vs knee height",
    );
    assert!(
        p.hip.x.0 < p.knee.x.0,
        "the hip sits behind the knee: hip {} knee {}",
        p.hip.x.0,
        p.knee.x.0
    );
    let load = p.load.expect("the bar that was supplied");
    assert_close(load.kg, 100.0, 1e-12, "bar mass");
    assert_point_close(
        load.at,
        along(p.hip, p.shoulder, HIGH_BAR),
        1e-9,
        "the bar on the trunk line",
    );
    assert_close(
        system_com_x(&body, &p),
        0.0,
        1e-3,
        "system COM x, with the bar",
    );
}

/// B5b — as the bar's mass dominates, the solve converges to
/// bar-over-mid-foot, the limit claim of SPEC-0045 §2.6.3: at 10⁴ kg the bar
/// is within 5 mm of the line, and nearer than at 10³ kg (AC16).
#[test]
fn b5b_a_dominant_bar_converges_to_bar_over_the_mid_foot() {
    let body = lifter();
    let bar_x = |kg: f64| {
        solved(&body, Some(kg))
            .load
            .expect("a bar was supplied")
            .at
            .x
            .0
    };
    let heavy = bar_x(1_000.0);
    let dominant = bar_x(10_000.0);
    assert_close(dominant, 0.0, 5e-3, "bar x at 10⁴ kg");
    assert!(
        dominant.abs() < heavy.abs(),
        "the residual must shrink as the bar dominates: 10³ kg → {heavy}, 10⁴ kg → {dominant}"
    );
}

// ===========================================================================
// B6 · Symmetry (AC16)
// ===========================================================================

/// B6 — a posture mirrored in the mid-foot line gives every moment mirrored,
/// with and without a bar: the sign rule is a coordinate rule applied
/// consistently at all four joints, never an `.abs()` (AC16).
#[test]
fn b6_a_mirrored_posture_gives_mirrored_moments_at_every_joint() {
    let body = lifter();
    for bar_kg in [None, Some(100.0)] {
        let p = thirty_degrees(bar_kg);
        let flipped = mirrored(p);
        for j in JOINTS {
            let forward = moment_about(j, &body, &p).0;
            let backward = moment_about(j, &body, &flipped).0;
            assert_close(
                backward,
                -forward,
                1e-9,
                &format!("{j:?} mirrored, bar {bar_kg:?}"),
            );
        }
        assert!(
            moment_about(Joint::Hip, &body, &p).0.abs() > 1.0,
            "the fixture must load the hip, or the claim is vacuous"
        );
    }
}

// ===========================================================================
// B9–B10 · Rejection (AC9, AC13) — each reached through the public solve
// ===========================================================================

/// B9 — `DoesNotBalance`: a femur the trunk cannot reach past — the trunk
/// would have to lean beyond horizontal to bring the system COM over the
/// mid-foot. The baseline's unknown is the trunk angle (SPEC-0045 §2.6.3), so
/// that is the unknown named. With and without a bar (AC9, AC13).
#[test]
fn b9_a_femur_the_trunk_cannot_reach_past_does_not_balance() {
    let measured = MeasuredLengths {
        thigh: Some(Metres(1.0)),
        trunk: Some(Metres(0.30)),
        ..MeasuredLengths::default()
    };
    let body = scale(Metres(HEIGHT_M), MASS_KG, &measured);
    for bar_kg in [None, Some(100.0)] {
        assert_eq!(
            baseline(&body, bar_kg),
            Err(Rejection::DoesNotBalance {
                unknown: Unknown::TrunkAngle
            }),
            "bar {bar_kg:?}"
        );
    }
}

/// B10 — `AnatomicallyImplausible`: a short shank and a long femur under a
/// heavy bar balance only with the trunk past the stated hip range — more
/// than 40° of forward lean at parallel is more than 130° of hip flexion
/// (SPEC-0045 §2.6.4). The knee and ankle angles are fixed by the baseline's
/// conventions, so the hip is the joint that can leave its range (AC9, AC13).
///
/// The geometry is chosen so the solve balances (`sin θ` between 0.73 and
/// 0.93) for any baseline shank convention between 15° and 45°.
#[test]
fn b10_a_posture_that_balances_only_past_the_hip_range_is_implausible() {
    let measured = MeasuredLengths {
        shank: Some(Metres(0.25)),
        thigh: Some(Metres(0.55)),
        trunk: Some(Metres(0.72)),
    };
    let body = scale(Metres(HEIGHT_M), MASS_KG, &measured);
    match baseline(&body, Some(200.0)) {
        Err(Rejection::AnatomicallyImplausible {
            joint: Joint::Hip,
            angle_deg,
        }) => assert!(
            angle_deg > 130.0,
            "hip flexion {angle_deg}° must be past the stated 130° range"
        ),
        other => panic!("expected the hip to be implausible, got {other:?}"),
    }
}

// ===========================================================================
// Placement conventions and structure (SPEC-0045 §2.4.2, §2.5)
// ===========================================================================

/// B15 — with a bar the arm COMs sit at the shoulder → bar midpoint (hands on
/// the bar); without one they hang, directly below the shoulder at 0.436 of
/// the upper-arm length (AC12; SPEC-0045 §2.4.2).
#[test]
fn b15_arms_are_on_the_bar_when_there_is_one_and_hang_when_there_is_not() {
    let body = lifter();
    let loaded = thirty_degrees(Some(100.0));
    let bar = loaded.load.expect("a bar").at;
    assert_point_close(
        Arms.com(&body, &loaded),
        along(loaded.shoulder, bar, 0.5),
        1e-9,
        "arms on the bar",
    );
    let unloaded = Posture {
        load: None,
        ..loaded
    };
    let hang = WINTER.upper_arm.com_from_proximal * table_length(WINTER.upper_arm);
    assert_point_close(
        Arms.com(&body, &unloaded),
        pt(unloaded.shoulder.x.0, unloaded.shoulder.y.0 - hang),
        1e-9,
        "arms hanging",
    );
}

/// SPEC-0045 §2.4.2 — the head & neck COM sits on the trunk line extended
/// beyond the shoulder by half the head & neck length (0.5 × 0.130 H); a
/// stated convention, error carried (AC2, AC12).
#[test]
fn head_com_is_on_the_trunk_line_beyond_the_shoulder() {
    let body = lifter();
    let p = thirty_degrees(None);
    assert_point_close(
        HeadNeck.com(&body, &p),
        hand_com(HeadNeck, &p),
        1e-9,
        "head & neck COM",
    );
}

/// SPEC-0045 §2.4.2 — four fixed load-side lists: everything above the joint,
/// the pelvis on the hip's side in full, nothing below (AC1, AC4).
#[test]
fn above_is_the_four_fixed_load_side_lists() {
    let expect = |j: Joint, want: &[ChainSegment]| {
        let got = above(j);
        assert!(
            got.len() == want.len() && want.iter().all(|s| got.contains(s)),
            "above({j:?}) = {got:?}, want {want:?}"
        );
    };
    expect(
        Joint::Ankle,
        &[Shank, Thigh, Pelvis, ThoraxAbdomen, HeadNeck, Arms],
    );
    expect(Joint::Knee, &[Thigh, Pelvis, ThoraxAbdomen, HeadNeck, Arms]);
    expect(Joint::Hip, &[Pelvis, ThoraxAbdomen, HeadNeck, Arms]);
    expect(Joint::Lumbar, &[ThoraxAbdomen, HeadNeck, Arms]);
}

/// SPEC-0045 §2.5 — `Posture::at` returns the posture's own ankle, knee and
/// hip, and places L5/S1 on the trunk line 0.10 of the way from the hip to
/// the shoulder — a stated convention pending citation (AC4).
#[test]
fn posture_at_returns_the_joints_and_places_l5s1_by_convention() {
    let p = thirty_degrees(None);
    assert_eq!(p.at(Joint::Ankle), p.ankle);
    assert_eq!(p.at(Joint::Knee), p.knee);
    assert_eq!(p.at(Joint::Hip), p.hip);
    assert_point_close(
        p.at(Joint::Lumbar),
        along(p.hip, p.shoulder, L5S1_UP_THE_TRUNK),
        1e-9,
        "L5/S1",
    );
}
