//! R-0044 core test suite (SPEC-0044 §8, tests 1–27) — **no video, no
//! database, no model**.
//!
//! Every fixture is a synthetic [`LiftSeries`] built by [`synthetic`], which
//! authors joint positions in the isotropic canvas space of SPEC-0044 §2.2.2.
//! Tests 28–40 of the spec's plan drive the api edge and belong to slice B.
//!
//! These tests were written **before** the algorithms they exercise
//! (constitution §4): the red state is the `todo!()` in each submodule.

// Test prose quotes acceptance-criterion ids and the COCO-17 vocabulary.
#![allow(clippy::doc_markdown)]

mod synthetic;

use synthetic::{
    bench_pose, ease, hold, jitter_series, ramp, series, BenchPose, Lifter, UprightPose,
};

use crate::periodize::lift_key;
use crate::pose::{Keypoint, Landmark, PoseKeypoints};
use crate::technique::{
    analyze, AnalysisOutcome, Calibration, CameraView, Finding, FindingSeverity, Lift,
    LiftAnalysis, LiftSeries, Measurement, Metric, Refusal, Side, Unit, Unmeasurable, ViewClass,
    MAX_FRAMES, MAX_FRAME_BYTES, MAX_TOTAL_BYTES, MIN_FRAMES, NOT_MEASURABLE, SAMPLE_HZ,
};

// ===========================================================================
// Scenario builders
// ===========================================================================

/// Canvas units per torso length — the fixture's own scale, so a test can talk
/// in torso lengths the way SPEC-0044 does.
const TORSO_CANVAS: f64 = 0.26;

/// A standing back squat.
fn squat_standing() -> UprightPose {
    UprightPose::default()
}

/// A squat at the bottom, bottoming out `depth_deg` **above** parallel
/// (negative = hip below knee — the ground truth the analysis must recover).
fn squat_bottom(depth_deg: f64) -> UprightPose {
    UprightPose {
        thigh_pitch_deg: 90.0 - depth_deg,
        shank_pitch_deg: 30.0,
        torso_pitch_deg: 35.0,
        ..UprightPose::default()
    }
}

/// Interpolate an upright pose between two keyframes.
fn lerp_pose(from: UprightPose, to: UprightPose, at: f64) -> UprightPose {
    let mix = |start: f64, end: f64| start + (end - start) * at;
    UprightPose {
        thigh_pitch_deg: mix(from.thigh_pitch_deg, to.thigh_pitch_deg),
        shank_pitch_deg: mix(from.shank_pitch_deg, to.shank_pitch_deg),
        torso_pitch_deg: mix(from.torso_pitch_deg, to.torso_pitch_deg),
        hand_height: match (from.hand_height, to.hand_height) {
            (Some(start), Some(end)) => Some(mix(start, end)),
            (start, _) => start,
        },
        knee_inward: mix(from.knee_inward, to.knee_inward),
        shift_x: mix(from.shift_x, to.shift_x),
        shift_y: mix(from.shift_y, to.shift_y),
    }
}

/// A squat clip: quiet stand, a lateral **walkout**, a settle, then `reps`
/// reps, then a settle. The walkout is the case SPEC-0044 §2.6.1 exists for.
fn squat_series(lifter: Lifter, view: CameraView, depth_deg: f64, reps: usize) -> LiftSeries {
    let top = squat_standing();
    let bottom = squat_bottom(depth_deg);
    let mut poses: Vec<PoseKeypoints> = Vec::new();

    // Stand before approaching the bar.
    poses.extend(hold(8, 0.0).iter().map(|_| lifter.upright(top)));
    // Walk out: a rigid lateral shift, hip height unchanged.
    poses.extend(ramp(10, ease).iter().map(|&t| {
        lifter.upright(UprightPose {
            shift_x: 0.6 * t,
            ..top
        })
    }));
    let settled = UprightPose {
        shift_x: 0.6,
        ..top
    };
    // Settle.
    poses.extend(hold(8, 0.0).iter().map(|_| lifter.upright(settled)));
    let settled_bottom = UprightPose {
        shift_x: 0.6,
        ..bottom
    };
    for _ in 0..reps {
        poses.extend(
            ramp(9, ease)
                .iter()
                .map(|&t| lifter.upright(lerp_pose(settled, settled_bottom, t))),
        );
        poses.extend(
            ramp(9, ease)
                .iter()
                .skip(1)
                .map(|&t| lifter.upright(lerp_pose(settled_bottom, settled, t))),
        );
    }
    poses.extend(hold(6, 0.0).iter().map(|_| lifter.upright(settled)));
    series(Lift::Squat, view, poses)
}

/// A deadlift keyframe: hips low at the setup, bar on the floor.
fn deadlift_setup() -> UprightPose {
    UprightPose {
        thigh_pitch_deg: 70.0,
        shank_pitch_deg: 15.0,
        torso_pitch_deg: 60.0,
        hand_height: Some(0.42),
        ..UprightPose::default()
    }
}

/// A deadlift keyframe: standing at lockout.
fn deadlift_lockout() -> UprightPose {
    UprightPose {
        hand_height: Some(1.55),
        ..UprightPose::default()
    }
}

/// A deadlift keyframe: standing before approaching the bar.
fn deadlift_standing() -> UprightPose {
    deadlift_lockout()
}

/// A deadlift clip: stand, bend to the setup, hold, then `reps` pulls.
///
/// `hip_lead` inserts a "stripper" phase — the hips rising while the bar stays
/// on the floor — of that many frames at the start of each pull.
fn deadlift_series(reps: usize, hip_lead: usize, drop_last: bool) -> LiftSeries {
    let lifter = Lifter::side();
    let stand = deadlift_standing();
    let setup = deadlift_setup();
    let lockout = deadlift_lockout();
    let mut poses: Vec<PoseKeypoints> = Vec::new();

    poses.extend(hold(8, 0.0).iter().map(|_| lifter.upright(stand)));
    poses.extend(
        ramp(10, ease)
            .iter()
            .map(|&t| lifter.upright(lerp_pose(stand, setup, t))),
    );
    poses.extend(hold(7, 0.0).iter().map(|_| lifter.upright(setup)));

    for rep in 0..reps {
        // Hips rise while the bar stays down: only the thigh/torso angles move.
        for i in 0..hip_lead {
            #[allow(clippy::cast_precision_loss)]
            let t = (i + 1) as f64 / (hip_lead + 1) as f64;
            let mut p = lerp_pose(setup, lockout, t * 0.5);
            p.hand_height = setup.hand_height;
            poses.push(lifter.upright(p));
        }
        poses.extend(
            ramp(11, ease)
                .iter()
                .skip(1)
                .map(|&t| lifter.upright(lerp_pose(setup, lockout, t))),
        );
        poses.extend(hold(4, 0.0).iter().map(|_| lifter.upright(lockout)));
        let last = rep + 1 == reps;
        if !(last && drop_last) {
            poses.extend(
                ramp(9, ease)
                    .iter()
                    .skip(1)
                    .map(|&t| lifter.upright(lerp_pose(lockout, setup, t))),
            );
            poses.extend(hold(4, 0.0).iter().map(|_| lifter.upright(setup)));
        }
    }
    series(Lift::Deadlift, CameraView::Side, poses)
}

/// A bench clip: bar on the hooks, a rack-out, a settle, then `reps` reps.
///
/// `bar_caudal` places each rep's touch point, in torso lengths toward the
/// feet; `forearm_deg` sets the forearm angle at the touch.
fn bench_series(reps: usize, touch_points: &[f64], forearm_deg: f64) -> LiftSeries {
    let lock = |caudal: f64| BenchPose {
        bar_height: 0.85,
        bar_caudal: caudal,
        forearm_angle_deg: 0.0,
    };
    let touch = |caudal: f64| BenchPose {
        bar_height: 0.0,
        bar_caudal: caudal,
        forearm_angle_deg: forearm_deg,
    };
    let mut poses: Vec<PoseKeypoints> = Vec::new();

    // On the hooks: behind the shoulder, still.
    poses.extend(hold(6, 0.0).iter().map(|_| bench_pose(lock(-0.45))));
    // Rack-out: a horizontal translation.
    poses.extend(
        ramp(8, ease)
            .iter()
            .map(|&t| bench_pose(lock(-0.45 + 0.45 * t))),
    );
    poses.extend(hold(8, 0.0).iter().map(|_| bench_pose(lock(0.0))));

    for rep in 0..reps {
        let caudal = touch_points.get(rep).copied().unwrap_or(0.0);
        let (a, b) = (lock(0.0), touch(caudal));
        poses.extend(ramp(9, ease).iter().map(|&t| {
            bench_pose(BenchPose {
                bar_height: a.bar_height + (b.bar_height - a.bar_height) * t,
                bar_caudal: a.bar_caudal + (b.bar_caudal - a.bar_caudal) * t,
                forearm_angle_deg: a.forearm_angle_deg
                    + (b.forearm_angle_deg - a.forearm_angle_deg) * t,
            })
        }));
        poses.extend(ramp(9, ease).iter().skip(1).map(|&t| {
            bench_pose(BenchPose {
                bar_height: b.bar_height + (a.bar_height - b.bar_height) * t,
                bar_caudal: b.bar_caudal + (a.bar_caudal - b.bar_caudal) * t,
                forearm_angle_deg: b.forearm_angle_deg
                    + (a.forearm_angle_deg - b.forearm_angle_deg) * t,
            })
        }));
    }
    poses.extend(hold(6, 0.0).iter().map(|_| bench_pose(lock(0.0))));
    series(Lift::Bench, CameraView::Side, poses)
}

// ===========================================================================
// Assertion helpers
// ===========================================================================

#[track_caller]
fn analyzed(series: &LiftSeries) -> LiftAnalysis {
    match analyze(series, None) {
        AnalysisOutcome::Analyzed(a) => a,
        AnalysisOutcome::Refused(r) => panic!("expected an analysis, got {r:?}"),
    }
}

#[track_caller]
fn refused(series: &LiftSeries) -> Refusal {
    match analyze(series, None) {
        AnalysisOutcome::Refused(r) => r,
        AnalysisOutcome::Analyzed(a) => panic!("expected a refusal, got {a:?}"),
    }
}

#[track_caller]
fn finding(analysis: &LiftAnalysis, metric: Metric, rep: Option<u32>) -> Finding {
    analysis
        .measurements
        .iter()
        .find_map(|m| match m {
            Measurement::Measured(f) if f.metric == metric && f.rep == rep => Some(*f),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no {metric:?} finding for rep {rep:?} in {analysis:#?}"))
}

#[track_caller]
fn unavailable(analysis: &LiftAnalysis, metric: Metric) -> Unmeasurable {
    analysis
        .measurements
        .iter()
        .find_map(|m| match m {
            Measurement::Unavailable {
                metric: got,
                reason,
                ..
            } if *got == metric => Some(*reason),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no unavailable {metric:?} in {analysis:#?}"))
}

/// Rewrite one landmark on every frame.
fn map_landmark(
    series: &LiftSeries,
    landmark: Landmark,
    f: impl Fn(Keypoint) -> Keypoint,
) -> LiftSeries {
    let mut out = series.clone();
    for frame in &mut out.frames {
        let mut points = *frame.pose.points();
        points[landmark.index()] = f(points[landmark.index()]);
        frame.pose = PoseKeypoints::new(points);
    }
    out
}

// ===========================================================================
// 1–6 · Segmentation (SPEC-0044 §2.6) — the cases the rejected draft got wrong
// ===========================================================================

/// Test 1 — **the walkout is excluded**, the rep count is right, and the
/// standing reference is taken from the quiet window rather than frame 0.
///
/// Architect review finding 1: the *first* quiet window is the lifter standing
/// before approaching the bar. Using it puts the walkout inside the
/// segmentation window, which trips `CameraMoved` on every squat.
#[test]
fn walkout_is_excluded_and_reps_counted() {
    let s = squat_series(Lifter::side(), CameraView::Side, -5.0, 2);
    let a = analyzed(&s);
    assert_eq!(a.rep_count, 2, "the walkout must not read as a rep");
    // The reference window ends after the walkout, so rep 1 starts well past
    // frame 0 — 8 stand + 10 walkout + 8 settle.
    assert!(
        a.reps[0].start >= 18,
        "rep 1 started at {} — inside the walkout",
        a.reps[0].start
    );
}

/// Test 2 — a 2 s bottom hold with keypoint jitter is **one** rep, not three.
/// Bare reversal detection counts every micro-reversal in the hold.
#[test]
fn paused_rep_is_one_rep() {
    let lifter = Lifter::side();
    let top = squat_standing();
    let bottom = squat_bottom(-5.0);
    let mut poses: Vec<PoseKeypoints> = Vec::new();
    poses.extend(hold(10, 0.0).iter().map(|_| lifter.upright(top)));
    poses.extend(
        ramp(9, ease)
            .iter()
            .map(|&t| lifter.upright(lerp_pose(top, bottom, t))),
    );
    poses.extend(hold(20, 0.0).iter().map(|_| lifter.upright(bottom)));
    poses.extend(
        ramp(9, ease)
            .iter()
            .skip(1)
            .map(|&t| lifter.upright(lerp_pose(bottom, top, t))),
    );
    poses.extend(hold(8, 0.0).iter().map(|_| lifter.upright(top)));
    let s = jitter_series(
        &series(Lift::Squat, CameraView::Side, poses),
        0.04 * TORSO_CANVAS,
        7,
    );
    assert_eq!(analyzed(&s).rep_count, 1);
}

/// Test 3 — a grinder: a mid-ascent plateau with a small reversal is **one**
/// rep.
#[test]
fn mid_ascent_stall_is_one_rep() {
    let lifter = Lifter::side();
    let top = squat_standing();
    let bottom = squat_bottom(-5.0);
    let stall = lerp_pose(bottom, top, 0.45);
    let dip = lerp_pose(bottom, top, 0.38);
    let mut poses: Vec<PoseKeypoints> = Vec::new();
    poses.extend(hold(10, 0.0).iter().map(|_| lifter.upright(top)));
    poses.extend(
        ramp(9, ease)
            .iter()
            .map(|&t| lifter.upright(lerp_pose(top, bottom, t))),
    );
    poses.extend(
        ramp(5, ease)
            .iter()
            .skip(1)
            .map(|&t| lifter.upright(lerp_pose(bottom, stall, t))),
    );
    // The sticking point: a small reversal, then through.
    poses.extend(hold(3, 0.0).iter().map(|_| lifter.upright(dip)));
    poses.extend(
        ramp(9, ease)
            .iter()
            .map(|&t| lifter.upright(lerp_pose(dip, top, t))),
    );
    poses.extend(hold(8, 0.0).iter().map(|_| lifter.upright(top)));
    let s = series(Lift::Squat, CameraView::Side, poses);
    assert_eq!(analyzed(&s).rep_count, 1);
}

/// Test 4 — touch-and-go deadlift: reps 2..N reference **their own start**, not
/// rep 1's setup, and every rep is measured.
#[test]
fn touch_and_go_deadlift_reps_reference_their_own_start() {
    let s = deadlift_series(3, 0, false);
    let a = analyzed(&s);
    assert_eq!(a.rep_count, 3);
    // Contiguous: each rep starts where the previous one ended.
    for pair in a.reps.windows(2) {
        assert_eq!(pair[0].end, pair[1].start);
    }
    for rep in 1..=3 {
        let f = finding(&a, Metric::DeadliftBarDriftFromAnkle, Some(rep));
        assert!(f.value.is_finite());
    }
}

/// Test 5 — a weight shift while re-racking does not clear
/// `MIN_REP_EXCURSION` and is not a rep.
#[test]
fn sub_excursion_wobble_is_not_a_rep() {
    let lifter = Lifter::side();
    let top = squat_standing();
    let dip = lerp_pose(top, squat_bottom(-5.0), 0.10);
    let mut poses: Vec<PoseKeypoints> = Vec::new();
    poses.extend(hold(10, 0.0).iter().map(|_| lifter.upright(top)));
    poses.extend(
        ramp(6, ease)
            .iter()
            .map(|&t| lifter.upright(lerp_pose(top, dip, t))),
    );
    poses.extend(
        ramp(6, ease)
            .iter()
            .skip(1)
            .map(|&t| lifter.upright(lerp_pose(dip, top, t))),
    );
    poses.extend(hold(10, 0.0).iter().map(|_| lifter.upright(top)));
    let s = series(Lift::Squat, CameraView::Side, poses);
    assert_eq!(refused(&s), Refusal::NoRepsDetected);
}

/// Test 6 — **centred, not trailing**. The detected extremum index must equal
/// the ground-truth bottom frame; a trailing average would offset it by the
/// smoothing half-width, and depth would then be read at the wrong instant.
#[test]
fn extremum_index_is_not_phase_shifted() {
    let lifter = Lifter::side();
    let top = squat_standing();
    let bottom = squat_bottom(-5.0);
    let mut poses: Vec<PoseKeypoints> = Vec::new();
    poses.extend(hold(10, 0.0).iter().map(|_| lifter.upright(top)));
    poses.extend(
        ramp(11, ease)
            .iter()
            .map(|&t| lifter.upright(lerp_pose(top, bottom, t))),
    );
    let ground_truth = poses.len() - 1;
    poses.extend(
        ramp(11, ease)
            .iter()
            .skip(1)
            .map(|&t| lifter.upright(lerp_pose(bottom, top, t))),
    );
    poses.extend(hold(10, 0.0).iter().map(|_| lifter.upright(top)));
    let s = series(Lift::Squat, CameraView::Side, poses);
    let a = analyzed(&s);
    assert_eq!(a.reps.len(), 1);
    assert_eq!(a.reps[0].extremum, ground_truth);
}

// ===========================================================================
// 7–10 · View and stability (SPEC-0044 §2.5)
// ===========================================================================

/// Test 7 — the yaw sweep. `shoulder_span / torso_length` falls monotonically
/// from front-on to side-on; the classification is Front at small yaw, Side at
/// large yaw, and **everything in the deliberately wide band between is
/// refused**.
#[test]
fn yaw_sweep_is_monotone_and_the_band_refuses() {
    let mut previous = f64::INFINITY;
    for step in 0..=18 {
        #[allow(clippy::cast_precision_loss)]
        let yaw = f64::from(step) * 5.0;
        let lifter = Lifter {
            yaw_deg: yaw,
            ..Lifter::side()
        };
        let cues = crate::technique::view::cues(&lifter.upright(squat_standing()))
            .expect("a standing pose has every view landmark");
        assert!(
            cues.shoulder_ratio <= previous + 1e-9,
            "view_ratio rose at yaw {yaw}: {} > {previous}",
            cues.shoulder_ratio
        );
        previous = cues.shoulder_ratio;

        let class = crate::technique::view::classify(&cues);
        if yaw <= 25.0 {
            assert_eq!(class, ViewClass::Front, "yaw {yaw}");
        } else if yaw >= 70.0 {
            assert_eq!(class, ViewClass::Side, "yaw {yaw}");
        }
        if (35.0..=60.0).contains(&yaw) {
            assert_eq!(class, ViewClass::Indeterminate, "yaw {yaw} must refuse");
            let s = squat_series(lifter, CameraView::Side, -5.0, 1);
            assert!(matches!(refused(&s), Refusal::WrongView { .. }));
        }
    }
}

/// Test 8 — **the rejected rule's failure, pinned.** `shoulder_span / hip_span`
/// is constant across the same sweep: both spans are frontal-plane segments and
/// the cosine cancels exactly. Nobody may reintroduce it.
#[test]
fn shoulder_over_hip_span_is_constant_across_yaw() {
    let mut ratios = Vec::new();
    for step in 0..=16 {
        let lifter = Lifter {
            yaw_deg: f64::from(step) * 5.0,
            ..Lifter::side()
        };
        let pose = lifter.upright(squat_standing());
        let span = |a: Landmark, b: Landmark| pose.get(a).distance_to(pose.get(b));
        let shoulders = span(Landmark::LeftShoulder, Landmark::RightShoulder);
        let hips = span(Landmark::LeftHip, Landmark::RightHip);
        if hips > 1e-6 {
            ratios.push(shoulders / hips);
        }
    }
    // The keypoints are `f32`, so "constant" bottoms out at that precision.
    // Over the same sweep the true cosine falls from 1.0 to 0.34 — the ratio
    // moves by less than a part in a thousand, which is the point.
    let first = ratios[0];
    for r in &ratios {
        assert!(
            (r - first).abs() < 1e-3,
            "shoulder/hip moved with yaw: {r} vs {first} — it cannot measure yaw"
        );
    }
}

/// Test 9 — a clip that reads Side for its first half and Front for its second
/// is a panned camera or a rotating lifter. There is no single view the
/// geometry may assume.
#[test]
fn view_that_changes_mid_clip_is_refused() {
    let side = squat_series(Lifter::side(), CameraView::Side, -5.0, 1);
    let front = squat_series(Lifter::front(), CameraView::Side, -5.0, 1);
    let mut mixed = side.clone();
    let half = mixed.frames.len() / 2;
    for i in half..mixed.frames.len() {
        mixed.frames[i].pose = front.frames[i].pose;
    }
    assert!(
        matches!(refused(&mixed), Refusal::UnstableView { .. }),
        "got {:?}",
        refused(&mixed)
    );
}

/// Test 10 — a camera pan across the segmentation window is `CameraMoved`; the
/// **same** pan confined to the pre-quiet walkout is not.
///
/// The pan translates the whole lifter, which is what a pan does — and which
/// leaves every segment length, and therefore every normalizer, untouched. The
/// check is on the landmark's **standard deviation**, not its range, so one bad
/// frame cannot condemn a clip and a slow pan has to be large: a linear sweep
/// of `PAN` across the clip is a σ of roughly a fifth of a shank over the
/// segmentation window, against an allowance of 0.15.
#[test]
fn camera_drift_refuses_only_inside_the_segmentation_window() {
    /// Sweep applied across the clip, in shank lengths.
    const PAN: f64 = 1.2;
    let clean = squat_series(Lifter::side(), CameraView::Side, -5.0, 2);
    let shank_canvas = 0.85 * TORSO_CANVAS;

    let pan = |series: &LiftSeries, shift: &dyn Fn(f64) -> f64| {
        let mut out = series.clone();
        let n = out.frames.len();
        for (i, frame) in out.frames.iter_mut().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let dx = shift(i as f64 / (n - 1) as f64);
            let mut points = *frame.pose.points();
            for point in &mut points {
                #[allow(clippy::cast_possible_truncation)]
                {
                    point.x += (dx * PAN * shank_canvas) as f32;
                }
            }
            frame.pose = PoseKeypoints::new(points);
        }
        out
    };

    let drifting = pan(&clean, &|t| t);
    assert!(
        matches!(refused(&drifting), Refusal::CameraMoved { .. }),
        "got {:?}",
        refused(&drifting)
    );

    // The identical sweep, finished before the quiet window the segmentation
    // starts from, must not refuse.
    let quiet_starts_at = 18.0;
    #[allow(clippy::cast_precision_loss)]
    let last = (clean.frames.len() - 1) as f64;
    let early = pan(&clean, &|t| (1.0 - t * last / quiet_starts_at).max(0.0));
    assert_eq!(analyzed(&early).rep_count, 2);
}

// ===========================================================================
// 11 · Timestamps (SPEC-0044 §2.2.1)
// ===========================================================================

/// Test 11 — a series sampled at half the rate it claims, and a series with one
/// long gap, are both `IrregularSampling`. Analysing a 5 Hz series as if it were
/// 10 Hz would misread every tempo and every turning point.
#[test]
fn irregular_sampling_is_refused() {
    let mut slow = squat_series(Lifter::side(), CameraView::Side, -5.0, 2);
    for (i, frame) in slow.frames.iter_mut().enumerate() {
        frame.t_ms = u32::try_from(i).unwrap() * 200;
    }
    assert!(matches!(refused(&slow), Refusal::IrregularSampling { .. }));

    let mut gappy = squat_series(Lifter::side(), CameraView::Side, -5.0, 2);
    for (i, frame) in gappy.frames.iter_mut().enumerate() {
        let extra = if i >= 20 { 700 } else { 0 };
        frame.t_ms = u32::try_from(i).unwrap() * 100 + extra;
    }
    assert!(matches!(
        refused(&gappy),
        Refusal::IrregularSampling {
            max_gap_ms: 800,
            ..
        }
    ));
}

// ===========================================================================
// 12–20 · Measurements (SPEC-0044 §2.7.3, §2.8)
// ===========================================================================

/// Test 12 — depth is recovered, and the **owner's threshold decision** is
/// exercised: all three severities are reachable by the straddling-interval
/// rule, and the uncertainty is non-zero and includes the hip-crease bound.
#[test]
fn squat_depth_value_and_all_three_severities() {
    for (truth, expected) in [
        (-18.0, FindingSeverity::Ok),
        (-2.0, FindingSeverity::Borderline),
        (20.0, FindingSeverity::Flagged),
    ] {
        let s = squat_series(Lifter::side(), CameraView::Side, truth, 1);
        let a = analyzed(&s);
        let f = finding(&a, Metric::SquatDepth, Some(1));
        assert_eq!(f.unit, Unit::Degrees);
        assert!(
            (f.value - truth).abs() < 1.5,
            "depth {} should be ≈ {truth}",
            f.value
        );
        assert!(f.uncertainty > 0.0, "uncertainty must be reported");
        assert!(
            f.uncertainty > 4.0,
            "uncertainty {} does not carry the hip-crease bound",
            f.uncertainty
        );
        assert_eq!(f.severity, Some(expected), "truth {truth}");
        assert!(f.frame.is_some(), "depth is read at one instant");
    }
}

/// Every metric other than depth ships **without** a severity: no citable
/// threshold exists, and the UI renders such a value neutrally.
#[test]
fn only_depth_carries_a_severity() {
    let a = analyzed(&squat_series(Lifter::side(), CameraView::Side, -5.0, 2));
    for m in &a.measurements {
        if let Measurement::Measured(f) = m {
            if f.metric == Metric::SquatDepth {
                assert!(f.severity.is_some());
            } else {
                assert_eq!(f.severity, None, "{:?} must not carry a verdict", f.metric);
            }
        }
    }
}

/// Test 13 — knee travel is a front-view measurement, reported **per side**;
/// the same footage declared as a side view is refused rather than measured.
#[test]
fn knee_travel_is_front_only_and_per_side() {
    let lifter = Lifter::front();
    let top = squat_standing();
    let bottom = UprightPose {
        knee_inward: 0.18,
        ..squat_bottom(-5.0)
    };
    let mut poses: Vec<PoseKeypoints> = Vec::new();
    poses.extend(hold(10, 0.0).iter().map(|_| lifter.upright(top)));
    poses.extend(
        ramp(9, ease)
            .iter()
            .map(|&t| lifter.upright(lerp_pose(top, bottom, t))),
    );
    poses.extend(
        ramp(9, ease)
            .iter()
            .skip(1)
            .map(|&t| lifter.upright(lerp_pose(bottom, top, t))),
    );
    poses.extend(hold(8, 0.0).iter().map(|_| lifter.upright(top)));

    let front = series(Lift::Squat, CameraView::Front, poses.clone());
    let a = analyzed(&front);
    for side in [Side::Left, Side::Right] {
        let f = a
            .measurements
            .iter()
            .find_map(|m| match m {
                Measurement::Measured(f)
                    if f.metric == Metric::KneeTravelInward && f.side == Some(side) =>
                {
                    Some(*f)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("no knee-travel finding for {side:?}"));
        assert_eq!(f.unit, Unit::NormalizedLength);
        assert!(
            f.value > 0.10,
            "inward travel {} should be positive",
            f.value
        );
    }
    // Depth cannot be seen from the front and must say so, not guess.
    assert_eq!(
        unavailable(&a, Metric::SquatDepth),
        Unmeasurable::NotThisView
    );

    let mislabelled = series(Lift::Squat, CameraView::Side, poses);
    assert!(matches!(
        refused(&mislabelled),
        Refusal::WrongView {
            looks_like: ViewClass::Front,
            ..
        }
    ));
}

/// Test 14 — bar-path deviation is normalized by forearm length, and a
/// **single-rep** set yields `Unavailable { SingleRep }` for touch-point
/// consistency — never a fabricated 0.0, which reads as perfect.
#[test]
fn bench_bar_path_and_single_rep_consistency() {
    let multi = bench_series(3, &[0.0, 0.22, -0.18], 0.0);
    let a = analyzed(&multi);
    assert_eq!(a.rep_count, 3);
    let drift = finding(&a, Metric::BenchBarPathDeviation, Some(2));
    assert_eq!(drift.unit, Unit::NormalizedLength);
    assert!(drift.value > 0.0);
    let consistency = finding(&a, Metric::BenchTouchPointConsistency, None);
    assert!(consistency.value > 0.0, "three different touch points vary");

    let single = bench_series(1, &[0.0], 0.0);
    let a1 = analyzed(&single);
    assert_eq!(
        unavailable(&a1, Metric::BenchTouchPointConsistency),
        Unmeasurable::SingleRep
    );
}

/// Test 15 — the forearm angle at the touch frame, signed both ways. Positive
/// is the elbow caudal (toward the feet) of the wrist.
#[test]
fn bench_forearm_angle_at_touch_is_signed() {
    for truth in [-22.0, 22.0] {
        let s = bench_series(1, &[0.0], truth);
        let a = analyzed(&s);
        let f = finding(&a, Metric::BenchForearmAngleAtTouch, Some(1));
        assert_eq!(f.unit, Unit::Degrees);
        assert!(
            (f.value - truth).abs() < 3.0,
            "forearm angle {} should be ≈ {truth}",
            f.value
        );
        assert_eq!(f.frame, Some(u32::try_from(a.reps[0].extremum).unwrap()));
    }
}

/// Test 16 — hip rise before the bar is a **ratio**, not a correlation: ≫ 1 when
/// the hips shoot up, ≈ 1 on a clean pull, and a typed `BarDidNotBreak` when the
/// bar never leaves the floor.
#[test]
fn deadlift_hip_rise_is_a_ratio_with_a_typed_guard() {
    let clean = analyzed(&deadlift_series(1, 0, false));
    let clean_ratio = finding(&clean, Metric::DeadliftHipRiseBeforeBar, Some(1));
    assert_eq!(clean_ratio.unit, Unit::Ratio);
    assert!(
        clean_ratio.value < 4.0,
        "a clean pull should not read as a hip shoot: {}",
        clean_ratio.value
    );

    let stripper = analyzed(&deadlift_series(1, 6, false));
    let stripper_ratio = finding(&stripper, Metric::DeadliftHipRiseBeforeBar, Some(1));
    assert!(
        stripper_ratio.value > clean_ratio.value * 2.0,
        "hips leading the bar must read far higher: {} vs {}",
        stripper_ratio.value,
        clean_ratio.value
    );

    // The bar never leaves the floor: the ratio has no denominator, and that is
    // a typed outcome rather than a division by something near zero.
    let base = deadlift_series(1, 6, false);
    let floor_y = base.frames[0].pose.get(Landmark::LeftWrist).y;
    let stuck = [Landmark::LeftWrist, Landmark::RightWrist]
        .into_iter()
        .fold(base, |s, l| {
            map_landmark(&s, l, |k| Keypoint { y: floor_y, ..k })
        });
    let a = analyzed(&stuck);
    assert_eq!(
        unavailable(&a, Metric::DeadliftHipRiseBeforeBar),
        Unmeasurable::BarDidNotBreak
    );
}

/// The degeneracy pin for test 16: a correlation against a near-constant bar
/// signal has a vanishing denominator, so it is undefined *exactly* in the case
/// it would exist to detect. This is why the measurement is a ratio.
#[test]
fn correlation_against_a_static_bar_is_degenerate() {
    let bar = [0.400_1, 0.399_9, 0.400_0, 0.400_1, 0.399_9];
    let mean = bar.iter().sum::<f64>() / 5.0;
    let variance = bar.iter().map(|b| (b - mean).powi(2)).sum::<f64>() / 5.0;
    assert!(
        variance < 1e-7,
        "a bar on the floor has ~zero variance, so Pearson's denominator collapses"
    );
}

/// Test 17 — torso angle change is reported against the **frame of maximum
/// change during the pull**, and that frame is reported and is not the start
/// frame. "Setup versus bottom" was identically zero: for a deadlift the setup
/// *is* the bottom.
#[test]
fn deadlift_torso_angle_change_reports_a_frame_that_is_not_the_start() {
    let s = deadlift_series(1, 4, false);
    let a = analyzed(&s);
    let f = finding(&a, Metric::DeadliftTorsoAngleChange, Some(1));
    assert_eq!(f.unit, Unit::Degrees);
    assert!(f.value > 5.0, "the torso does change through a pull");
    let frame = f.frame.expect("the frame of maximum change is reported");
    assert_ne!(usize::try_from(frame).unwrap(), a.reps[0].start);
    assert!(usize::try_from(frame).unwrap() <= a.reps[0].extremum);
}

/// Test 18 — bar drift is measured from the **ankle** and normalized by shank
/// length. COCO-17 has no foot landmark, so there is nothing else it could use —
/// and the per-lift modules are scanned to make sure nobody invents one.
#[test]
fn deadlift_bar_drift_is_from_the_ankle_over_shank() {
    let s = deadlift_series(1, 0, false);
    let a = analyzed(&s);
    let f = finding(&a, Metric::DeadliftBarDriftFromAnkle, Some(1));
    assert_eq!(f.unit, Unit::NormalizedLength);
    assert!(f.value >= 0.0);

    for source in [
        include_str!("../deadlift.rs"),
        include_str!("../squat.rs"),
        include_str!("../bench.rs"),
        include_str!("../geometry.rs"),
    ] {
        for line in source.lines() {
            let code = line.split("//").next().unwrap_or_default();
            assert!(
                !code.to_lowercase().contains("foot"),
                "a foot landmark or foot normalizer does not exist: {line}"
            );
        }
    }
}

/// Test 19 — **the occluded far side is never averaged in.** A midpoint of a
/// near and an occluded far landmark is a fabricated centre-line whose error is
/// largest at the bottom of a squat — exactly where depth is read.
#[test]
fn far_side_landmarks_do_not_affect_the_measurement() {
    let base = squat_series(Lifter::side(), CameraView::Side, -5.0, 1);
    let far_hip = Landmark::RightHip;
    let a = analyzed(&map_landmark(&base, far_hip, |k| Keypoint {
        score: 0.05,
        ..k
    }));
    let b = analyzed(&map_landmark(&base, far_hip, |k| Keypoint {
        #[allow(clippy::cast_possible_truncation)]
        y: k.y + (0.20 * TORSO_CANVAS) as f32,
        score: 0.05,
        ..k
    }));
    // Exact equality is the assertion: the far hip must not enter the
    // computation at all, so the two values are the same float, not close ones.
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(
            finding(&a, Metric::SquatDepth, Some(1)).value,
            finding(&b, Metric::SquatDepth, Some(1)).value,
            "moving an occluded far hip changed the depth"
        );
    }
}

/// Test 20 — per-rep tempo in seconds, and a deadlift whose last rep was
/// dropped reports `eccentric_s: None` rather than a fabricated 0.0.
#[test]
fn tempo_is_reported_and_a_dropped_deadlift_has_no_eccentric() {
    let a = analyzed(&squat_series(Lifter::side(), CameraView::Side, -5.0, 2));
    assert_eq!(a.tempo.len(), 2);
    for t in &a.tempo {
        assert!(t.eccentric_s.unwrap() > 0.0);
        assert!(t.concentric_s > 0.0);
    }

    let d = analyzed(&deadlift_series(2, 0, true));
    assert_eq!(d.tempo.len(), 2);
    assert!(d.tempo[0].eccentric_s.is_some());
    assert_eq!(
        d.tempo[1].eccentric_s, None,
        "the bar was dropped — there is no eccentric to report"
    );
}

// ===========================================================================
// 21–27 · Properties
// ===========================================================================

/// Test 21 — determinism. The same series twice is the same outcome.
#[test]
fn analysis_is_deterministic() {
    let s = squat_series(Lifter::side(), CameraView::Side, -5.0, 3);
    assert_eq!(analyze(&s, None), analyze(&s, None));
}

/// Test 22 — noise stability. Jitter at MoveNet-scale (a fraction of the
/// lifter's own torso, never pixels) must not move a value by more than its
/// reported uncertainty, and must not change the rep count.
#[test]
fn jitter_stays_inside_the_reported_uncertainty() {
    let clean = squat_series(Lifter::side(), CameraView::Side, -6.0, 2);
    let baseline = analyzed(&clean);
    for seed in [1_u32, 12_345, 999_983] {
        let noisy = analyzed(&jitter_series(&clean, 0.03 * TORSO_CANVAS, seed));
        assert_eq!(noisy.rep_count, baseline.rep_count, "seed {seed}");
        for rep in 1..=baseline.rep_count {
            let a = finding(&baseline, Metric::SquatDepth, Some(rep));
            let b = finding(&noisy, Metric::SquatDepth, Some(rep));
            assert!(
                (a.value - b.value).abs() <= b.uncertainty,
                "seed {seed} rep {rep}: depth moved {:.2}° against an uncertainty of {:.2}°",
                (a.value - b.value).abs(),
                b.uncertainty
            );
        }
    }
}

/// Test 23 — the §2.3.1 bounds invariant, restated at runtime beside the
/// `const` assertions: the shortest legal request at the largest legal frame
/// must fit, and the total cap must be reachable.
#[test]
// The constants are compile-time knowable, which is exactly why this restates
// the `const` assertion: a reader who edits one of the four sees it here too.
#[allow(clippy::assertions_on_constants)]
fn bounds_invariant_holds() {
    assert!(MAX_FRAME_BYTES * MIN_FRAMES <= MAX_TOTAL_BYTES);
    assert!(MAX_TOTAL_BYTES <= MAX_FRAME_BYTES * MAX_FRAMES);
    assert!(MIN_FRAMES < MAX_FRAMES);
    assert!(SAMPLE_HZ > 0);
}

/// Test 24 — the exact user-facing copy, and a scan that makes AC14
/// mechanically reviewable: no metric string may name an injury, a diagnosis, or
/// a clinical term.
#[test]
fn metric_copy_is_exact_and_carries_no_medical_claim() {
    /// Vocabulary that would turn a movement description into a diagnosis.
    const FORBIDDEN: [&str; 6] = ["injur", "pain", "damage", "danger", "risk", "valgus"];

    assert_eq!(Metric::SquatDepth.label(), "Depth at the bottom");
    assert_eq!(
        Metric::SquatDepth.cue(),
        "Aim to bring the hip crease level with the top of the knee."
    );
    assert_eq!(Metric::KneeTravelInward.label(), "Knee travel (inward)");
    assert_eq!(
        Metric::KneeTravelInward.cue(),
        "Think about spreading the floor with your feet so the knees track over the toes."
    );
    assert_eq!(Metric::SquatTorsoAngleChange.label(), "Torso angle change");
    assert_eq!(
        Metric::SquatTorsoAngleChange.cue(),
        "Try to hold the torso angle you start the rep with, all the way down and up."
    );
    assert_eq!(
        Metric::BenchBarPathDeviation.label(),
        "Bar path (horizontal travel)"
    );
    assert_eq!(
        Metric::BenchBarPathDeviation.cue(),
        "Aim for the bar to travel the same line down and up."
    );
    assert_eq!(
        Metric::BenchForearmAngleAtTouch.label(),
        "Forearm angle at the chest"
    );
    assert_eq!(
        Metric::BenchForearmAngleAtTouch.cue(),
        "At the chest, aim to have the forearm vertical under the bar."
    );
    assert_eq!(
        Metric::BenchTouchPointConsistency.label(),
        "Touch point consistency"
    );
    assert_eq!(
        Metric::BenchTouchPointConsistency.cue(),
        "Aim to touch the same spot on the chest each rep."
    );
    assert_eq!(
        Metric::DeadliftHipRiseBeforeBar.label(),
        "Hip rise before the bar moves"
    );
    assert_eq!(
        Metric::DeadliftHipRiseBeforeBar.cue(),
        "Try to start the hips and the bar together rather than letting the hips rise first."
    );
    assert_eq!(
        Metric::DeadliftTorsoAngleChange.label(),
        "Torso angle change during the pull"
    );
    assert_eq!(
        Metric::DeadliftTorsoAngleChange.cue(),
        "Aim to hold the torso angle you set up with until the bar passes the knee."
    );
    assert_eq!(
        Metric::DeadliftBarDriftFromAnkle.label(),
        "Bar distance from the ankle"
    );
    assert_eq!(
        Metric::DeadliftBarDriftFromAnkle.cue(),
        "Aim to keep the bar close to the leg through the pull."
    );

    for metric in Metric::ALL {
        for text in [metric.label(), metric.cue()] {
            let lower = text.to_lowercase();
            for word in FORBIDDEN {
                assert!(!lower.contains(word), "{metric:?} copy contains {word:?}");
            }
        }
    }
}

/// Test 25 — the `Lift` ↔ free-text bridge. Every key is already in `lift_key`
/// form, so the two cannot drift; `sumo deadlift` is deliberately unmatched.
#[test]
fn lift_keys_are_canonical_and_round_trip() {
    for lift in [Lift::Squat, Lift::Bench, Lift::Deadlift] {
        let key = lift.canonical_key();
        assert_eq!(
            lift_key(key),
            key,
            "{lift:?} canonical key is not normalized"
        );
        assert!(lift.aliases().contains(&key));
        for alias in lift.aliases() {
            assert_eq!(lift_key(alias), *alias, "alias {alias:?} is not normalized");
            assert_eq!(Lift::from_exercise_name(alias), Some(lift));
        }
        assert_eq!(Lift::from_exercise_name(&key.to_uppercase()), Some(lift));
        assert_eq!(Lift::from_exercise_name(&format!("  {key} ")), Some(lift));
    }
    assert_eq!(Lift::from_exercise_name("sumo deadlift"), None);
    assert_eq!(Lift::from_exercise_name("leg press"), None);
    assert_eq!(Lift::from_exercise_name(""), None);
}

/// Test 26 — every `Refusal` variant is reachable, deliberately.
#[test]
fn every_refusal_variant_is_reachable() {
    // TooFewFrames — unreachable over HTTP, but `analyze` is total.
    let mut short = squat_series(Lifter::side(), CameraView::Side, -5.0, 1);
    short.frames.truncate(4);
    assert!(matches!(
        refused(&short),
        Refusal::TooFewFrames { have: 4, .. }
    ));

    // IrregularSampling — covered by its own test.
    let mut slow = squat_series(Lifter::side(), CameraView::Side, -5.0, 1);
    for (i, frame) in slow.frames.iter_mut().enumerate() {
        frame.t_ms = u32::try_from(i).unwrap() * 250;
    }
    assert!(matches!(refused(&slow), Refusal::IrregularSampling { .. }));

    // OutOfFrame — a load-bearing landmark below the floor in most frames.
    let base = squat_series(Lifter::side(), CameraView::Side, -5.0, 1);
    let cropped = {
        let mut s = base.clone();
        let n = s.frames.len();
        for frame in s.frames.iter_mut().take(n / 2) {
            let mut points = *frame.pose.points();
            points[Landmark::LeftAnkle.index()].score = 0.0;
            frame.pose = PoseKeypoints::new(points);
        }
        s
    };
    assert!(matches!(refused(&cropped), Refusal::OutOfFrame { .. }));

    // LowConfidence — above the per-point floor, below the mean floor.
    let murky = {
        let mut s = base.clone();
        for frame in &mut s.frames {
            let mut points = *frame.pose.points();
            for p in &mut points {
                p.score = 0.25;
            }
            frame.pose = PoseKeypoints::new(points);
        }
        s
    };
    assert!(matches!(refused(&murky), Refusal::LowConfidence { .. }));

    // WrongView / UnstableView / CameraMoved — covered by tests 7, 9, 10.
    let front_footage = squat_series(Lifter::front(), CameraView::Side, -5.0, 1);
    assert!(matches!(refused(&front_footage), Refusal::WrongView { .. }));

    // NoStableStart — never still.
    let restless = {
        let lifter = Lifter::side();
        let top = squat_standing();
        let bottom = squat_bottom(-5.0);
        let mut poses = Vec::new();
        for i in 0..40 {
            let t = ease((f64::from(i) / 6.0).fract());
            poses.push(lifter.upright(lerp_pose(top, bottom, t)));
        }
        series(Lift::Squat, CameraView::Side, poses)
    };
    assert_eq!(refused(&restless), Refusal::NoStableStart);

    // NoRepsDetected — still throughout.
    let motionless = {
        let lifter = Lifter::side();
        let poses = (0..40).map(|_| lifter.upright(squat_standing())).collect();
        series(Lift::Squat, CameraView::Side, poses)
    };
    assert_eq!(refused(&motionless), Refusal::NoRepsDetected);
}

/// Test 27 — `analyze` is **total**: no series makes it panic.
#[test]
fn analyze_is_total() {
    let base = squat_series(Lifter::side(), CameraView::Side, -5.0, 1);

    let mut empty = base.clone();
    empty.frames.clear();
    assert!(matches!(
        analyze(&empty, None),
        AnalysisOutcome::Refused(Refusal::TooFewFrames { have: 0, .. })
    ));

    let mut one = base.clone();
    one.frames.truncate(1);
    assert!(matches!(
        analyze(&one, None),
        AnalysisOutcome::Refused(Refusal::TooFewFrames { have: 1, .. })
    ));

    let blind = {
        let mut s = base.clone();
        for frame in &mut s.frames {
            let mut points = *frame.pose.points();
            for p in &mut points {
                p.score = 0.0;
            }
            frame.pose = PoseKeypoints::new(points);
        }
        s
    };
    assert!(matches!(analyze(&blind, None), AnalysisOutcome::Refused(_)));

    // Degenerate geometry: every joint collapsed onto one point.
    let collapsed = {
        let mut s = base.clone();
        for frame in &mut s.frames {
            frame.pose = PoseKeypoints::new(
                [Keypoint {
                    x: 0.5,
                    y: 0.5,
                    score: 0.9,
                }; 17],
            );
        }
        s
    };
    assert!(matches!(
        analyze(&collapsed, None),
        AnalysisOutcome::Refused(_)
    ));

    // A zero sample rate must not divide by zero.
    let mut unrated = base.clone();
    unrated.sample_hz = 0;
    assert!(matches!(
        analyze(&unrated, None),
        AnalysisOutcome::Refused(_)
    ));
}

// ===========================================================================
// AC7 · The calibration seam (owner decision 2, architect review finding 4b)
// ===========================================================================

/// Calibration must **remove an actual term**, or the seam is decoration.
/// Without it the population `S/T` spread widens every interval; with it the
/// spread is zero and the crease offset is the lifter's own.
#[test]
fn calibration_narrows_the_interval_without_changing_a_code_path() {
    let s = squat_series(Lifter::side(), CameraView::Side, 14.0, 1);
    let calibrated = Calibration {
        shoulder_span_to_torso: 0.85,
        thigh_to_shank: 0.90 / 0.85,
        hip_crease_offset_thigh_fraction: 0.02,
    };

    let AnalysisOutcome::Analyzed(without) = analyze(&s, None) else {
        panic!("expected an analysis without calibration");
    };
    let AnalysisOutcome::Analyzed(with) = analyze(&s, Some(&calibrated)) else {
        panic!("calibration must never change whether the analysis runs");
    };

    assert_eq!(
        without.rep_count, with.rep_count,
        "calibration must not change a code path"
    );
    let a = finding(&without, Metric::SquatDepth, Some(1));
    let b = finding(&with, Metric::SquatDepth, Some(1));
    assert!(
        (a.value - b.value).abs() < 1e-9,
        "calibration narrows the interval; it does not move the value"
    );
    assert!(
        b.uncertainty < a.uncertainty,
        "calibration must actually remove a term: {} vs {}",
        b.uncertainty,
        a.uncertainty
    );
}

// ===========================================================================
// Output contract
// ===========================================================================

/// AC10b — what cannot be measured travels **with the result**, so the UI
/// cannot omit it, and back rounding is named explicitly.
#[test]
fn what_cannot_be_measured_is_carried_in_the_output() {
    let a = analyzed(&squat_series(Lifter::side(), CameraView::Side, -5.0, 1));
    assert_eq!(a.not_measurable.len(), NOT_MEASURABLE.len());
    assert!(
        a.not_measurable
            .iter()
            .any(|s| s.contains("Back rounding is not detectable")),
        "the fault most likely to injure someone must be named as unmeasurable"
    );
    assert_eq!(
        a.view,
        CameraView::Side,
        "the assumed view is always stated"
    );
}

/// Architect review finding 15 — the rep boundaries are exposed so a consumer
/// can slice the same series without re-running segmentation.
#[test]
fn rep_boundaries_are_exposed_and_well_ordered() {
    let a = analyzed(&squat_series(Lifter::side(), CameraView::Side, -5.0, 3));
    assert_eq!(a.reps.len(), usize::try_from(a.rep_count).unwrap());
    for rep in &a.reps {
        assert!(rep.start <= rep.extremum, "{rep:?}");
        assert!(rep.extremum <= rep.end, "{rep:?}");
        assert!(rep.end < a.reps.len() + 1_000);
    }
}

/// The series is a stable on-disk format (SPEC-0044 §2.10.2), and a pose
/// serializes as a **bare array** — a wrapper object per pose would put a field
/// name on disk four hundred times to say nothing.
#[test]
fn series_round_trips_and_poses_serialize_transparently() {
    let s = squat_series(Lifter::side(), CameraView::Side, -5.0, 1);
    let json = serde_json::to_string(&s).unwrap();
    let back: LiftSeries = serde_json::from_str(&json).unwrap();
    assert_eq!(back, s);
    assert_eq!(back.schema_version, 1);
    assert_eq!(back.frame_width, s.frame_width);
    assert_eq!(back.frame_height, s.frame_height);

    let pose_json = serde_json::to_string(&s.frames[0].pose).unwrap();
    assert!(
        pose_json.starts_with('['),
        "a pose must serialize as a bare array, got {pose_json}"
    );

    let outcome_json = serde_json::to_string(&analyze(&s, None)).unwrap();
    assert!(
        outcome_json.contains("\"outcome\":\"analyzed\""),
        "the persisted shape is a tagged enum, not a serialized Result"
    );
}
