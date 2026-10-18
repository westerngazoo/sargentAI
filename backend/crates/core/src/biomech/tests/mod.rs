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
//! Dorsiflexion is a **required** solve input (SPEC-0045 §2.6.1.1), so every
//! fixture below states an ankle. That is the point of the decision: a test
//! cannot quietly inherit a number nobody sourced, and neither can a caller.
//!
//! Slice B1's range claims (R1–R12) live in [`range`] and reuse the hand model
//! below.

#![allow(clippy::doc_markdown)]

mod range;

use crate::biomech::ChainSegment::{Arms, HeadNeck, Pelvis, Shank, Thigh, ThoraxAbdomen};
use crate::biomech::{
    above, analyse_baseline, baseline, baseline_mechanics, moment_about, scale, Assumption,
    Baseline, Body, ChainSegment, Dorsiflexion, Joint, Load, LoadBasis, MeasuredLengths, Mechanics,
    Metres, Point, Position, Posture, Rejection, ResistingGroup, Segment, Unknown, Verification,
    COM_PLACEMENT_SPREAD_M, L5S1_PLACEMENT_SPREAD_M, L5S1_UP_THE_TRUNK, WINTER,
    WINTER_VERIFICATION,
};
use crate::workout::LoadKg;

// ===========================================================================
// Fixture conventions
// ===========================================================================

/// `g`, as SPEC-0045 §2.2 fixes it.
const G: f64 = 9.81;

/// The fixture lifter: table proportions at 1.75 m, 80 kg — the body
/// SPEC-0045 §2.4.3 scales the reel against.
const HEIGHT_M: f64 = 1.75;
const MASS_KG: f64 = 80.0;

// `L5S1_UP_THE_TRUNK` is imported rather than restated: it is an *input* to
// the hand computations below, not one of their results, and a second literal
// would only test that two literals match. What keeps it honest is
// `hat_row_is_reproduced_by_the_split`, which derives it from the table.

/// SPEC-0045 §2.6.1: the bar attaches this far up the trunk line (high-bar).
const HIGH_BAR: f64 = 0.95;

/// SPEC-0045 §2.6.1: the ankle sits this fraction of a foot length behind the
/// mid-foot line — the ankle at the heel-side quarter, mid-foot at half.
const ANKLE_BEHIND_MID_FOOT: f64 = 0.26;

/// The four joints, in the result's fixed order.
const JOINTS: [Joint; 4] = [Joint::Hip, Joint::Knee, Joint::Ankle, Joint::Lumbar];

/// An ankle that comfortably reaches parallel bodyweight-only for the fixture
/// lifter — whose threshold is 28.7° (SPEC-0045 §2.6.1.1).
const REACHES_PARALLEL: f64 = 40.0;

/// A restricted ankle: short of the same lifter's bodyweight threshold of
/// 28.7°, and well inside their 13.4° under a bar.
const RESTRICTED: f64 = 25.0;

/// The fixture's bodyweight threshold, degrees (SPEC-0045 §2.6.1.1).
const BODYWEIGHT_THRESHOLD_DEG: f64 = 28.7;

/// A dorsiflexion the domain accepts.
fn dorsi(degrees: f64) -> Dorsiflexion {
    Dorsiflexion::from_degrees(degrees).expect("a dorsiflexion inside 0°..=90°")
}

/// A bar the workout crate's validated load accepts.
fn bar(kg: f64) -> LoadKg {
    LoadKg::try_new(kg).expect("a load inside (0, 1000]")
}

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
        kg: bar(kg),
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
fn solve(body: &Body, bar_kg: Option<f64>, dorsiflexion: f64) -> Baseline {
    baseline(body, bar_kg.map(bar), dorsi(dorsiflexion)).unwrap_or_else(|r| {
        panic!("baseline (bar {bar_kg:?}, ankle {dorsiflexion}°) rejected: {r:?}")
    })
}

/// The same, as a bare posture.
fn solved(body: &Body, bar_kg: Option<f64>, dorsiflexion: f64) -> Posture {
    *solve(body, bar_kg, dorsiflexion).posture()
}

/// The solved trunk lean, degrees from vertical — what §2.6.1.1's table is a
/// table of.
fn lean_deg(p: &Posture) -> f64 {
    (p.shoulder.x.0 - p.hip.x.0)
        .atan2(p.shoulder.y.0 - p.hip.y.0)
        .to_degrees()
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

/// How far below the shoulder a straight, hanging arm's centre of mass sits:
/// the mass-weighted mean of the upper arm, forearm and hand, each at its own
/// Table 4.1 centre along the arm.
///
/// By hand at 1.75 m: segment centres 0.1419, 0.4354 and 0.6766 m below the
/// shoulder, masses 0.028, 0.016 and 0.006 M, so **0.300 m** — more than twice
/// the upper arm's own 0.142, which is what the draft placed the whole arm at.
fn whole_arm_hang() -> f64 {
    let (upper, fore, hand) = (
        table_length(WINTER.upper_arm),
        table_length(WINTER.forearm),
        table_length(WINTER.hand),
    );
    let rows = [
        (
            WINTER.upper_arm.mass_fraction,
            WINTER.upper_arm.com_from_proximal * upper,
        ),
        (
            WINTER.forearm.mass_fraction,
            upper + WINTER.forearm.com_from_proximal * fore,
        ),
        (
            WINTER.hand.mass_fraction,
            upper + fore + WINTER.hand.com_from_proximal * hand,
        ),
    ];
    let mass: f64 = rows.iter().map(|&(m, _)| m).sum();
    rows.iter().map(|&(m, depth)| m * depth).sum::<f64>() / mass
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
            None => pt(p.shoulder.x.0, p.shoulder.y.0 - whole_arm_hang()),
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
    let load = p
        .load
        .map_or(0.0, |l| l.kg.get() * G * (l.at.x.0 - joint_x));
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
        first_moment += l.kg.get() * l.at.x.0;
        mass += l.kg.get();
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

/// B0 — the fourth sum claim, and the only one that constrains **placement**
/// rather than mass (AC2; SPEC-0045 §2.5).
///
/// `WINTER.hat` is the same mass on the same line as the §2.4.2 split — head,
/// arms and trunk, greater trochanter to glenohumeral — carrying its own
/// centre of mass. So the split has to reproduce it, and since the table
/// supplies every placement in the split *except* L5/S1, the identity is what
/// determines that one. The other three sum claims check that no mass was
/// lost; this checks that none was put in the wrong place.
///
/// Its absence cost 16%. With the draft's `L5S1_UP_THE_TRUNK = 0.10` the
/// split's centre of mass sat at 0.539 of the trunk line against the table's
/// own 0.626 — every hip moment wrong by that ratio, and nothing in the suite
/// to say so.
#[test]
fn hat_row_is_reproduced_by_the_split() {
    // Fraction of the trunk line each split segment's centre of mass sits at,
    // re-derived from the §2.4.2 conventions. Arms hang: their centre of mass
    // is directly below the shoulder, so in the horizontal projection every
    // moment is taken in, they sit at the shoulder.
    let pelvis = L5S1_UP_THE_TRUNK * (1.0 - WINTER.pelvis.com_from_proximal);
    let thorax = 1.0 - (1.0 - L5S1_UP_THE_TRUNK) * WINTER.thorax_abdomen.com_from_proximal;
    let head = 1.0
        + WINTER.head_neck.com_from_proximal * table_length(WINTER.head_neck)
            / table_length(WINTER.trunk);
    let arms = 1.0;

    let rows = [
        (WINTER.pelvis.mass_fraction, pelvis),
        (WINTER.thorax_abdomen.mass_fraction, thorax),
        (WINTER.head_neck.mass_fraction, head),
        (
            2.0 * (WINTER.upper_arm.mass_fraction
                + WINTER.forearm.mass_fraction
                + WINTER.hand.mass_fraction),
            arms,
        ),
    ];
    let mass: f64 = rows.iter().map(|&(m, _)| m).sum();
    assert_close(
        mass,
        WINTER.hat.mass_fraction,
        1e-9,
        "the split is the same mass as the lump",
    );
    let centre: f64 = rows.iter().map(|&(m, k)| m * k).sum::<f64>() / mass;
    assert_close(
        centre,
        WINTER.hat.com_from_proximal,
        1e-4,
        "the split is the same centre of mass as the lump",
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
/// constants ship: edition, pages, date, who. A recollection is not the
/// control.
///
/// **Ignored, not deleted, and not passing.** It needs someone to open the
/// printed 4th edition, which no amount of code can do; leaving it red would
/// hold the merge gate hostage to a library errand, and deleting it would lose
/// the only record that the errand is outstanding. Run it with
/// `cargo test -- --ignored`; it turns green when [`WINTER_VERIFICATION`] is
/// filled in, and the `#[ignore]` comes off in the same commit.
#[test]
#[ignore = "AC2: needs the printed Winter 4th ed. checked against WINTER, then WINTER_VERIFICATION filled in"]
fn winter_verification_record_is_filled_in() {
    assert!(
        matches!(WINTER_VERIFICATION, Verification::Verified { .. }),
        "WINTER is {WINTER_VERIFICATION:?}: check Table 4.1 / Figure 4.1 against the printed edition and record it"
    );
}

/// SPEC-0045 §2.3 — `scale` stays total by falling back to zero for a row
/// Figure 4.1 gives no length; this is the claim that the fallback is never
/// taken, and it is asserted on what `scale` **produced** rather than on a
/// hand-kept list of the rows it reads, so a seventh row cannot slip past it.
///
/// A zero-length segment would not panic — it would quietly corrupt the
/// balance coefficients and return a posture, which is the failure mode worth
/// a test rather than a `todo!()`.
#[test]
fn scaling_never_takes_the_missing_length_fallback() {
    let body = lifter();
    let lengths = [
        ("thigh", body.thigh()),
        ("shank", body.shank()),
        ("trunk", body.trunk()),
        ("foot", body.foot()),
        ("head & neck", body.head_neck()),
        ("upper arm", body.upper_arm()),
        ("forearm", body.forearm()),
        ("hand", body.hand()),
        ("ankle height", body.ankle_height()),
    ];
    for (name, metres) in lengths {
        assert!(
            metres > 0.0,
            "{name} scaled to {metres} m — a row with no Figure 4.1 length"
        );
    }
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
/// By hand at 1.75 m / 80 kg (trunk 0.504 m, `sin 30° = 0.5`): levers 0.0603
/// (pelvis), 0.1357 (thorax & abdomen), 0.3089 (head), 0.252 (arms); masses
/// 11.36, 28.4, 6.48, 8.0 kg; `Σ m·lever = 8.557 kg·m`, × 9.81 ≈ **83.9 N·m**.
#[test]
fn b2_hip_moment_at_thirty_degrees_is_the_hand_sum_and_positive() {
    let p = thirty_degrees(None);
    let expected = hand_raw_sum(&p, p.hip.x.0, &[Pelvis, ThoraxAbdomen, HeadNeck, Arms]);
    assert_close(
        expected,
        83.9,
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
/// By hand: levers from the knee −0.243 (thigh), −0.368 (pelvis), −0.293
/// (thorax & abdomen), −0.120 (head), −0.177 (arms); `Σ m·lever = −18.59
/// kg·m`, × 9.81 ≈ −182 N·m raw, so **+182 N·m** resisting extension.
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
    assert_close(raw, -182.4, 0.5, "the hand arithmetic in the doc comment");
    let m = moment_about(Joint::Knee, &lifter(), &p).0;
    assert_close(m, -raw, 1e-6, "knee moment");
    assert!(m > 0.0, "knee extensors must be loaded: {m} N·m");
}

/// The lumbar row by hand (SPEC-0045 §2.5; AC4): L5/S1 on the trunk line 0.10
/// of the way to the shoulder; load side thorax & abdomen, head, arms —
/// ≈ 0.536 M, not the HAT lump; the sign is the hip's.
///
/// By hand: L5/S1 sits 0.0674 m ahead of the hip, so the levers are 0.0683,
/// 0.2415, 0.1846; `Σ m·lever = 4.982 kg·m`, × 9.81 ≈ **48.9 N·m**.
#[test]
fn lumbar_moment_is_the_hand_sum_about_l5s1() {
    let p = thirty_degrees(None);
    let l5s1_x = along(p.hip, p.shoulder, L5S1_UP_THE_TRUNK).x.0;
    let expected = hand_raw_sum(&p, l5s1_x, &[ThoraxAbdomen, HeadNeck, Arms]);
    assert_close(
        expected,
        48.9,
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
/// By hand: `Σ m·lever = −2.561 kg·m`, × 9.81 ≈ **−25.1 N·m**.
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
        -25.1,
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
/// model puts the hip at **370.6 N·m** — the published figure is understated
/// by 35%. The reel's number is recovered as a *component*, so the correction
/// is derived, not asserted (AC2, AC16; SPEC-0045 §2.4.3).
///
/// A hand-built posture, not the baseline solve: the setup *is* the reel's
/// geometry, with the trunk at 33.64° putting the bar over the mid-foot.
///
/// The band is ±1 N·m, not ±5%. A loose band here is what let the model
/// disagree with its own table by 16% unnoticed: the split landed at 357
/// against the HAT lump's 371 and the test passed anyway. With
/// `L5S1_UP_THE_TRUNK` derived from the table the two agree to 0.2%, which is
/// a claim worth making tightly — see `hat_row_is_reproduced_by_the_split`.
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
    assert_close(total, 370.6, 1.0, "the honest hip moment");
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
        let p = solved(&body, Some(100.0), REACHES_PARALLEL);
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

/// B5 — bodyweight-only solves **at a dorsiflexion that reaches parallel**
/// (§2.6.1.1): a posture exists, it carries no load the model invented, the
/// combined COM of everything above the ankle is over the mid-foot within
/// 1 mm, and the result says `BodyweightOnly` rather than assuming a bar
/// (AC3, AC9).
#[test]
fn b5_bodyweight_only_solves_with_the_system_com_over_the_mid_foot() {
    let body = lifter();
    let solved = solve(&body, None, REACHES_PARALLEL);
    let p = *solved.posture();
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
    assert_eq!(
        analyse_baseline(&body, &solved).load,
        LoadBasis::BodyweightOnly,
        "the result must say it ran on bodyweight"
    );
}

/// B5c — the §2.6.1.1 relation: more ankle, less forward lean, strictly, and
/// the published table is reproduced within 1°. Bodyweight-only with the arms
/// hanging — the conditions are part of the claim, which is what the first
/// version of that table was missing.
#[test]
fn b5c_the_dorsiflexion_to_lean_relation_reproduces_the_published_table() {
    let body = lifter();
    let table = [
        (30.0, 74.5),
        (35.0, 55.7),
        (40.0, 44.1),
        (45.0, 35.1),
        (50.0, 27.7),
        (55.0, 21.5),
    ];
    for (dorsiflexion, lean) in table {
        let got = lean_deg(&solved(&body, None, dorsiflexion));
        assert_close(got, lean, 1.0, &format!("lean at {dorsiflexion}° of ankle"));
    }

    let mut previous = f64::INFINITY;
    for step in 29..=90 {
        let lean = lean_deg(&solved(&body, None, f64::from(step)));
        assert!(
            lean < previous,
            "lean must fall strictly with dorsiflexion: {step}° gave {lean}° after {previous}°"
        );
        previous = lean;
    }
}

/// B5d — a restricted ankle does not reach parallel bodyweight-only, and the
/// model says which joint stopped them and what it would take (AC9, AC13).
///
/// **Not** `AnatomicallyImplausible`: no limit constant is committed, so that
/// variant is unreachable by construction (§2.6.4). `needed_deg` is the
/// lifter's own `u_min`, not a comparison against anybody's normative range.
#[test]
fn b5d_a_restricted_ankle_cannot_reach_parallel_and_the_ankle_is_named() {
    match baseline(&lifter(), None, dorsi(RESTRICTED)) {
        Err(Rejection::DepthNotReachable {
            limiting: Joint::Ankle,
            needed_deg,
            available_deg,
        }) => {
            assert_close(available_deg, RESTRICTED, 1e-12, "what they have");
            assert_close(
                needed_deg,
                BODYWEIGHT_THRESHOLD_DEG,
                0.1,
                "what parallel would take",
            );
            assert!(
                baseline(&lifter(), None, dorsi(needed_deg + 0.1)).is_ok(),
                "`needed_deg` must be the least ankle that works, or it is just a number"
            );
            assert!(
                needed_deg > available_deg,
                "the rejection is only honest if more ankle would fix it"
            );
        }
        other => panic!("expected the ankle to be named, got {other:?}"),
    }
}

/// B5e — the same lifter, the same ankle, with 100 kg on their back: it
/// **solves**. The bar sits at 0.95 of the trunk line, a far longer lever than
/// the body's own load side, so a smaller lean carries the system over the
/// mid-foot. The rejection is a property of the lift, not a label on the
/// lifter (§2.6.1.1).
#[test]
fn b5e_the_same_restricted_ankle_reaches_parallel_under_a_bar() {
    let body = lifter();
    let loaded = solved(&body, Some(100.0), RESTRICTED);
    assert_close(
        system_com_x(&body, &loaded),
        0.0,
        1e-3,
        "system COM x, restricted ankle under a bar",
    );
    assert!(
        baseline(&body, None, dorsi(RESTRICTED)).is_err(),
        "the bodyweight case must still be rejected, or the comparison is empty"
    );
    let same_ankle_bodyweight = lean_deg(&solved(&body, None, REACHES_PARALLEL));
    let same_ankle_loaded = lean_deg(&solved(&body, Some(100.0), REACHES_PARALLEL));
    assert!(
        same_ankle_loaded < same_ankle_bodyweight,
        "at the same ankle the loaded squat must be the more upright: \
         bodyweight {same_ankle_bodyweight}°, loaded {same_ankle_loaded}°"
    );
}

/// SPEC-0045 §2.6.1 (AC6) — the baseline is a **stated** posture: the ankle
/// at `(−0.26 × foot, 0.039 H)`, the thigh parallel with the hip behind the
/// knee, the bar at 0.95 of the trunk line carrying the supplied mass — and
/// it balances with the bar included.
#[test]
fn baseline_with_a_bar_is_the_stated_posture_and_balances() {
    let body = lifter();
    let p = solved(&body, Some(100.0), REACHES_PARALLEL);
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
    assert_close(load.kg.get(), 100.0, 1e-12, "bar mass");
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

/// B5b — as the bar's mass grows the solve converges to bar-over-mid-foot,
/// the limit claim of SPEC-0045 §2.6.3 (AC16).
///
/// The draft probed 10⁴ kg. That is not representable: `LoadKg` caps at
/// 1000 kg, and a limit claim tested outside the type's domain is a claim
/// about a lift nobody can log. Stated instead as the convergence it actually
/// is — strictly shrinking over the sequence, and inside 6 mm at the cap.
#[test]
fn b5b_a_heavier_bar_converges_to_bar_over_the_mid_foot() {
    let body = lifter();
    let bar_x = |kg: f64| {
        solved(&body, Some(kg), REACHES_PARALLEL)
            .load
            .expect("a bar was supplied")
            .at
            .x
            .0
    };
    let sequence: Vec<f64> = [100.0, 250.0, 500.0, LoadKg::MAX]
        .into_iter()
        .map(bar_x)
        .collect();
    for pair in sequence.windows(2) {
        assert!(
            pair[1].abs() < pair[0].abs(),
            "the residual must shrink as the bar dominates: {sequence:?}"
        );
    }
    let at_cap = sequence.last().copied().expect("four entries");
    assert_close(at_cap, 0.0, 6e-3, "bar x at LoadKg::MAX");
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
    for bar_kg in [None, Some(bar(100.0))] {
        assert_eq!(
            baseline(&body, bar_kg, dorsi(REACHES_PARALLEL)),
            Err(Rejection::DoesNotBalance {
                unknown: Unknown::TrunkAngle
            }),
            "bar {bar_kg:?}"
        );
    }
    // The two rejections are distinguished by *"would more ankle fix this?"*,
    // so this one is only correct if no ankle in the domain would — otherwise
    // it should have been `DepthNotReachable` (§2.6.4).
    for step in 0..=90 {
        assert!(
            baseline(&body, None, dorsi(f64::from(step))).is_err(),
            "no dorsiflexion may rescue this body, but {step}° did"
        );
    }
}

/// B10 — `AnatomicallyImplausible` is **deferred**, and this is the test that
/// keeps the deferral honest: no input reaches it, because no limit constant
/// is committed until one is cited from measured squat kinematics
/// (SPEC-0045 §2.6.4).
///
/// The draft's placeholder — hip flexion > 130° — **rejected the model's own
/// baseline**: a real parallel squat reaches 115–172° across the §2.6.1.1
/// range, so the check fired on the primary output. A passive-ROM norm is the
/// wrong instrument for a loaded deep squat, which exceeds passive norms by
/// design. This test fails the day a limit is added, which is exactly when
/// someone should have to show the citation.
#[test]
fn b10_anatomically_implausible_is_unreachable_until_a_limit_is_cited() {
    let bodies = [
        MeasuredLengths::default(),
        MeasuredLengths {
            shank: Some(Metres(0.25)),
            thigh: Some(Metres(0.55)),
            trunk: Some(Metres(0.72)),
        },
        Lengths::reel().measured(),
    ];
    for measured in bodies {
        let body = scale(Metres(HEIGHT_M), MASS_KG, &measured);
        for step in 0..=90 {
            for bar_kg in [None, Some(60.0), Some(200.0)] {
                let outcome = baseline(&body, bar_kg.map(bar), dorsi(f64::from(step)));
                assert!(
                    !matches!(outcome, Err(Rejection::AnatomicallyImplausible { .. })),
                    "a limit constant reached production without a citation: \
                     {measured:?} at {step}°, bar {bar_kg:?}"
                );
            }
        }
    }
}

/// SPEC-0045 §2.6.4's table has four rows; `B9` and `B5d` cover two. These are
/// the other two, both reachable and neither previously claimed (AC9).
#[test]
fn the_other_two_rows_of_the_rejection_table_are_reachable() {
    // `u > u_max`: a long shank and a short femur put the knee so far forward
    // that the lifter would have to lean *backwards*. More ankle cannot fix
    // what too much ankle caused, so this is `DoesNotBalance`, not
    // `DepthNotReachable`.
    let long_shank = scale(
        Metres(HEIGHT_M),
        MASS_KG,
        &MeasuredLengths {
            shank: Some(Metres(0.50)),
            thigh: Some(Metres(0.35)),
            ..MeasuredLengths::default()
        },
    );
    assert_eq!(
        baseline(&long_shank, None, dorsi(50.0)),
        Err(Rejection::DoesNotBalance {
            unknown: Unknown::TrunkAngle
        }),
        "past u_max the trunk would have to lean backwards"
    );

    // No mass on the trunk line and no bar: the balance equation is `0 = 0`,
    // the trunk angle is not determined, and picking one would be inventing a
    // posture. B3 builds exactly this body to isolate the reel's bar term.
    let massless = scale(Metres(HEIGHT_M), 0.0, &MeasuredLengths::default());
    assert_eq!(
        baseline(&massless, None, dorsi(REACHES_PARALLEL)),
        Err(Rejection::DoesNotBalance {
            unknown: Unknown::TrunkAngle
        }),
        "an undetermined lean is not a posture"
    );
}

// ===========================================================================
// B13, B14, B16 · What the result admits to (AC5, AC12, AC14)
// ===========================================================================

/// The fixture lifter's baseline analysis, bar or none.
fn analysed(bar_kg: Option<f64>) -> Mechanics {
    baseline_mechanics(&lifter(), bar_kg.map(bar), dorsi(REACHES_PARALLEL))
        .unwrap_or_else(|r| panic!("baseline (bar {bar_kg:?}) rejected: {r:?}"))
}

/// B13 — every result carries the five unconditional assumptions, and
/// `TableDerivedLengths` (AC12).
///
/// In v1 that last one is present whatever the user has had measured: the
/// foot, head & neck and upper arm have **no** measurement path, so measuring
/// the three that do cannot remove it. The `iff` of §2.8 therefore reduces to
/// *always* here — asserted both ways round so it stops quietly reducing to
/// that the day a fourth length becomes measurable.
#[test]
fn b13_every_result_states_what_it_was_computed_under() {
    for bar_kg in [None, Some(100.0)] {
        let result = analysed(bar_kg);
        for required in [
            Assumption::SagittalPlane,
            Assumption::QuasiStatic,
            Assumption::SinglePosture {
                at: Position::Parallel,
            },
            Assumption::SolvedBaseline,
            Assumption::TableDerivedMasses,
            Assumption::TableDerivedLengths,
        ] {
            assert!(
                result.assumptions.contains(&required),
                "bar {bar_kg:?}: {required:?} missing from {:?}",
                result.assumptions
            );
        }
    }

    let every_length_measured = scale(Metres(HEIGHT_M), MASS_KG, &Lengths::table().measured());
    let result = baseline_mechanics(&every_length_measured, None, dorsi(REACHES_PARALLEL))
        .expect("the table lifter's own lengths must still solve");
    assert!(
        result
            .assumptions
            .contains(&Assumption::TableDerivedLengths),
        "the foot, head and upper arm have no measurement path, so this stands"
    );
}

/// B14 — the lumbar row is present, so the result says the trunk was treated
/// as one rigid link through a conventionally-placed L5/S1 (AC12).
#[test]
fn b14_a_lumbar_row_carries_the_rigid_trunk_assumption() {
    let result = analysed(Some(100.0));
    assert!(
        result.joints.iter().any(|j| j.joint == Joint::Lumbar),
        "the fixed four include the lumbar row"
    );
    assert!(
        result.assumptions.contains(&Assumption::RigidTrunkLumbar),
        "a lumbar row must carry its assumption: {:?}",
        result.assumptions
    );
}

/// The result's fixed order, its group attribution, and the load basis
/// (AC4, AC5, AC3).
#[test]
fn the_result_is_four_joints_in_order_each_attributed_to_its_group() {
    let result = analysed(Some(140.0));
    let order: Vec<Joint> = result.joints.iter().map(|j| j.joint).collect();
    assert_eq!(order, JOINTS, "hip, knee, ankle, lumbar — fixed");
    assert_eq!(result.load, LoadBasis::Bar { kg: 140.0 });
    assert_eq!(
        analysed(None).load,
        LoadBasis::BodyweightOnly,
        "and it never invents a bar"
    );
}

/// Every value the output namespace can take, serialized. Built behind
/// exhaustive `match`es so a new variant cannot skip the AC14 check by being
/// forgotten — a single sampled result would never see `BodyweightOnly`, the
/// conditional assumptions, or any `Rejection` at all.
fn every_output_value_as_json() -> String {
    let groups = [
        ResistingGroup::HipExtensors,
        ResistingGroup::KneeExtensors,
        ResistingGroup::PlantarFlexors,
        ResistingGroup::SpinalErectors,
    ]
    .map(|g| match g {
        ResistingGroup::HipExtensors
        | ResistingGroup::KneeExtensors
        | ResistingGroup::PlantarFlexors
        | ResistingGroup::SpinalErectors => g,
    });
    let assumptions = [
        Assumption::SagittalPlane,
        Assumption::QuasiStatic,
        Assumption::RigidSegments,
        Assumption::SinglePosture {
            at: Position::Parallel,
        },
        Assumption::SinglePosture {
            at: Position::Upright,
        },
        Assumption::SinglePosture {
            at: Position::Partial,
        },
        Assumption::SolvedBaseline,
        Assumption::RigidTrunkLumbar,
        Assumption::TableDerivedMasses,
        Assumption::TableDerivedLengths,
        Assumption::KneesTrackFeet,
        Assumption::BodyFatIgnored,
        Assumption::StraightBodyUpright,
        Assumption::JointsAdvanceTogether,
    ]
    .map(|a| match a {
        Assumption::SagittalPlane
        | Assumption::QuasiStatic
        | Assumption::RigidSegments
        | Assumption::SinglePosture { .. }
        | Assumption::SolvedBaseline
        | Assumption::RigidTrunkLumbar
        | Assumption::TableDerivedMasses
        | Assumption::TableDerivedLengths
        | Assumption::KneesTrackFeet
        | Assumption::BodyFatIgnored
        | Assumption::StraightBodyUpright
        | Assumption::JointsAdvanceTogether => a,
    });
    let rejections = [
        Rejection::DoesNotBalance {
            unknown: Unknown::TrunkAngle,
        },
        Rejection::DoesNotBalance {
            unknown: Unknown::ShankAngle,
        },
        Rejection::AnatomicallyImplausible {
            joint: Joint::Hip,
            angle_deg: 0.0,
        },
        Rejection::DepthNotReachable {
            limiting: Joint::Ankle,
            needed_deg: 0.0,
            available_deg: 0.0,
        },
    ]
    .map(|r| match r {
        Rejection::DoesNotBalance { .. }
        | Rejection::AnatomicallyImplausible { .. }
        | Rejection::DepthNotReachable { .. } => r,
    });
    let bases = [LoadBasis::Bar { kg: 100.0 }, LoadBasis::BodyweightOnly].map(|b| match b {
        LoadBasis::Bar { .. } | LoadBasis::BodyweightOnly => b,
    });
    let everything = (
        analysed(Some(100.0)),
        analysed(None),
        groups,
        assumptions,
        rejections,
        bases,
        JOINTS,
    );
    serde_json::to_string(&everything).expect("the output namespace serializes")
}

/// B16 — AC5 and AC14 over the **whole** output namespace: four resisting
/// groups, no muscle, and no field name or variant that reads as a verdict
/// about the lifter's body.
#[test]
fn b16_the_output_names_groups_and_never_risk() {
    assert_eq!(
        JOINTS.map(Joint::resisting),
        [
            ResistingGroup::HipExtensors,
            ResistingGroup::KneeExtensors,
            ResistingGroup::PlantarFlexors,
            ResistingGroup::SpinalErectors,
        ]
    );

    let json = every_output_value_as_json().to_lowercase();
    for forbidden in ["risk", "danger", "safe", "injur", "muscle"] {
        assert!(
            !json.contains(forbidden),
            "the output namespace must not say {forbidden:?}: {json}"
        );
    }
}

/// SPEC-0045 §2.8 — what the placement conventions cost, derived by hand.
///
/// Each spread displaces one point along a line the posture fixes, so the
/// moment is linear in it and the eight-corner walk lands on the sum of the
/// three sensitivities. Worked from the table rather than read off the result:
/// nudging L5/S1 moves the pelvis and thorax centres of mass on every load
/// side, and on the lumbar row it moves the joint centre itself.
#[test]
fn placement_uncertainty_is_the_sum_of_its_three_stated_spreads() {
    let body = lifter();
    let solved = solve(&body, Some(100.0), REACHES_PARALLEL);
    let p = solved.posture();
    let result = analyse_baseline(&body, &solved);

    let trunk = (p.shoulder.x.0 - p.hip.x.0).hypot(p.shoulder.y.0 - p.hip.y.0);
    let lean = (p.shoulder.x.0 - p.hip.x.0) / trunk;
    let cost = |kg_metres: f64| kg_metres * G * lean;

    let head = cost(hand_mass(HeadNeck) * COM_PLACEMENT_SPREAD_M);
    let arms = cost(hand_mass(Arms) * COM_PLACEMENT_SPREAD_M);
    // Off the lumbar row the joint does not move, so only the two centres of
    // mass placed from L5/S1 do.
    let l5s1_elsewhere = cost(
        (hand_mass(Pelvis) * (1.0 - WINTER.pelvis.com_from_proximal)
            + hand_mass(ThoraxAbdomen) * WINTER.thorax_abdomen.com_from_proximal)
            * L5S1_PLACEMENT_SPREAD_M,
    );
    // On it, the joint centre itself moves under a load side that mostly does
    // not — **including the bar**, which is on every joint's load side and
    // here is the largest mass of all. That is why the lumbar row's bound is
    // three times the others': an L5/S1 placement this model states rather
    // than cites is the one convention a heavy bar amplifies.
    let bar_kg = p.load.expect("the bar that was supplied").kg.get();
    let l5s1_lumbar = cost(
        (hand_mass(ThoraxAbdomen) * (1.0 - WINTER.thorax_abdomen.com_from_proximal)
            + hand_mass(HeadNeck)
            + hand_mass(Arms)
            + bar_kg)
            * L5S1_PLACEMENT_SPREAD_M,
    );

    for moment in &result.joints {
        let expected = head
            + arms
            + if moment.joint == Joint::Lumbar {
                l5s1_lumbar
            } else {
                l5s1_elsewhere
            };
        assert_close(
            moment.uncertainty.0,
            expected,
            1e-6,
            &format!("{:?} placement half-width", moment.joint),
        );
    }

    let [hip, knee, ankle, _] = result.joints;
    assert_close(knee.uncertainty.0, hip.uncertainty.0, 1e-12, "knee vs hip");
    assert_close(
        ankle.uncertainty.0,
        hip.uncertainty.0,
        1e-12,
        "ankle vs hip",
    );
    assert_close(
        result.uncertainty.l5s1_placement.0,
        L5S1_PLACEMENT_SPREAD_M,
        1e-12,
        "the result names the spread it propagated",
    );
    assert_close(
        result.uncertainty.com_placement.0,
        COM_PLACEMENT_SPREAD_M,
        1e-12,
        "and the other one",
    );
}

// ===========================================================================
// Placement conventions and structure (SPEC-0045 §2.4.2, §2.5)
// ===========================================================================

/// B15 — with a bar the arm COMs sit at the shoulder → bar midpoint (hands on
/// the bar); without one they hang straight down from the shoulder, at the
/// **whole arm's** centre of mass — 0.300 m below it at 1.75 m, not the upper
/// arm's 0.142 the draft used (AC12; SPEC-0045 §2.4.2).
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
    let hang = whole_arm_hang();
    assert_close(hang, 0.300, 5e-4, "the hand arithmetic in whole_arm_hang");
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
