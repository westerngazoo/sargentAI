//! The **synthetic pose generator** — a segment model, posed by joint angles,
//! projected under a camera yaw, and mapped into MoveNet's isotropic
//! letterboxed canvas.
//!
//! Architect review finding 16 calls this out as load-bearing and unspecified,
//! so it is specified here.
//!
//! # Why it must exist
//!
//! R-0044 AC17 requires the analysis suite to run with **no video whatsoever**.
//! Hand-authoring 17 keypoints × 60 frames per scenario is unreadable and, worse,
//! unfalsifiable: a test would assert whatever geometry the author happened to
//! type. A generator lets a test say *"a squat that bottoms out 5° below
//! parallel"* and then assert the analysis recovers −5°, which is a real test of
//! the measurement rather than of the fixture.
//!
//! # The coordinate contract (SPEC-0044 §2.2.2) — load-bearing
//!
//! Fixtures **must** be authored in the same space production sees: the model's
//! letterboxed square canvas, which is source pixels under **one uniform scale
//! plus a translation**. That space is isotropic, so angles and length ratios
//! survive it. A fixture authored in an anisotropic space (`x/width`, `y/height`)
//! would make every test measure a different geometry from production, and the
//! suite would pass while the product was wrong.
//!
//! [`Lifter::project`] therefore ends with exactly one uniform [`CANVAS_SCALE`]
//! and one translation, and never scales `x` and `y` differently.
//!
//! # The model
//!
//! A lifter is a set of joints in a right-handed body frame, origin at the
//! mid-ankle:
//!
//! - `x` — **lateral**, positive toward the lifter's left. The frontal-plane
//!   axis; shoulder and hip spans lie along it.
//! - `y` — **anterior**, positive in front of the lifter. The sagittal axis.
//! - `z` — **up**.
//!
//! A camera at yaw `θ` (0° = front-on, 90° = side-on) looks horizontally, so the
//! image axes are
//!
//! ```text
//! u = x·cos θ + y·sin θ        (image horizontal)
//! v = z                        (image vertical, flipped to image-y at the end)
//! ```
//!
//! which is exactly the projection SPEC-0044 §2.5.1 reasons about: a shoulder
//! span lying along `x` images as `S·cos θ`, while the torso — a segment along
//! the body's long axis, and a yaw is a rotation *about* that axis — keeps its
//! image length. That is what makes `shoulder_span / torso_length` a clean
//! cosine and the rejected draft's `shoulder_span / hip_span` a constant.
//!
//! Upright lifts (squat, deadlift) are built in this frame and projected.
//! The bench is built directly in the image plane: a supine lifter filmed
//! perpendicular to the bench has no interesting yaw sweep, and the honest
//! fixture is "both sides land on nearly the same pixel, the far side scores
//! low".

// The generator's prose names the pose model and the COCO-17 vocabulary.
#![allow(clippy::doc_markdown)]

use std::f64::consts::PI;

use crate::pose::{Keypoint, Landmark, PoseKeypoints};
use crate::technique::{CameraView, Lift, LiftSeries, PoseFrame, SAMPLE_HZ};

// ---------------------------------------------------------------------------
// Canvas mapping — one uniform scale, one translation (SPEC-0044 §2.2.2)
// ---------------------------------------------------------------------------

/// Body units → canvas units. A standing lifter is ~3.2 torso lengths tall, so
/// 0.26 puts the whole body inside the unit canvas with margin at both ends.
///
/// This is the **only** scale applied, and it is applied to `u` and `v`
/// identically — that is the isotropy contract, in one constant.
const CANVAS_SCALE: f64 = 0.26;

/// Canvas position of the body-frame origin (the mid-ankle).
const CANVAS_ORIGIN_U: f64 = 0.5;
const CANVAS_ORIGIN_V: f64 = 0.95;

/// Score given to a landmark the camera can see plainly.
const SCORE_NEAR: f64 = 0.95;
/// Score given to a landmark occluded by the lifter's own body.
const SCORE_FAR: f64 = 0.10;

// ---------------------------------------------------------------------------
// Segment model
// ---------------------------------------------------------------------------

/// Segment lengths, in body units. Torso is 1.0 by definition, so every other
/// length reads as a fraction of a torso and every threshold in SPEC-0044 —
/// which is expressed in torso lengths — is directly legible here.
#[derive(Clone, Copy, Debug)]
pub(super) struct Build {
    pub torso: f64,
    pub shoulder_span: f64,
    pub hip_span: f64,
    pub thigh: f64,
    pub shank: f64,
    pub upper_arm: f64,
    pub forearm: f64,
    /// Head centre above the shoulder line.
    pub head_rise: f64,
    /// Ear half-separation, lateral.
    pub ear_half_span: f64,
    /// How far the nose sits in front of the head centre.
    pub nose_anterior: f64,
    /// Half the distance between the ankles.
    pub stance_half: f64,
}

impl Default for Build {
    /// Roughly average proportions; `shoulder_span / torso = 0.85` matches the
    /// nominal `S/T` SPEC-0044 §2.5.3 sizes the view band against, and
    /// `hip_span / torso = 0.62` sits clear of `HIP_VIEW_FRONT_MIN` so a
    /// fixture never balances on a threshold it is not testing.
    fn default() -> Self {
        Self {
            torso: 1.0,
            shoulder_span: 0.85,
            hip_span: 0.62,
            thigh: 0.90,
            shank: 0.85,
            upper_arm: 0.62,
            forearm: 0.58,
            head_rise: 0.42,
            ear_half_span: 0.16,
            nose_anterior: 0.20,
            stance_half: 0.22,
        }
    }
}

/// A point in the body frame.
#[derive(Clone, Copy, Debug)]
struct P3 {
    x: f64,
    y: f64,
    z: f64,
}

impl P3 {
    const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

/// One posed frame of an upright lifter, before projection.
#[derive(Clone, Copy, Debug)]
pub(super) struct UprightPose {
    /// Thigh pitch: the angle of the **knee→hip** segment from straight up,
    /// rotating posteriorly. `0°` = hip directly above knee (standing); `90°` =
    /// hip level with the knee; `> 90°` = hip below the knee.
    ///
    /// The analysis reports depth as the hip→knee angle from horizontal,
    /// positive when the hip is above the knee, so the ground truth is exactly
    /// **`90° − thigh_pitch_deg`**.
    pub thigh_pitch_deg: f64,
    /// Shank pitch: the angle of the ankle→knee segment from straight up,
    /// rotating anteriorly (the knee travelling forward over the foot).
    pub shank_pitch_deg: f64,
    /// Torso pitch: the angle of the hip→shoulder segment from straight up,
    /// rotating anteriorly. This is exactly the "torso angle from vertical" the
    /// analysis reports.
    pub torso_pitch_deg: f64,
    /// Where the hands are, as an absolute height in body units above the
    /// mid-ankle. `None` puts them on the shoulders (a back squat).
    pub hand_height: Option<f64>,
    /// Lateral displacement of each knee toward the midline, in shank lengths.
    /// Positive is inward. Applied to both knees.
    pub knee_inward: f64,
    /// Rigid lateral shift of the whole lifter, in body units — a walkout.
    pub shift_x: f64,
    /// Rigid anterior shift of the whole lifter, in body units.
    pub shift_y: f64,
}

impl Default for UprightPose {
    fn default() -> Self {
        Self {
            thigh_pitch_deg: 0.0,
            shank_pitch_deg: 0.0,
            torso_pitch_deg: 0.0,
            hand_height: None,
            knee_inward: 0.0,
            shift_x: 0.0,
            shift_y: 0.0,
        }
    }
}

/// A synthetic lifter: a [`Build`] filmed at a camera yaw.
#[derive(Clone, Copy, Debug)]
pub(super) struct Lifter {
    pub build: Build,
    /// Camera yaw in degrees: `0` front-on, `90` side-on.
    pub yaw_deg: f64,
    /// Which side is nearest the camera in a side view (the other is occluded).
    pub near_is_left: bool,
}

impl Default for Lifter {
    fn default() -> Self {
        Self {
            build: Build::default(),
            yaw_deg: 90.0,
            near_is_left: true,
        }
    }
}

impl Lifter {
    /// A side-on camera.
    pub(super) fn side() -> Self {
        Self::default()
    }

    /// A front-on camera.
    pub(super) fn front() -> Self {
        Self {
            yaw_deg: 0.0,
            ..Self::default()
        }
    }

    /// The camera yaw, in radians.
    fn yaw(self) -> f64 {
        self.yaw_deg.to_radians()
    }

    /// Project a body-frame point into the isotropic canvas.
    ///
    /// **One uniform scale, one translation** — the whole contract of
    /// SPEC-0044 §2.2.2 lives in these three lines.
    fn project(self, point: P3) -> (f32, f32) {
        let (sin, cos) = self.yaw().sin_cos();
        let across = point.x.mul_add(cos, point.y * sin);
        let up = point.z;
        #[allow(clippy::cast_possible_truncation)] // fixtures are small and exact enough in f32
        (
            across.mul_add(CANVAS_SCALE, CANVAS_ORIGIN_U) as f32,
            CANVAS_ORIGIN_V.mul_add(1.0, -(up * CANVAS_SCALE)) as f32,
        )
    }

    /// How visible a landmark on `side` is. Side-on, the far limb is behind the
    /// body; front-on, both are plain. The transition is continuous so the yaw
    /// sweep is not a step function.
    fn score_for(self, is_left: bool) -> f64 {
        let near = self.near_is_left == is_left;
        let frontality = self.yaw().cos().abs();
        if near {
            SCORE_NEAR
        } else {
            SCORE_FAR + (SCORE_NEAR - SCORE_FAR) * frontality * frontality
        }
    }

    /// The far ear disappears behind the head faster than a limb does.
    fn ear_score(self, is_left: bool) -> f64 {
        let near = self.near_is_left == is_left;
        let frontality = self.yaw().cos().abs();
        if near {
            SCORE_NEAR
        } else {
            0.03 + (SCORE_NEAR - 0.03) * frontality.powi(3)
        }
    }

    /// Build the 17 keypoints for an upright pose.
    pub(super) fn upright(self, pose: UprightPose) -> PoseKeypoints {
        let shift = |q: P3| P3::new(q.x + pose.shift_x, q.y + pose.shift_y, q.z);
        let mut points = [Keypoint {
            x: 0.0,
            y: 0.0,
            score: 0.0,
        }; 17];
        let mut set = |landmark: Landmark, point: P3, score: f64| {
            let (x, y) = self.project(shift(point));
            #[allow(clippy::cast_possible_truncation)]
            {
                points[landmark.index()] = Keypoint {
                    x,
                    y,
                    score: score as f32,
                };
            }
        };

        let hips = self.set_legs(&mut set, pose);
        let hip_mid = P3::new(
            0.0,
            f64::midpoint(hips[0].y, hips[1].y),
            f64::midpoint(hips[0].z, hips[1].z),
        );
        let torso = pose.torso_pitch_deg.to_radians();
        let shoulder_mid = P3::new(
            0.0,
            self.build.torso.mul_add(torso.sin(), hip_mid.y),
            self.build.torso.mul_add(torso.cos(), hip_mid.z),
        );
        self.set_arms(&mut set, pose, shoulder_mid);
        self.set_head(&mut set, shoulder_mid, torso);
        PoseKeypoints::new(points)
    }

    /// Ankle, knee and hip per side; returns the two hip positions so the torso
    /// can be hung from their midpoint.
    fn set_legs(self, set: &mut impl FnMut(Landmark, P3, f64), pose: UprightPose) -> [P3; 2] {
        let b = self.build;
        let thigh = pose.thigh_pitch_deg.to_radians();
        let shank = pose.shank_pitch_deg.to_radians();
        let mut hips = [P3::new(0.0, 0.0, 0.0); 2];
        for (index, is_left) in [true, false].into_iter().enumerate() {
            let lateral = if is_left { 1.0 } else { -1.0 };
            let ankle = P3::new(lateral * b.stance_half, 0.0, 0.0);
            // Knee: shank pitched anteriorly, then drawn toward the midline.
            let knee = P3::new(
                (-lateral * pose.knee_inward).mul_add(b.shank, ankle.x),
                b.shank.mul_add(shank.sin(), ankle.y),
                b.shank.mul_add(shank.cos(), ankle.z),
            );
            // Hip: thigh pitched posteriorly from the knee.
            let hip = P3::new(
                lateral * b.hip_span / 2.0,
                b.thigh.mul_add(-thigh.sin(), knee.y),
                b.thigh.mul_add(thigh.cos(), knee.z),
            );
            hips[index] = hip;
            let score = self.score_for(is_left);
            let pick = |left, right| if is_left { left } else { right };
            set(
                pick(Landmark::LeftAnkle, Landmark::RightAnkle),
                ankle,
                score,
            );
            set(pick(Landmark::LeftKnee, Landmark::RightKnee), knee, score);
            set(pick(Landmark::LeftHip, Landmark::RightHip), hip, score);
        }
        hips
    }

    /// Shoulder, elbow and wrist per side. The hands rest on the bar: at
    /// shoulder height for a back squat, or at a commanded height for a
    /// deadlift's bar.
    fn set_arms(
        self,
        set: &mut impl FnMut(Landmark, P3, f64),
        pose: UprightPose,
        shoulder_mid: P3,
    ) {
        let b = self.build;
        for is_left in [true, false] {
            let lateral = if is_left { 1.0 } else { -1.0 };
            let half_span = lateral * b.shoulder_span / 2.0;
            let shoulder = P3::new(half_span, shoulder_mid.y, shoulder_mid.z);
            let wrist = P3::new(
                half_span,
                shoulder.y,
                pose.hand_height.unwrap_or(shoulder.z),
            );
            let elbow = P3::new(
                f64::midpoint(shoulder.x, wrist.x),
                f64::midpoint(shoulder.y, wrist.y),
                f64::midpoint(shoulder.z, wrist.z) - b.upper_arm * 0.08,
            );
            let score = self.score_for(is_left);
            let pick = |left, right| if is_left { left } else { right };
            set(
                pick(Landmark::LeftShoulder, Landmark::RightShoulder),
                shoulder,
                score,
            );
            set(
                pick(Landmark::LeftElbow, Landmark::RightElbow),
                elbow,
                score,
            );
            set(
                pick(Landmark::LeftWrist, Landmark::RightWrist),
                wrist,
                score,
            );
        }
    }

    /// Head landmarks: the head rides on top of the torso, tilted with it, and
    /// the nose sits anterior to the head centre — which is what makes
    /// `|nose.x − shoulder_mid.x| / torso` an orthogonal side/front cue.
    fn set_head(self, set: &mut impl FnMut(Landmark, P3, f64), shoulder_mid: P3, torso: f64) {
        let b = self.build;
        let head = P3::new(
            0.0,
            b.head_rise.mul_add(torso.sin(), shoulder_mid.y),
            b.head_rise.mul_add(torso.cos(), shoulder_mid.z),
        );
        let nose = P3::new(head.x, head.y + b.nose_anterior, head.z);
        set(Landmark::Nose, nose, SCORE_NEAR);
        for is_left in [true, false] {
            let lateral = if is_left { 1.0 } else { -1.0 };
            let ear = P3::new(lateral * b.ear_half_span, head.y, head.z);
            let eye = P3::new(
                lateral * b.ear_half_span * 0.45,
                head.y + b.nose_anterior * 0.8,
                head.z + 0.03,
            );
            set(
                if is_left {
                    Landmark::LeftEar
                } else {
                    Landmark::RightEar
                },
                ear,
                self.ear_score(is_left),
            );
            set(
                if is_left {
                    Landmark::LeftEye
                } else {
                    Landmark::RightEye
                },
                eye,
                self.score_for(is_left),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The bench: a supine lifter, authored directly in the image plane
// ---------------------------------------------------------------------------

/// One posed frame of a bench press, in body units along the bench.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct BenchPose {
    /// Height of the wrists above the chest, in torso lengths. `0` is the touch.
    pub bar_height: f64,
    /// Horizontal position of the bar relative to the shoulder, in torso
    /// lengths, positive **toward the feet**.
    pub bar_caudal: f64,
    /// Angle of the wrist→elbow segment from vertical, degrees, positive when
    /// the elbow is **caudal** of the wrist. The analysis reports exactly this.
    pub forearm_angle_deg: f64,
}

/// A bench-press pose in the isotropic canvas.
///
/// The lifter lies with the head at small `u` and the feet at large `u`; the
/// bar rises in `−y`. Both sides land on nearly the same pixel — a genuine
/// perpendicular side view — with a small residual span so the view cues see a
/// realistic near-zero rather than an exact zero, and the far side scored low.
pub(super) fn bench_pose(pose: BenchPose) -> PoseKeypoints {
    const TORSO: f64 = 1.0;
    /// Residual image span of a bilateral pair under a perpendicular view.
    const RESIDUAL_SPAN: f64 = 0.08;
    const ORIGIN_U: f64 = 0.25;
    const ORIGIN_V: f64 = 0.75;

    // One set of proportions across both generators.
    let forearm = Build::default().forearm;
    let mut points = [Keypoint {
        x: 0.0,
        y: 0.0,
        score: 0.0,
    }; 17];
    #[allow(clippy::cast_possible_truncation)]
    let mut set = |landmark: Landmark, u: f64, v: f64, score: f64| {
        points[landmark.index()] = Keypoint {
            x: u.mul_add(CANVAS_SCALE, ORIGIN_U) as f32,
            y: ORIGIN_V.mul_add(1.0, -(v * CANVAS_SCALE)) as f32,
            score: score as f32,
        };
    };

    let shoulder_u = 0.0;
    let hip_u = TORSO;
    let bench_v = 0.0;

    let wrist_u = shoulder_u + pose.bar_caudal;
    let wrist_v = bench_v + pose.bar_height + 0.30;
    let a = pose.forearm_angle_deg.to_radians();
    let elbow_u = forearm.mul_add(a.sin(), wrist_u);
    let elbow_v = forearm.mul_add(-a.cos(), wrist_v);

    for (is_left, offset, score) in [(true, 0.0, SCORE_NEAR), (false, RESIDUAL_SPAN, SCORE_FAR)] {
        let pick = |l: Landmark, r: Landmark| if is_left { l } else { r };
        set(
            pick(Landmark::LeftShoulder, Landmark::RightShoulder),
            shoulder_u,
            bench_v + offset,
            score,
        );
        set(
            pick(Landmark::LeftHip, Landmark::RightHip),
            hip_u,
            bench_v + offset,
            score,
        );
        set(
            pick(Landmark::LeftWrist, Landmark::RightWrist),
            wrist_u,
            wrist_v + offset,
            score,
        );
        set(
            pick(Landmark::LeftElbow, Landmark::RightElbow),
            elbow_u,
            elbow_v + offset,
            score,
        );
        set(
            pick(Landmark::LeftKnee, Landmark::RightKnee),
            hip_u + 0.85,
            bench_v + offset,
            score,
        );
        set(
            pick(Landmark::LeftAnkle, Landmark::RightAnkle),
            hip_u + 1.5,
            bench_v - 0.45 + offset,
            score,
        );
        set(
            pick(Landmark::LeftEar, Landmark::RightEar),
            shoulder_u - 0.35,
            bench_v + offset,
            if is_left { SCORE_NEAR } else { 0.03 },
        );
        set(
            pick(Landmark::LeftEye, Landmark::RightEye),
            shoulder_u - 0.40,
            bench_v + 0.05 + offset,
            score,
        );
    }
    // The nose sits well off the shoulder line along the image horizontal —
    // side-on, that offset is the orthogonal cue.
    set(
        Landmark::Nose,
        shoulder_u - 0.45,
        bench_v + 0.05,
        SCORE_NEAR,
    );

    PoseKeypoints::new(points)
}

// ---------------------------------------------------------------------------
// Series assembly
// ---------------------------------------------------------------------------

/// Wrap poses into a [`LiftSeries`] at the nominal [`SAMPLE_HZ`].
pub(super) fn series(lift: Lift, view: CameraView, poses: Vec<PoseKeypoints>) -> LiftSeries {
    let step_ms = 1000 / SAMPLE_HZ;
    LiftSeries {
        schema_version: 1,
        lift,
        view,
        frame_width: 854,
        frame_height: 480,
        sample_hz: SAMPLE_HZ,
        frames: poses
            .into_iter()
            .enumerate()
            .map(|(i, pose)| PoseFrame {
                #[allow(clippy::cast_possible_truncation)]
                t_ms: u32::try_from(i).unwrap_or(u32::MAX) * step_ms,
                pose,
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Motion helpers
// ---------------------------------------------------------------------------

/// A smooth 0→1 ramp with zero-slope ends — the shape a limb actually follows,
/// and (unlike a linear ramp) it gives a rep a genuine turning point rather
/// than a corner.
pub(super) fn ease(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    0.5 - 0.5 * (PI * t).cos()
}

/// `n` samples of `f` over `t ∈ [0, 1]`.
pub(super) fn ramp(n: usize, f: impl Fn(f64) -> f64) -> Vec<f64> {
    #[allow(clippy::cast_precision_loss)] // frame counts are small
    (0..n)
        .map(|i| {
            let t = if n <= 1 {
                0.0
            } else {
                i as f64 / (n - 1) as f64
            };
            f(t)
        })
        .collect()
}

/// `n` frames all holding `value`.
pub(super) fn hold(n: usize, value: f64) -> Vec<f64> {
    vec![value; n]
}

/// A deterministic ±1 pseudo-random sequence — a 32-bit xorshift, so a jitter
/// test is reproducible and a failure is debuggable. Never `rand`: the suite
/// must be deterministic (test 21).
pub(super) struct Jitter(u32);

impl Jitter {
    pub(super) const fn new(seed: u32) -> Self {
        Self(seed | 1)
    }

    /// The next value in `[-1, 1]`.
    pub(super) fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        f64::from(self.0 >> 8).mul_add(2.0 / f64::from(1_u32 << 24), -1.0)
    }
}

/// Add independent jitter of `amplitude` (in canvas units) to every keypoint of
/// every frame, leaving the scores alone.
pub(super) fn jitter_series(series: &LiftSeries, amplitude: f64, seed: u32) -> LiftSeries {
    let mut rng = Jitter::new(seed);
    let mut out = series.clone();
    for frame in &mut out.frames {
        let mut points = *frame.pose.points();
        for point in &mut points {
            #[allow(clippy::cast_possible_truncation)]
            {
                point.x += (rng.next() * amplitude) as f32;
                point.y += (rng.next() * amplitude) as f32;
            }
        }
        frame.pose = PoseKeypoints::new(points);
    }
    out
}
