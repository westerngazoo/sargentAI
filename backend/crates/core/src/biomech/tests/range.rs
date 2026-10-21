//! R-0045 slice B1 claims (SPEC-0045 §7: R1–R12) — the squat over its range,
//! from the solved upright toward the §2.6 bottom (§2.6.6).
//!
//! Written **before** the range exists (constitution §4 step 3): the red state
//! is the `todo!()` in `range.rs`.
//!
//! Every expectation is re-derived here, independently of `range.rs` and
//! `squat.rs`: the legs from §2.6.6's formulas restated, the trunk solved by
//! two probes of the parent module's hand model (`hand_mass`, `hand_com` — the
//! slice A pattern), and the upright lean λ from the table's fractions along
//! the straight body, by a route that shares nothing with either. The model is
//! called to produce an expectation in one place only: slice A's `baseline`,
//! for the claims (R1, R7, R10) that *are* "the range returns slice A's
//! answer".
//!
//! **Every claim read across frames is a claim about the convention, not the
//! lifter.** Where a moment peaks, how far the range reaches and whether the
//! trunk is monotone are what *this* leg coupling gives; the lifter's own curve
//! needs their own kinematics (SPEC-0045 §2.6.6).

use std::collections::HashSet;

use super::{
    along, assert_close, assert_point_close, bar, dorsi, hand_com, hand_mass, hand_raw_sum,
    lean_deg, lifter, pt, table_ankle, table_length, Lengths, ANKLE_BEHIND_MID_FOOT, G, HEIGHT_M,
    HIGH_BAR, REACHES_PARALLEL, RESTRICTED,
};
use crate::biomech::ChainSegment::{Arms, HeadNeck, Pelvis, Shank, Thigh, ThoraxAbdomen};
use crate::biomech::{
    baseline, baseline_mechanics, scale, squat_range, Assumption, Body, ChainSegment, FrameCount,
    Joint, Load, MeasuredLengths, Mechanics, Metres, Point, Position, Posture, RangeEnd,
    RangeFrame, Rejection, SquatRange, Unknown, L5S1_UP_THE_TRUNK, WINTER,
};

// ===========================================================================
// Fixture conventions
// ===========================================================================

/// The §2.6.6 fixture's bar: 100 kg, high-bar.
const FIXTURE_BAR_KG: f64 = 100.0;

/// The bodyweight-only ankle at which slice A solves the bottom (trunk 74.5°)
/// but the convention passes horizontal on the way (SPEC-0045 §2.6.6).
const BOTTOM_BALANCES_RANGE_DOES_NOT: f64 = 30.0;

/// The same, under the fixture bar: inside the loaded window, 13.41° to
/// 13.78° (§2.6.6).
const LOADED_BOTTOM_BALANCES_RANGE_DOES_NOT: f64 = 13.6;

/// A dorsiflexion below the fixture's upright lean (4.04° bodyweight, 3.35°
/// under 100 kg): it would drive the shank backwards along the range.
const BELOW_THE_UPRIGHT_LEAN: f64 = 2.0;

/// Everything above the ankle — §2.4.2's ankle list, restated rather than
/// read from `above()`.
const ABOVE_ANKLE: [ChainSegment; 6] = [Shank, Thigh, Pelvis, ThoraxAbdomen, HeadNeck, Arms];

/// A frame count the type accepts.
fn frames(n: u16) -> FrameCount {
    FrameCount::new(n).expect("a frame count inside 2..=241")
}

/// A range that must exist, naming the rejection if it does not.
fn range(body: &Body, bar_kg: Option<f64>, dorsiflexion: f64, n: u16) -> SquatRange {
    squat_range(body, bar_kg.map(bar), dorsi(dorsiflexion), frames(n)).unwrap_or_else(|r| {
        panic!("range (bar {bar_kg:?}, ankle {dorsiflexion}°, n {n}) rejected: {r:?}")
    })
}

/// The §2.6.6 fixture: 1.75 m, 80 kg, 40° of dorsiflexion, 100 kg high-bar.
fn fixture(n: u16) -> SquatRange {
    range(&lifter(), Some(FIXTURE_BAR_KG), REACHES_PARALLEL, n)
}

/// `s` of frame `i` of `n`, as §2.6.6 defines it: `i / (n − 1)`, computed
/// directly.
fn sample(i: u16, n: u16) -> f64 {
    f64::from(i) / f64::from(n - 1)
}

/// How many of `n` sampled frames sit at `s ≤ reach`.
fn sampled_up_to(reach: f64, n: u16) -> usize {
    (0..n).filter(|&i| sample(i, n) <= reach).count()
}

/// The moment resisting extension `j` carries in `m`.
fn moment_of(m: &Mechanics, j: Joint) -> f64 {
    m.joints
        .iter()
        .find(|row| row.joint == j)
        .map(|row| row.moment.0)
        .expect("all four joints are in every result")
}

fn assert_posture_close(actual: &Posture, expected: &Posture, tol: f64, what: &str) {
    assert_point_close(actual.ankle, expected.ankle, tol, &format!("{what}: ankle"));
    assert_point_close(actual.knee, expected.knee, tol, &format!("{what}: knee"));
    assert_point_close(actual.hip, expected.hip, tol, &format!("{what}: hip"));
    assert_point_close(
        actual.shoulder,
        expected.shoulder,
        tol,
        &format!("{what}: shoulder"),
    );
    match (actual.load, expected.load) {
        (Some(a), Some(e)) => {
            assert_close(a.kg.get(), e.kg.get(), 1e-12, &format!("{what}: bar mass"));
            assert_point_close(a.at, e.at, tol, &format!("{what}: bar"));
        }
        (None, None) => {}
        (a, e) => panic!("{what}: load {a:?}, expected {e:?}"),
    }
}

// ===========================================================================
// The convention by hand — §2.6.6's formulas, §2.6.3's balance
// ===========================================================================

/// The §2.6.6 legs at progress `s`, degrees, in the spec's own
/// `(1 − s)·a + s·b` form: `α(s) = (1 − s)·λ + s·δ` for the shank (knee ahead
/// positive) and `φ(s) = −(1 − s)·λ + s·90°` for the femur (hip behind the
/// knee positive).
fn legs_deg(s: f64, lean_deg: f64, dorsiflexion: f64) -> (f64, f64) {
    (
        (1.0 - s) * lean_deg + s * dorsiflexion,
        -(1.0 - s) * lean_deg + s * 90.0,
    )
}

/// A chain built by hand at table lengths: the §2.6.1 ankle, the shank
/// `alpha_deg` forward of vertical, the femur `phi_deg` from vertical with the
/// hip behind the knee positive, the trunk at `sin_trunk` forward of
/// vertical, and the bar — if any — at [`HIGH_BAR`] along the trunk line.
fn range_chain(alpha_deg: f64, phi_deg: f64, sin_trunk: f64, bar_kg: Option<f64>) -> Posture {
    let lengths = Lengths::table();
    let ankle = table_ankle();
    let (sin_shank, cos_shank) = alpha_deg.to_radians().sin_cos();
    let (sin_femur, cos_femur) = phi_deg.to_radians().sin_cos();
    let cos_trunk = sin_trunk.mul_add(-sin_trunk, 1.0).max(0.0).sqrt();
    let knee = pt(
        ankle.x.0 + lengths.shank * sin_shank,
        ankle.y.0 + lengths.shank * cos_shank,
    );
    let hip = pt(
        knee.x.0 - lengths.thigh * sin_femur,
        knee.y.0 + lengths.thigh * cos_femur,
    );
    let shoulder = pt(
        hip.x.0 + lengths.trunk * sin_trunk,
        hip.y.0 + lengths.trunk * cos_trunk,
    );
    Posture {
        ankle,
        knee,
        hip,
        shoulder,
        load: bar_kg.map(|kg| Load {
            kg: bar(kg),
            at: along(hip, shoulder, HIGH_BAR),
        }),
    }
}

/// Everything above the ankle, kg, plus the bar.
fn hand_mass_above_ankle(bar_kg: Option<f64>) -> f64 {
    ABOVE_ANKLE.iter().map(|&s| hand_mass(s)).sum::<f64>() + bar_kg.unwrap_or(0.0)
}

/// `Σ m·x` about the mid-foot line, by hand: what balance sets to zero.
fn hand_first_moment(p: &Posture) -> f64 {
    let segments: f64 = ABOVE_ANKLE
        .iter()
        .map(|&s| hand_mass(s) * hand_com(s, p).x.0)
        .sum();
    segments + p.load.map_or(0.0, |l| l.kg.get() * l.at.x.0)
}

/// The system centre of mass `x`, by hand.
fn hand_com_x(p: &Posture) -> f64 {
    hand_first_moment(p) / hand_mass_above_ankle(p.load.map(|l| l.kg.get()))
}

/// The balancing trunk's `sin θ` at fixed legs, by two probes: every
/// trunk-line `x` is `hip_x + k·trunk·sin θ`, so `Σ m·x` is affine in it.
/// Unclamped, so the caller can see it leave `[0, 1]`.
fn hand_sin_trunk(alpha_deg: f64, phi_deg: f64, bar_kg: Option<f64>) -> f64 {
    let at = |sin_trunk| hand_first_moment(&range_chain(alpha_deg, phi_deg, sin_trunk, bar_kg));
    let vertical = at(0.0);
    -vertical / (at(1.0) - vertical)
}

/// λ, the solved upright lean, worked from the §2.3 fractions along the
/// straight body — sharing nothing with `hand_com` or the model.
///
/// The straight body puts every joint on one line from the ankle, so each
/// centre of mass sits a distance `r` along it and `x = x_ankle + r·sin λ`
/// (the hanging arm is under the shoulder, so its `x` is the shoulder's).
/// Balance is `Σ m·x = 0`, so `sin λ = M·(0.26 × foot) / Σ m·r` with `M` the
/// mass above the ankle.
///
/// By hand at 1.75 m / 80 kg: `r` = 0.2441 (shank), 0.6736 (thigh), 0.9799
/// (pelvis), 1.1306 (thorax & abdomen), 1.4770 (head), 1.3633 (arms hanging);
/// `Σ m·r = 76.312 kg·m` against `77.68 × 0.06916 = 5.3723`, so
/// `sin λ = 0.07040` and **λ = 4.04°**. Under 100 kg the arms move to the
/// shoulder → bar midpoint (1.3507) and the bar adds `100 × 1.3381`:
/// `Σ m·r = 210.017` against `177.68 × 0.06916 = 12.2883`, `sin λ = 0.05851`,
/// **λ = 3.35°**.
fn hand_upright_lean_deg(bar_kg: Option<f64>) -> f64 {
    let (shank, thigh, trunk) = (
        table_length(WINTER.shank),
        table_length(WINTER.thigh),
        table_length(WINTER.trunk),
    );
    let hip = shank + thigh;
    let shoulder = hip + trunk;
    let l5s1 = hip + L5S1_UP_THE_TRUNK * trunk;
    let bar_at = hip + HIGH_BAR * trunk;
    let arms = if bar_kg.is_some() {
        0.5 * (shoulder + bar_at)
    } else {
        shoulder
    };
    let rows = [
        (
            hand_mass(Shank),
            shank * (1.0 - WINTER.shank.com_from_proximal),
        ),
        (
            hand_mass(Thigh),
            hip - WINTER.thigh.com_from_proximal * thigh,
        ),
        (
            hand_mass(Pelvis),
            l5s1 - WINTER.pelvis.com_from_proximal * (l5s1 - hip),
        ),
        (
            hand_mass(ThoraxAbdomen),
            shoulder - WINTER.thorax_abdomen.com_from_proximal * (shoulder - l5s1),
        ),
        (
            hand_mass(HeadNeck),
            shoulder + WINTER.head_neck.com_from_proximal * table_length(WINTER.head_neck),
        ),
        (hand_mass(Arms), arms),
        (bar_kg.unwrap_or(0.0), bar_at),
    ];
    let reach: f64 = rows.iter().map(|&(m, r)| m * r).sum();
    let behind = hand_mass_above_ankle(bar_kg) * ANKLE_BEHIND_MID_FOOT * table_length(WINTER.foot);
    (behind / reach).asin().to_degrees()
}

/// The frame at `s` by hand: the convention's legs, the trunk balanced.
fn hand_frame(s: f64, dorsiflexion: f64, bar_kg: Option<f64>) -> Posture {
    let (alpha, phi) = legs_deg(s, hand_upright_lean_deg(bar_kg), dorsiflexion);
    range_chain(alpha, phi, hand_sin_trunk(alpha, phi, bar_kg), bar_kg)
}

/// `s*` by hand: the first `s` at which the balancing trunk leaves
/// `sin θ ∈ [0, 1]` — a scan at 1/4000, then bisection to 1e-12. `None` when
/// the convention balances all the way to the bottom.
fn hand_reach(dorsiflexion: f64, bar_kg: Option<f64>) -> Option<f64> {
    const STEPS: u16 = 4000;
    let lean = hand_upright_lean_deg(bar_kg);
    let balances = |s: f64| {
        let (alpha, phi) = legs_deg(s, lean, dorsiflexion);
        (0.0..=1.0).contains(&hand_sin_trunk(alpha, phi, bar_kg))
    };
    let mut good = 0.0;
    for i in 1..=STEPS {
        let s = f64::from(i) / f64::from(STEPS);
        if !balances(s) {
            let mut bad = s;
            while bad - good > 1e-12 {
                let mid = 0.5 * (good + bad);
                if balances(mid) {
                    good = mid;
                } else {
                    bad = mid;
                }
            }
            return Some(good);
        }
        good = s;
    }
    None
}

/// The moment resisting extension about `j`, by hand: §2.4.2's load sides and
/// §2.4.1's sign rule (the knee flips), both restated.
fn hand_moment(j: Joint, p: &Posture) -> f64 {
    let (joint_x, side, sign): (f64, &[ChainSegment], f64) = match j {
        Joint::Hip => (p.hip.x.0, &[Pelvis, ThoraxAbdomen, HeadNeck, Arms], 1.0),
        Joint::Knee => (
            p.knee.x.0,
            &[Thigh, Pelvis, ThoraxAbdomen, HeadNeck, Arms],
            -1.0,
        ),
        Joint::Ankle => (p.ankle.x.0, &ABOVE_ANKLE, 1.0),
        Joint::Lumbar => (
            along(p.hip, p.shoulder, L5S1_UP_THE_TRUNK).x.0,
            &[ThoraxAbdomen, HeadNeck, Arms],
            1.0,
        ),
    };
    sign * hand_raw_sum(p, joint_x, side)
}

/// Degrees from vertical of `from → to`, forward positive.
fn from_vertical(from: Point, to: Point) -> f64 {
    (to.x.0 - from.x.0).atan2(to.y.0 - from.y.0).to_degrees()
}

/// The shank's angle, read off the points.
fn shank_of(p: &Posture) -> f64 {
    from_vertical(p.ankle, p.knee)
}

/// Knee flexion read off the points: the shank forward of vertical plus the
/// femur behind it (`knee → hip` leans back by φ).
fn knee_flexion_of(p: &Posture) -> f64 {
    from_vertical(p.ankle, p.knee) - from_vertical(p.knee, p.hip)
}

/// Hip flexion read off the points: the trunk forward of vertical plus the
/// femur behind it — `90° + lean` at parallel, 0 for a straight body.
fn hip_flexion_of(p: &Posture) -> f64 {
    from_vertical(p.hip, p.shoulder) - from_vertical(p.knee, p.hip)
}

// ===========================================================================
// R1–R3 · The two ends, and every frame between them balances
// ===========================================================================

/// R1 — the last frame **is** slice A's baseline: the same posture, the same
/// four moments, the same assumptions (`SinglePosture { at: Parallel }` among
/// them), and `end: RangeEnd::Bottom` (SPEC-0045 §2.6.6).
///
/// Asserted with exact equality on purpose. At `s = 1` the formulas give the
/// baseline's geometry only up to f64 rounding (`cos 90°` is 6e-17, and the
/// spec warns that `a + s·(b − a)` misses `b`), so bit-equality is the claim
/// that the frame was *taken* from `squat::baseline`, not rebuilt.
#[test]
fn r1_the_last_frame_is_slice_a_baseline() {
    let body = lifter();
    for bar_kg in [None, Some(FIXTURE_BAR_KG)] {
        let ankle = dorsi(REACHES_PARALLEL);
        let solved = baseline(&body, bar_kg.map(bar), ankle).expect("40° reaches parallel");
        let analysed = baseline_mechanics(&body, bar_kg.map(bar), ankle).expect("and solves");
        for n in [2, 6, 241] {
            let what = format!("bar {bar_kg:?}, n {n}");
            let got = range(&body, bar_kg, REACHES_PARALLEL, n);
            assert_eq!(got.end, RangeEnd::Bottom, "{what}");
            assert_eq!(
                got.frames.len(),
                usize::from(n),
                "{what}: reaching the bottom returns every sampled frame"
            );
            let last = got.frames.last().expect("frames");
            assert_eq!(last.posture, *solved.posture(), "{what}: the posture");
            assert_eq!(last.mechanics, analysed, "{what}: the mechanics");
            assert!(
                last.mechanics
                    .assumptions
                    .contains(&Assumption::SinglePosture {
                        at: Position::Parallel
                    }),
                "{what}: the bottom is at parallel"
            );
            assert_eq!(last.progress.to_bits(), 1.0_f64.to_bits(), "{what}: s");
            assert_close(last.shank_deg, REACHES_PARALLEL, 1e-12, &what);
            assert_close(last.knee_flexion_deg, REACHES_PARALLEL + 90.0, 1e-12, &what);
            let lean = lean_deg(solved.posture());
            assert_close(last.trunk_lean_deg, lean, 1e-9, &what);
            assert_close(last.hip_flexion_deg, 90.0 + lean, 1e-9, &what);
        }
    }
}

/// R2 — the first frame is the solved upright: knee and hip flexion 0, the
/// trunk leaning exactly as far as the shank, every joint on one straight line
/// from the ankle, and the system centre of mass over the mid-foot within
/// 1 mm. The lean is worked by hand in [`hand_upright_lean_deg`]: **4.04°**
/// bodyweight-only, **3.35°** under 100 kg (SPEC-0045 §2.6.6).
///
/// λ is the body's and the load's, never the ankle's — at `s = 0` the
/// formulas drop `δ` — so the upright is the same frame at any dorsiflexion,
/// including one whose range later stops.
#[test]
fn r2_the_first_frame_is_the_solved_straight_body_upright() {
    let body = lifter();
    for (bar_kg, by_hand) in [(None, 4.04), (Some(FIXTURE_BAR_KG), 3.35)] {
        let what = format!("bar {bar_kg:?}");
        let lean = hand_upright_lean_deg(bar_kg);
        assert_close(
            lean,
            by_hand,
            0.005,
            "the hand arithmetic in the doc comment",
        );

        let got = range(&body, bar_kg, REACHES_PARALLEL, 6);
        let first = got.frames.first().expect("the upright is always a frame");
        assert_eq!(first.progress.to_bits(), 0.0_f64.to_bits(), "{what}: s");
        assert_close(first.knee_flexion_deg, 0.0, 1e-12, &what);
        assert_close(first.hip_flexion_deg, 0.0, 1e-9, &what);
        assert_close(first.shank_deg, lean, 1e-9, &format!("{what}: λ"));
        assert_close(
            first.trunk_lean_deg,
            first.shank_deg,
            1e-9,
            &format!("{what}: trunk lean equals shank lean"),
        );
        let straight = range_chain(lean, -lean, lean.to_radians().sin(), bar_kg);
        assert_posture_close(
            &first.posture,
            &straight,
            1e-9,
            &format!("{what}: straight body"),
        );
        assert_close(knee_flexion_of(&first.posture), 0.0, 1e-9, &what);
        assert_close(hip_flexion_of(&first.posture), 0.0, 1e-9, &what);
        assert_close(
            hand_com_x(&first.posture),
            0.0,
            1e-3,
            &format!("{what}: system COM over the mid-foot"),
        );

        for dorsiflexion in [RESTRICTED, BOTTOM_BALANCES_RANGE_DOES_NOT, 55.0] {
            let other = range(&body, bar_kg, dorsiflexion, 6);
            let upright = other.frames.first().expect("the upright is always a frame");
            let at = format!("{what}: the upright at {dorsiflexion}°");
            assert_posture_close(&upright.posture, &first.posture, 1e-12, &at);
            for j in [Joint::Hip, Joint::Knee, Joint::Ankle, Joint::Lumbar] {
                assert_close(
                    moment_of(&upright.mechanics, j),
                    moment_of(&first.mechanics, j),
                    1e-9,
                    &format!("{at}: {j:?}"),
                );
            }
        }
    }
}

/// R3 — every frame balances: the system centre of mass is over the mid-foot
/// within 1 mm in every frame of every range, at both ends and at both kinds
/// of stop, and the trunk stays between vertical and horizontal.
///
/// And every frame is the convention's own posture: rebuilt by hand from
/// §2.6.6's formulas with the trunk solved by two probes, it agrees to 1e-9 m.
/// A frame that balanced but interpolated the sines, or the trunk, would pass
/// the first half of this claim and fail the second.
#[test]
fn r3_every_frame_balances_over_the_mid_foot() {
    let body = lifter();
    let cases = [
        (None, REACHES_PARALLEL),
        (Some(FIXTURE_BAR_KG), REACHES_PARALLEL),
        (Some(FIXTURE_BAR_KG), RESTRICTED),
        (None, RESTRICTED),
        (None, BOTTOM_BALANCES_RANGE_DOES_NOT),
        (Some(FIXTURE_BAR_KG), LOADED_BOTTOM_BALANCES_RANGE_DOES_NOT),
    ];
    for (bar_kg, dorsiflexion) in cases {
        let got = range(&body, bar_kg, dorsiflexion, 241);
        assert!(!got.frames.is_empty(), "the upright is always a frame");
        for frame in &got.frames {
            let what = format!("bar {bar_kg:?}, {dorsiflexion}°, s {}", frame.progress);
            assert_close(hand_com_x(&frame.posture), 0.0, 1e-3, &what);
            assert!(
                (0.0..=90.0).contains(&frame.trunk_lean_deg),
                "{what}: trunk {}° is outside vertical-to-horizontal",
                frame.trunk_lean_deg
            );
            let by_hand = hand_frame(frame.progress, dorsiflexion, bar_kg);
            assert_posture_close(&frame.posture, &by_hand, 1e-9, &what);
        }
    }
}

// ===========================================================================
// R4–R6 · What the frames say, under this convention
// ===========================================================================

/// R4 — the ankle moment is the same in every frame, and that tells nothing
/// about the posture. Under the balance constraint `Σ m·x = 0`, so the moment
/// about the ankle is `g·Σ m·(x − x_ankle) = M·g·(0.26 × 0.152 H)`: the weight
/// above the ankle times the §2.6.1 mid-foot convention, 0.06916 m. A theorem,
/// at any δ, in slice A as here — never a finding about the plantar flexors.
///
/// By hand: 77.68 kg above the ankle (shank 7.44, thigh 16.0, pelvis 11.36,
/// thorax & abdomen 28.4, head 6.48, arms 8.0). Under 100 kg,
/// `177.68 × 9.81 × 0.06916` = **120.55 N·m**; bodyweight-only,
/// `77.68 × 9.81 × 0.06916` = **52.70 N·m**. The spread across frames is held
/// to 1e-9 N·m, as §2.6.6 states it.
#[test]
fn r4_the_ankle_moment_is_the_weight_times_the_mid_foot_convention_in_every_frame() {
    let body = lifter();
    let lever = ANKLE_BEHIND_MID_FOOT * table_length(WINTER.foot);
    for (bar_kg, by_hand) in [(Some(FIXTURE_BAR_KG), 120.55), (None, 52.70)] {
        let expected = hand_mass_above_ankle(bar_kg) * G * lever;
        assert_close(
            expected,
            by_hand,
            0.005,
            "the hand arithmetic in the doc comment",
        );
        for dorsiflexion in [
            REACHES_PARALLEL,
            RESTRICTED,
            BOTTOM_BALANCES_RANGE_DOES_NOT,
            55.0,
        ] {
            let what = format!("bar {bar_kg:?}, {dorsiflexion}°");
            let got = range(&body, bar_kg, dorsiflexion, 241);
            let ankle: Vec<f64> = got
                .frames
                .iter()
                .map(|f| moment_of(&f.mechanics, Joint::Ankle))
                .collect();
            let low = ankle.iter().copied().fold(f64::INFINITY, f64::min);
            let high = ankle.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            assert!(
                high - low <= 1e-9,
                "{what}: the ankle row moved by {} N·m across frames",
                high - low
            );
            assert_close(low, expected, 1e-9, &what);
        }
    }
}

/// R5 — knee flexion is exactly `s·(δ + 90°)` — an identity of the
/// convention, independent of λ and the load, which is what gives `s` a
/// geometric meaning — and it rises strictly with `s`. The frame's angles are
/// its posture's: knee, shank `α(s)`, hip and trunk read off the points agree
/// with the fields. ("Exactly" is held to 1e-12°, f64 rounding of `α + φ`.)
///
/// Hip flexion is not monotone in general; **for the fixture** (100 kg, 40°)
/// it is, and the claim is stated for the fixture only.
#[test]
fn r5_knee_flexion_is_the_progress_times_the_bottom_knee_angle() {
    let body = lifter();
    let cases = [
        (Some(FIXTURE_BAR_KG), REACHES_PARALLEL),
        (None, REACHES_PARALLEL),
        (None, RESTRICTED),
        (None, BOTTOM_BALANCES_RANGE_DOES_NOT),
    ];
    for (bar_kg, dorsiflexion) in cases {
        let lean = hand_upright_lean_deg(bar_kg);
        let mut previous = f64::NEG_INFINITY;
        for f in &range(&body, bar_kg, dorsiflexion, 241).frames {
            let what = format!("bar {bar_kg:?}, {dorsiflexion}°, s {}", f.progress);
            let expected = f.progress * (dorsiflexion + 90.0);
            assert_close(f.knee_flexion_deg, expected, 1e-12, &what);
            assert_close(knee_flexion_of(&f.posture), expected, 1e-9, &what);
            assert!(
                f.knee_flexion_deg > previous,
                "{what}: knee flexion must rise strictly"
            );
            previous = f.knee_flexion_deg;

            let (alpha, _) = legs_deg(f.progress, lean, dorsiflexion);
            assert_close(f.shank_deg, alpha, 1e-9, &format!("{what}: α(s)"));
            assert_close(shank_of(&f.posture), alpha, 1e-9, &format!("{what}: shank"));
            assert_close(f.hip_flexion_deg, hip_flexion_of(&f.posture), 1e-9, &what);
            assert_close(f.trunk_lean_deg, lean_deg(&f.posture), 1e-9, &what);
        }
    }

    let mut previous = f64::NEG_INFINITY;
    for f in &fixture(241).frames {
        assert!(
            f.hip_flexion_deg > previous,
            "the fixture's hip flexion rises strictly: {}° at s {} after {previous}°",
            f.hip_flexion_deg,
            f.progress
        );
        previous = f.hip_flexion_deg;
    }
}

/// R6 — the fixture at six frames reproduces SPEC-0045 §2.6.6's table, moments
/// in N·m resisting extension, with its largest hip moment at `s = 0.8`
/// (373.6 N·m).
///
/// **A claim about the convention, not the lifter.** Interpolating the sines
/// instead of the angles would make the same hip rise the whole way to
/// 333.9 N·m at the bottom; the peak at `s = 0.8` is where *this* coupling,
/// sampled at `n = 6`, puts it, and the trunk's 35.9° → 31.6° turn is the
/// convention's too.
///
/// The table is re-derived by hand first and must agree to the 0.1 it prints;
/// then the model must match both — the table to 0.1, the hand model to 1e-6.
/// Every column, hip flexion included, is printed to 0.1 and held to 0.1
/// (the hip column was printed to the whole degree until `f3c9c42`).
#[test]
fn r6_the_fixture_reproduces_the_spec_table_under_this_convention() {
    // s, knee flex, hip flex, trunk, hip, knee, ankle, L5/S1 — SPEC-0045 §2.6.6
    const TABLE: [[f64; 8]; 6] = [
        [0.0, 0.0, 0.0, 3.4, 37.2, -77.4, 120.5, 25.4],
        [0.2, 26.0, 30.4, 15.1, 165.4, 16.0, 120.5, 112.8],
        [0.4, 52.0, 59.8, 25.8, 276.7, 107.3, 120.5, 188.7],
        [0.6, 78.0, 86.2, 33.5, 351.3, 194.8, 120.5, 239.6],
        [0.8, 104.0, 107.3, 35.9, 373.6, 277.2, 120.5, 254.8],
        [1.0, 130.0, 121.6, 31.6, 333.9, 353.0, 120.5, 227.7],
    ];
    const JOINT_COLUMNS: [Joint; 4] = [Joint::Hip, Joint::Knee, Joint::Ankle, Joint::Lumbar];

    // The table by hand first, so the expectation is checked before the model
    // is ever called.
    let by_hand: Vec<Posture> = TABLE
        .iter()
        .map(|row| hand_frame(row[0], REACHES_PARALLEL, Some(FIXTURE_BAR_KG)))
        .collect();
    for (p, row) in by_hand.iter().zip(TABLE) {
        let what = format!("s {}, by hand vs the table", row[0]);
        assert_close(
            knee_flexion_of(p),
            row[1],
            0.1,
            &format!("{what}: knee flex"),
        );
        assert_close(hip_flexion_of(p), row[2], 0.1, &format!("{what}: hip flex"));
        assert_close(lean_deg(p), row[3], 0.1, &format!("{what}: trunk"));
        for (j, printed) in JOINT_COLUMNS.into_iter().zip(&row[4..]) {
            assert_close(hand_moment(j, p), *printed, 0.1, &format!("{what}: {j:?}"));
        }
    }

    let got = fixture(6);
    assert_eq!(got.end, RangeEnd::Bottom);
    assert_eq!(got.frames.len(), TABLE.len());
    for ((frame, row), hand) in got.frames.iter().zip(TABLE).zip(&by_hand) {
        let [s, knee_flexion, hip_flexion, trunk] = [row[0], row[1], row[2], row[3]];
        let what = format!("s {s}");
        assert_eq!(frame.progress.to_bits(), s.to_bits(), "{what}: progress");
        assert_close(
            frame.knee_flexion_deg,
            knee_flexion,
            0.1,
            &format!("{what}: knee flex"),
        );
        assert_close(
            frame.hip_flexion_deg,
            hip_flexion,
            0.1,
            &format!("{what}: hip flex"),
        );
        assert_close(frame.trunk_lean_deg, trunk, 0.1, &format!("{what}: trunk"));
        for (j, printed) in JOINT_COLUMNS.into_iter().zip(&row[4..]) {
            let model = moment_of(&frame.mechanics, j);
            assert_close(model, *printed, 0.1, &format!("{what}: {j:?} vs the table"));
            assert_close(
                model,
                hand_moment(j, hand),
                1e-6,
                &format!("{what}: {j:?} vs the hand model"),
            );
        }
    }

    let hip = |f: &RangeFrame| moment_of(&f.mechanics, Joint::Hip);
    let peak = got
        .frames
        .iter()
        .max_by(|a, b| hip(a).total_cmp(&hip(b)))
        .expect("six frames");
    assert_eq!(
        peak.progress.to_bits(),
        0.8_f64.to_bits(),
        "where this convention peaks"
    );
    assert_close(hip(peak), 373.6, 0.1, "the peak, under this convention");
}

// ===========================================================================
// R7, R10 · Where the range ends
// ===========================================================================

/// The frames of a stopped range: the sampled `s ≤ reached`, in order, and
/// exactly those — the largest balanced prefix.
fn assert_prefix(got: &SquatRange, reached: f64, by_hand: f64, n: u16, what: &str) {
    assert_eq!(
        got.frames.len(),
        sampled_up_to(by_hand, n),
        "{what}: every sampled s ≤ s*, and nothing past it"
    );
    for (i, frame) in (0..n).zip(&got.frames) {
        assert_eq!(
            frame.progress.to_bits(),
            sample(i, n).to_bits(),
            "{what}: frame {i} sits at i / (n − 1)"
        );
        assert!(
            frame.progress <= reached,
            "{what}: a frame at s {} past s* {reached}",
            frame.progress
        );
    }
}

/// R7 — **the ankle stops it.** The fixture at 25°, bodyweight-only: the
/// trunk reaches horizontal at `s* ≈ 0.565` (0.5648 by hand, knee 65°), and
/// `bottom` carries slice A's own `DepthNotReachable` with `needed_deg` 28.7.
///
/// `s*` is found by a fixed scan plus bisection to 1e-9, so it does not depend
/// on how many frames are asked for: read literally, `reached` is the **same
/// f64** for `n` = 2, 6, 101 and 241 — a value merely within 1e-9 would let
/// the frame count leak into it. It is the convention's answer to "how deep",
/// not the lifter's: that is the depth knob (slice B).
#[test]
fn r7_a_restricted_ankle_stops_the_range_and_slice_a_says_why() {
    let body = lifter();
    let by_hand = hand_reach(RESTRICTED, None).expect("the convention passes horizontal at 25°");
    assert_close(by_hand, 0.565, 0.001, "s*, by hand");
    assert_close(by_hand * (RESTRICTED + 90.0), 65.0, 0.1, "the knee at s*");
    let slice_a = baseline(&body, None, dorsi(RESTRICTED))
        .map(|_| ())
        .expect_err("slice A rejects 25° bodyweight-only");

    let mut reach_by_n = Vec::new();
    for n in [2, 6, 101, 241] {
        let what = format!("n {n}");
        let got = range(&body, None, RESTRICTED, n);
        let RangeEnd::Stopped { reached, bottom } = got.end else {
            panic!("{what}: expected the range to stop, got {:?}", got.end)
        };
        assert_close(reached, by_hand, 1e-7, &format!("{what}: s*"));
        assert_eq!(bottom, Some(slice_a), "{what}: bottom is slice A's outcome");
        match bottom {
            Some(Rejection::DepthNotReachable {
                limiting: Joint::Ankle,
                needed_deg,
                available_deg,
            }) => {
                assert_close(needed_deg, 28.7, 0.1, &format!("{what}: needed"));
                assert_close(
                    available_deg,
                    RESTRICTED,
                    1e-12,
                    &format!("{what}: available"),
                );
            }
            other => panic!("{what}: expected the ankle to be named, got {other:?}"),
        }
        assert_prefix(&got, reached, by_hand, n, &what);
        reach_by_n.push(reached.to_bits());
    }
    assert!(
        reach_by_n.windows(2).all(|pair| pair[0] == pair[1]),
        "s* must not depend on the frame count: {:?}",
        reach_by_n
            .iter()
            .map(|&b| f64::from_bits(b))
            .collect::<Vec<_>>()
    );
}

/// R10 — **the convention stops it, though the bottom balances.** The fixture
/// at 30°, bodyweight-only: slice A solves the bottom (trunk 74.5°), but under
/// this convention the trunk passes horizontal for `s` between about 0.69 and
/// 0.94 — by hand, out of `[0, 1]` at `s = 0.8` and back inside by
/// `s = 0.95`. The range stops at `s* ≈ 0.69` (0.6920 by hand) with
/// `bottom: None`: the stop is attributed to the convention, not the ankle.
///
/// The same holds under the fixture bar at 13.6°, inside the loaded window
/// (the bottom balances from 13.4°).
#[test]
fn r10_the_convention_can_stop_a_range_whose_bottom_balances() {
    let lean = hand_upright_lean_deg(None);
    let sin_trunk_at = |s| {
        let (alpha, phi) = legs_deg(s, lean, BOTTOM_BALANCES_RANGE_DOES_NOT);
        hand_sin_trunk(alpha, phi, None)
    };
    assert!(
        sin_trunk_at(0.8) > 1.0,
        "by hand: past horizontal mid-range"
    );
    assert!(
        (0.0..=1.0).contains(&sin_trunk_at(0.95)),
        "by hand: back inside before the bottom — it is the path that fails"
    );

    let body = lifter();
    let cases = [
        (None, BOTTOM_BALANCES_RANGE_DOES_NOT, Some((0.69, 0.005))),
        (
            Some(FIXTURE_BAR_KG),
            LOADED_BOTTOM_BALANCES_RANGE_DOES_NOT,
            None,
        ),
    ];
    // Every premise and every expectation first, before the model is called.
    let expected: Vec<(Option<f64>, f64, f64)> = cases
        .into_iter()
        .map(|(bar_kg, dorsiflexion, stated)| {
            let what = format!("bar {bar_kg:?}, {dorsiflexion}°");
            assert!(
                baseline(&body, bar_kg.map(bar), dorsi(dorsiflexion)).is_ok(),
                "{what}: slice A must solve the bottom, or this is R7"
            );
            let by_hand =
                hand_reach(dorsiflexion, bar_kg).expect("the convention passes horizontal");
            if let Some((spec, tol)) = stated {
                assert_close(by_hand, spec, tol, &format!("{what}: the spec's s*"));
            }
            (bar_kg, dorsiflexion, by_hand)
        })
        .collect();

    for (bar_kg, dorsiflexion, by_hand) in expected {
        let what = format!("bar {bar_kg:?}, {dorsiflexion}°");
        let mut reach_by_n = Vec::new();
        for n in [6, 241] {
            let at = format!("{what}, n {n}");
            let got = range(&body, bar_kg, dorsiflexion, n);
            let RangeEnd::Stopped { reached, bottom } = got.end else {
                panic!("{at}: expected the range to stop, got {:?}", got.end)
            };
            assert_eq!(
                bottom, None,
                "{at}: the convention is blamed, not the ankle"
            );
            assert_close(reached, by_hand, 1e-7, &format!("{at}: s*"));
            assert_prefix(&got, reached, by_hand, n, &at);
            reach_by_n.push(reached.to_bits());
        }
        assert_eq!(
            reach_by_n[0], reach_by_n[1],
            "{what}: s* must not depend on n"
        );
    }
}

// ===========================================================================
// R8–R9 · What each frame admits to, and that it does not depend on n
// ===========================================================================

/// The assumptions every result carries whatever produced its posture — what
/// `analyse_baseline` emits for the table lifter, less its source half.
const MODEL_ASSUMPTIONS: [Assumption; 7] = [
    Assumption::SagittalPlane,
    Assumption::QuasiStatic,
    Assumption::RigidSegments,
    Assumption::RigidTrunkLumbar,
    Assumption::TableDerivedMasses,
    Assumption::TableDerivedLengths,
    Assumption::BodyFatIgnored,
];

fn assert_assumptions(frame: &RangeFrame, source: [Assumption; 3], what: &str) {
    let got = &frame.mechanics.assumptions;
    let want: HashSet<Assumption> = MODEL_ASSUMPTIONS.into_iter().chain(source).collect();
    assert_eq!(
        got.iter().copied().collect::<HashSet<_>>(),
        want,
        "{what}: s {}",
        frame.progress
    );
    assert_eq!(
        got.len(),
        want.len(),
        "{what}: no assumption twice: {got:?}"
    );
}

/// R8 — each frame admits to where in the range it sits (SPEC-0045 §2.6.6):
/// strictly between, `SinglePosture { at: Partial }`, `SolvedBaseline` and
/// `JointsAdvanceTogether`; the upright, `SinglePosture { at: Upright }`,
/// `SolvedBaseline` and `StraightBodyUpright` — no joint advances there; the
/// bottom, exactly `analyse_baseline`'s list and neither of the new ones. On
/// top of the source half, every frame carries the model's own assumptions
/// (AC12).
#[test]
fn r8_each_frame_states_where_in_the_range_it_sits() {
    let body = lifter();
    let upright = [
        Assumption::SinglePosture {
            at: Position::Upright,
        },
        Assumption::SolvedBaseline,
        Assumption::StraightBodyUpright,
    ];
    let between = [
        Assumption::SinglePosture {
            at: Position::Partial,
        },
        Assumption::SolvedBaseline,
        Assumption::JointsAdvanceTogether,
    ];
    let range_only = [
        Assumption::SinglePosture {
            at: Position::Upright,
        },
        Assumption::SinglePosture {
            at: Position::Partial,
        },
        Assumption::StraightBodyUpright,
        Assumption::JointsAdvanceTogether,
    ];
    let cases = [
        (Some(FIXTURE_BAR_KG), REACHES_PARALLEL, true),
        (None, REACHES_PARALLEL, true),
        (None, RESTRICTED, false),
    ];
    for (bar_kg, dorsiflexion, reaches_bottom) in cases {
        let what = format!("bar {bar_kg:?}, {dorsiflexion}°");
        let got = range(&body, bar_kg, dorsiflexion, 6);
        let (first, rest) = got.frames.split_first().expect("the upright");
        assert_assumptions(first, upright, &format!("{what}: upright"));
        let middle = if reaches_bottom {
            let (last, middle) = rest.split_last().expect("the bottom");
            let slice_a = baseline_mechanics(&body, bar_kg.map(bar), dorsi(dorsiflexion))
                .expect("the bottom solves");
            assert_eq!(
                last.mechanics.assumptions, slice_a.assumptions,
                "{what}: bottom"
            );
            for a in range_only {
                assert!(
                    !last.mechanics.assumptions.contains(&a),
                    "{what}: the bottom carries {a:?}"
                );
            }
            middle
        } else {
            rest
        };
        assert!(
            !middle.is_empty(),
            "{what}: the claim needs frames strictly between"
        );
        for f in middle {
            assert_assumptions(f, between, &format!("{what}: between"));
        }
    }
}

/// R9 — a frame at a given `s` is bit-identical whatever `n` is: `s` is
/// `i / (n − 1)` computed directly, never accumulated, and IEEE division is
/// correctly rounded, so `1/5`, `2/10` and `48/240` are one f64. Compared both
/// as values and as serialized text, which tells `0.0` from `-0.0`.
#[test]
fn r9_a_frame_does_not_depend_on_how_many_were_asked_for() {
    let body = lifter();
    let same_s: [[(u16, usize); 3]; 2] = [
        [(6, 1), (11, 2), (241, 48)],    // s = 0.2
        [(3, 1), (101, 50), (241, 120)], // s = 0.5
    ];
    for (bar_kg, dorsiflexion) in [(Some(FIXTURE_BAR_KG), REACHES_PARALLEL), (None, RESTRICTED)] {
        for group in same_s {
            let picked: Vec<RangeFrame> = group
                .iter()
                .map(|&(n, i)| {
                    range(&body, bar_kg, dorsiflexion, n)
                        .frames
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| panic!("n {n} has no frame {i}"))
                })
                .collect();
            let reference = &picked[0];
            let reference_text = serde_json::to_string(reference).expect("a frame serializes");
            for (frame, (n, _)) in picked.iter().zip(group).skip(1) {
                let what = format!(
                    "bar {bar_kg:?}, {dorsiflexion}°, s {}, n {n}",
                    reference.progress
                );
                assert_eq!(frame, reference, "{what}");
                assert_eq!(
                    serde_json::to_string(frame).expect("a frame serializes"),
                    reference_text,
                    "{what}: bit for bit"
                );
            }
        }
    }
}

/// R9 — `FrameCount` refuses fewer than 2 frames (a range has two ends) and
/// more than 241 (a resource bound), and keeps what it accepts.
#[test]
fn r9_frame_count_is_two_to_two_hundred_and_forty_one() {
    for refused in [0, 1, 242, 1000, u16::MAX] {
        assert_eq!(FrameCount::new(refused), None, "{refused} frames");
    }
    for accepted in [2, 3, 6, 101, 240, 241] {
        assert_eq!(
            FrameCount::new(accepted).map(FrameCount::get),
            Some(accepted),
            "{accepted} frames"
        );
    }
}

// ===========================================================================
// R11–R12 · Rejected outright, with no frames (AC9, AC13)
// ===========================================================================

/// R11 — a dorsiflexion below the upright lean (`δ < λ`) would drive the shank
/// *backwards* along the range, so the whole range is rejected with no frames:
/// `DepthNotReachable { limiting: Ankle, needed_deg: λ, available_deg: δ }`.
/// λ is by hand ([`hand_upright_lean_deg`]): 4.04° bodyweight-only, 3.35°
/// under 100 kg. And `needed_deg` is the least ankle that works: a tenth of a
/// degree more is a range.
#[test]
fn r11_an_ankle_below_the_upright_lean_has_no_range() {
    let body = lifter();
    for bar_kg in [None, Some(FIXTURE_BAR_KG)] {
        let lean = hand_upright_lean_deg(bar_kg);
        for available in [0.0, BELOW_THE_UPRIGHT_LEAN] {
            let what = format!("bar {bar_kg:?}, {available}°");
            match squat_range(&body, bar_kg.map(bar), dorsi(available), frames(6)) {
                Err(Rejection::DepthNotReachable {
                    limiting: Joint::Ankle,
                    needed_deg,
                    available_deg,
                }) => {
                    assert_close(needed_deg, lean, 1e-9, &format!("{what}: needed is λ"));
                    assert_close(
                        available_deg,
                        available,
                        1e-12,
                        &format!("{what}: available"),
                    );
                }
                other => panic!("{what}: expected the ankle to be named, got {other:?}"),
            }
        }
        let just_enough = range(&body, bar_kg, lean + 0.1, 6);
        assert!(
            !just_enough.frames.is_empty(),
            "bar {bar_kg:?}: λ + 0.1° must have at least the upright"
        );
    }
}

/// R12 — a body with no mass and no bar cannot stand balanced: the upright's
/// balance equation is `0 = 0`, its lean is undetermined, and the upright's
/// unknown is that lean — the shank — so the whole range is
/// `DoesNotBalance { unknown: ShankAngle }` at any ankle (AC9). With λ
/// undetermined, `δ < λ` has no truth value, so this fires even at 0°.
///
/// Slice A names the **trunk** for the same body, because the baseline's
/// unknown is the trunk. The two answers differ on purpose.
#[test]
fn r12_a_body_that_cannot_stand_balanced_has_no_range() {
    let massless = scale(Metres(HEIGHT_M), 0.0, &MeasuredLengths::default());
    for dorsiflexion in [
        0.0,
        BELOW_THE_UPRIGHT_LEAN,
        RESTRICTED,
        REACHES_PARALLEL,
        90.0,
    ] {
        assert_eq!(
            squat_range(&massless, None, dorsi(dorsiflexion), frames(6)),
            Err(Rejection::DoesNotBalance {
                unknown: Unknown::ShankAngle
            }),
            "{dorsiflexion}°"
        );
    }
    assert_eq!(
        baseline(&massless, None, dorsi(REACHES_PARALLEL)).map(|_| ()),
        Err(Rejection::DoesNotBalance {
            unknown: Unknown::TrunkAngle
        }),
        "slice A's answer for the same body"
    );
}

// ===========================================================================
// Serialize and AC14 over the range's namespace
// ===========================================================================

/// SPEC-0045 §2.6.6 and AC14 — the range serializes, a frame's points arrive
/// as metres a reel or a video overlay can draw without recomputing them, and
/// nothing in the range's namespace reads as a verdict about the lifter. Every
/// `RangeEnd` and `Position` value is listed behind an exhaustive `match`, so
/// a new variant cannot skip the check by being forgotten.
#[test]
fn the_range_serializes_its_points_and_never_says_risk() {
    let reached = fixture(6);
    let stopped = range(&lifter(), None, RESTRICTED, 6);
    let ends = [
        RangeEnd::Bottom,
        RangeEnd::Stopped {
            reached: 0.5,
            bottom: None,
        },
        RangeEnd::Stopped {
            reached: 0.5,
            bottom: Some(Rejection::DepthNotReachable {
                limiting: Joint::Ankle,
                needed_deg: 0.0,
                available_deg: 0.0,
            }),
        },
    ]
    .map(|e| match e {
        RangeEnd::Bottom | RangeEnd::Stopped { .. } => e,
    });
    let positions = [Position::Parallel, Position::Upright, Position::Partial].map(|p| match p {
        Position::Parallel | Position::Upright | Position::Partial => p,
    });

    let json = serde_json::to_value((&reached, &stopped, ends, positions))
        .expect("the range namespace serializes");
    let knee = &json[0]["frames"][0]["posture"]["knee"];
    let upright = reached.frames.first().expect("the upright").posture;
    for (axis, metres) in [("x", upright.knee.x.0), ("y", upright.knee.y.0)] {
        assert_eq!(
            knee[axis].as_f64().map(f64::to_bits),
            Some(metres.to_bits()),
            "the upright knee's {axis}, as metres: {knee}"
        );
    }

    let text = json.to_string().to_lowercase();
    for forbidden in ["risk", "danger", "safe", "injur", "muscle"] {
        assert!(
            !text.contains(forbidden),
            "the range namespace must not say {forbidden:?}: {text}"
        );
    }
}

/// R13 — a hole narrower than any fixed scan step is still found (SPEC-0045
/// §2.6.6).
///
/// Near the window's upper edge the stretch where the balancing trunk would
/// pass horizontal shrinks to nothing. At 31.258786790912° of dorsiflexion,
/// bodyweight-only, the trunk's sine peaks at 1.000000001 at `s ≈ 0.809865`,
/// and the first crossing is at `s* = 0.809839`: a hole about 5e-5 of the
/// range wide, a fifth of a 1/4096 scan step. Derived independently of the
/// crate (the §2.6.6 formulas in a separate script, fine scan and
/// bisection). Found by the qa sign-off: the first implementation's scan
/// stepped over it, reported `Bottom`, and returned frames cut short at the
/// first sample that fell inside.
#[test]
fn r13_a_hole_narrower_than_the_scan_step_still_stops_the_range() {
    const AT_THE_EDGE: f64 = 31.258_786_790_912;
    let range =
        squat_range(&lifter(), None, dorsi(AT_THE_EDGE), frames(143)).expect("the upright stands");
    match range.end {
        RangeEnd::Stopped { reached, bottom } => {
            assert_close(reached, 0.809_839, 1e-4, "s* at the hole");
            assert_eq!(bottom, None, "the bottom balances; the convention stops it");
            let last = range.frames.last().expect("frames up to s*");
            assert!(last.progress <= reached, "no frame past s*");
        }
        RangeEnd::Bottom => panic!("the range cannot reach the bottom through the hole"),
    }
}

/// R14 — `RangeEnd::Bottom` always means every requested frame, the last one
/// slice A's baseline; `Stopped` always means exactly the sampled frames with
/// `s ≤ s*`. Swept across the window's upper edge, where holes are
/// narrowest, and across frame counts: a frame count must never cut a range
/// short without the end saying so.
#[test]
fn r14_the_end_and_the_frames_never_disagree() {
    let body = lifter();
    let edge = 31.258_786_840_912;
    let offsets = [-1e-2, -1e-3, -1e-5, -1e-7, 1e-7, 1e-5, 1e-3];
    for offset in offsets {
        let degrees = edge + offset;
        let baseline_posture = baseline(&body, None, dorsi(degrees)).map(|b| *b.posture());
        for n in [2_u16, 6, 143, 241] {
            let what = format!("{degrees}°, n = {n}");
            let range = squat_range(&body, None, dorsi(degrees), frames(n)).expect("stands");
            match range.end {
                RangeEnd::Bottom => {
                    assert_eq!(range.frames.len(), usize::from(n), "{what}: all frames");
                    let last = range.frames.last().expect("frames");
                    assert_eq!(
                        Ok(last.posture),
                        baseline_posture,
                        "{what}: the last frame is the baseline"
                    );
                }
                RangeEnd::Stopped { reached, .. } => {
                    let expected = range_count_up_to(reached, n);
                    assert_eq!(range.frames.len(), expected, "{what}: frames with s ≤ s*");
                }
            }
        }
    }
}

/// How many of `n` evenly sampled `s` values are `≤ reached`.
fn range_count_up_to(reached: f64, n: u16) -> usize {
    (0..n)
        .filter(|&i| f64::from(i) / f64::from(n - 1) <= reached)
        .count()
}
