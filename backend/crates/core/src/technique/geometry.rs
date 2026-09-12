//! Near-side resolution, segment normalizers, angles and uncertainty
//! (SPEC-0044 §2.7.1–2.7.2, §2.8).
//!
//! Everything here is computed in the isotropic canvas space of SPEC-0044
//! §2.2.2, so angles and dimensionless ratios need no aspect correction.
//!
//! ## Sign convention
//!
//! Image `y` grows downward, so "height" is `−y` throughout.

use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::pose::{Keypoint, Landmark, PoseKeypoints, CONFIDENCE_FLOOR};

use super::{
    median, Bilateral, CameraView, Finding, Lift, LiftSeries, Measurement, Metric, Refusal, Side,
    Unit, Unmeasurable, HIP_CREASE_OFFSET_THIGH_FRACTION, KEYPOINT_SIGMA_FRACTION,
    MIN_FRAME_COVERAGE, MIN_MEAN_CONFIDENCE, POPULATION_RATIO_SPREAD,
};

/// A length below this is degenerate: dividing by it would manufacture a number
/// out of noise, so the caller reports the measurement as unavailable instead.
pub(crate) const DEGENERATE_LENGTH: f64 = 1e-6;

/// The height of a keypoint. Image `y` grows downward, so up is `−y`.
pub(crate) fn height(k: Keypoint) -> f64 {
    -f64::from(k.y)
}

/// The horizontal position of a keypoint.
pub(crate) fn horizontal(k: Keypoint) -> f64 {
    f64::from(k.x)
}

/// The midpoint of two keypoints; the score is the lower of the two, since a
/// midpoint is only as trustworthy as its worst end.
pub(crate) fn midpoint(a: Keypoint, b: Keypoint) -> Keypoint {
    Keypoint {
        x: f32::midpoint(a.x, b.x),
        y: f32::midpoint(a.y, b.y),
        score: a.score.min(b.score),
    }
}

/// Mean of the given keypoints' scores — what a [`Finding`](super::Finding)
/// reports as its `confidence`.
pub(crate) fn mean_score(points: &[Keypoint]) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)] // at most 17 points
    let n = points.len() as f64;
    points.iter().map(|k| f64::from(k.score)).sum::<f64>() / n
}

// ---------------------------------------------------------------------------
// Angles
// ---------------------------------------------------------------------------

/// The angle of the segment `from → to` above the horizontal, in degrees,
/// **positive when `from` is higher than `to`**.
///
/// This is squat depth: `from` is the hip, `to` is the knee, and a positive
/// value is "the hip is above the knee" — shallow.
pub(crate) fn angle_above_horizontal_deg(from: Keypoint, to: Keypoint) -> f64 {
    let rise = height(from) - height(to);
    let run = (horizontal(to) - horizontal(from)).abs();
    rise.atan2(run).to_degrees()
}

/// The unsigned angle of the segment `a → b` from the vertical, in degrees.
///
/// Used for torso inclination, where only the magnitude of the lean and of its
/// change through a rep is meaningful.
pub(crate) fn angle_from_vertical_deg(a: Keypoint, b: Keypoint) -> f64 {
    let vertical = (height(b) - height(a)).abs();
    let lateral = (horizontal(b) - horizontal(a)).abs();
    lateral.atan2(vertical).to_degrees()
}

/// The signed angle of the segment `a → b` from the vertical, in degrees,
/// positive when `b` lies in the `positive_x` direction from `a`.
///
/// Used for the bench forearm angle, where "which way the elbow points" is the
/// whole cue. `positive_x` is `+1.0` when increasing image `x` is the direction
/// the sign convention calls positive, `−1.0` when it is the other way.
pub(crate) fn signed_angle_from_vertical_deg(a: Keypoint, b: Keypoint, positive_x: f64) -> f64 {
    let vertical = (height(a) - height(b)).abs();
    let lateral = (horizontal(b) - horizontal(a)) * positive_x;
    lateral.atan2(vertical).to_degrees()
}

// ---------------------------------------------------------------------------
// Uncertainty (SPEC-0044 §2.8)
// ---------------------------------------------------------------------------

/// The positional scale of one keypoint, in canvas units.
///
/// **Weighted per landmark** (architect review finding 11): the nominal scale
/// is inflated in inverse proportion to the model's own score for that point,
/// bounded below by [`CONFIDENCE_FLOOR`] so a near-zero score cannot produce an
/// infinite interval. A point the model is unsure of contributes more
/// uncertainty than one it is sure of — a flat σ across all seventeen
/// landmarks would say the opposite.
pub(crate) fn sigma(k: Keypoint, torso: f64) -> f64 {
    let score = f64::from(k.score).max(f64::from(CONFIDENCE_FLOOR));
    KEYPOINT_SIGMA_FRACTION * torso / score
}

/// First-order uncertainty of an angle between two points `separation` apart,
/// in degrees.
pub(crate) fn angle_uncertainty_deg(sigma_a: f64, sigma_b: f64, separation: f64) -> f64 {
    if separation <= DEGENERATE_LENGTH {
        return 180.0;
    }
    (sigma_a.hypot(sigma_b) / separation).to_degrees()
}

/// First-order uncertainty of a length normalized by `normalizer`.
pub(crate) fn offset_uncertainty(sigma_a: f64, sigma_b: f64, normalizer: f64) -> f64 {
    if normalizer <= DEGENERATE_LENGTH {
        return f64::INFINITY;
    }
    sigma_a.hypot(sigma_b) / normalizer
}

/// First-order uncertainty of the ratio `a / b`.
pub(crate) fn ratio_uncertainty(a: f64, sigma_a: f64, b: f64, sigma_b: f64) -> f64 {
    if a.abs() <= DEGENERATE_LENGTH || b.abs() <= DEGENERATE_LENGTH {
        return f64::INFINITY;
    }
    (a / b).abs() * (sigma_a / a).hypot(sigma_b / b)
}

// ---------------------------------------------------------------------------
// Normalizers and calibration (SPEC-0044 §2.7.2, §2.11 — AC7)
// ---------------------------------------------------------------------------

/// The lifter's own segment lengths, in the isotropic canvas units of
/// SPEC-0044 §2.2.2 — dimensionless once a measurement is divided by one.
///
/// Taken at the reference window rather than per frame: a per-frame normalizer
/// would let a foreshortening limb rescale the very measurement it normalizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segments {
    /// Shoulder → hip.
    pub torso: f64,
    /// Knee → ankle.
    pub shank: f64,
    /// Hip → knee.
    pub thigh: f64,
    /// Wrist → elbow.
    pub forearm: f64,
}

/// The lifter's own limb ratios (AC7).
///
/// **Deferred in v1:** no capture UI, no storage, no endpoint. The type and the
/// `Option` parameter on [`analyze`](super::analyze) ship now so adding the
/// capture flow later is purely additive — and so that what calibration buys is
/// visible in the code rather than promised in a document.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Calibration {
    /// The lifter's own `shoulder_span / torso_length`. Collapses the
    /// deliberately wide view band, which exists only because this varies
    /// between people.
    pub shoulder_span_to_torso: f64,
    /// The lifter's own `thigh_length / shank_length`.
    pub thigh_to_shank: f64,
    /// Where the lifter's hip crease sits relative to the hip joint centre the
    /// model reports, as a fraction of thigh length. This is the systematic
    /// term that costs squat depth its precision.
    pub hip_crease_offset_thigh_fraction: f64,
}

/// The single place every ratio and every uncertainty is read through.
///
/// Absence of a [`Calibration`] changes **numbers, never a code path**: the
/// population variant carries a [`POPULATION_RATIO_SPREAD`] term and the
/// population bound on the hip-crease offset; the calibrated variant drops the
/// first outright and replaces the second with the lifter's own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Normalizers {
    pub(crate) segments: Segments,
    population_spread: f64,
    hip_crease_offset_thigh_fraction: f64,
}

impl Normalizers {
    /// Population priors — and the spread term that stands in for not knowing
    /// the lifter's own ratios.
    #[must_use]
    pub fn population(segments: Segments) -> Self {
        Self {
            segments,
            population_spread: POPULATION_RATIO_SPREAD,
            hip_crease_offset_thigh_fraction: HIP_CREASE_OFFSET_THIGH_FRACTION,
        }
    }

    /// The lifter's own ratios — which **drops** the population spread term.
    #[must_use]
    pub fn from_calibration(segments: Segments, calibration: &Calibration) -> Self {
        Self {
            segments,
            population_spread: 0.0,
            hip_crease_offset_thigh_fraction: calibration.hip_crease_offset_thigh_fraction,
        }
    }

    pub(crate) fn resolve(segments: Segments, calibration: Option<&Calibration>) -> Self {
        calibration.map_or_else(
            || Self::population(segments),
            |c| Self::from_calibration(segments, c),
        )
    }

    /// Widen a random-error half-width by the population term.
    ///
    /// The residual out-of-plane yaw is recoverable only through the lifter's
    /// `S/T`, so an unknown `S/T` leaves the foreshortening factor uncertain by
    /// a *relative* amount — which enters proportionally to the value.
    pub(crate) fn widen(&self, value: f64, random: f64) -> f64 {
        random.hypot(self.population_spread * value.abs())
    }

    /// The bound the hip crease's offset from the joint centre puts on a depth
    /// angle, in degrees.
    ///
    /// The offset's **sign and magnitude are unestablished**: at the extremum
    /// the thigh is near horizontal and the offset lies close to parallel with
    /// it, contributing little angular error, and which way the residual falls
    /// is not known. It is therefore carried as a worst-case bound — the offset
    /// taken perpendicular to the thigh, `atan(offset / thigh)`, in which the
    /// thigh cancels — rather than as a correction with an invented sign.
    pub(crate) fn crease_bound_deg(&self) -> f64 {
        self.hip_crease_offset_thigh_fraction.atan().to_degrees()
    }
}

/// Everything a per-lift module needs to measure.
pub(crate) struct Context<'a> {
    pub(crate) series: &'a LiftSeries,
    pub(crate) side: Side,
    pub(crate) norms: Normalizers,
}

impl Context<'_> {
    /// Resolve a bilateral joint for measurement: the near side for a side
    /// view, the midpoint of both for a front view (SPEC-0044 §2.7.1).
    pub(crate) fn joint(&self, frame: usize, joint: Bilateral) -> Keypoint {
        resolve(
            &self.series.frames[frame].pose,
            self.series.view,
            self.side,
            joint,
        )
    }

    /// A bilateral joint on one named side, for the per-side front-view
    /// measurements.
    pub(crate) fn joint_on(&self, frame: usize, joint: Bilateral, side: Side) -> Keypoint {
        self.series.frames[frame].pose.get(joint.on(side))
    }

    /// The positional scale of a keypoint, at this lifter's torso length.
    pub(crate) fn sigma(&self, k: Keypoint) -> f64 {
        sigma(k, self.norms.segments.torso)
    }
}

/// Resolve a bilateral joint under the near-side rule.
///
/// - `Side`: the near side only. **The far side is never averaged in** — a
///   midpoint of a near and an occluded far landmark is a fabricated centre
///   line whose error grows with limb separation, and is largest at the bottom
///   of a squat, which is exactly where depth is read.
/// - `Front`: the midpoint of both.
pub(crate) fn resolve(
    pose: &PoseKeypoints,
    view: CameraView,
    near: Side,
    joint: Bilateral,
) -> Keypoint {
    match view {
        CameraView::Side => pose.get(joint.on(near)),
        CameraView::Front => midpoint(
            pose.get(joint.on(Side::Left)),
            pose.get(joint.on(Side::Right)),
        ),
    }
}

// ---------------------------------------------------------------------------
// Segment measurement
// ---------------------------------------------------------------------------

impl Segments {
    /// The median segment lengths over a frame range.
    ///
    /// Each length is taken over the frames in which **its own** endpoints are
    /// confident, so an unconfident forearm does not cost the torso its
    /// normalizer. A length no frame could measure is `0.0`, which callers
    /// treat as [`DEGENERATE_LENGTH`] and report as unavailable rather than
    /// dividing by.
    pub(crate) fn median(series: &LiftSeries, side: Side, range: Range<usize>) -> Self {
        let mut torso = Vec::new();
        let mut shank = Vec::new();
        let mut thigh = Vec::new();
        let mut forearm = Vec::new();
        for frame in
            &series.frames[range.start.min(series.frames.len())..range.end.min(series.frames.len())]
        {
            let push = |out: &mut Vec<f64>, a: Bilateral, b: Bilateral| {
                if let Some(len) = segment_len(&frame.pose, series.view, side, a, b) {
                    out.push(len);
                }
            };
            push(&mut torso, Bilateral::Shoulder, Bilateral::Hip);
            push(&mut shank, Bilateral::Knee, Bilateral::Ankle);
            push(&mut thigh, Bilateral::Hip, Bilateral::Knee);
            push(&mut forearm, Bilateral::Wrist, Bilateral::Elbow);
        }
        Self {
            torso: median(&torso),
            shank: median(&shank),
            thigh: median(&thigh),
            forearm: median(&forearm),
        }
    }
}

/// The image length of one segment, or `None` when either end is unconfident.
fn segment_len(
    pose: &PoseKeypoints,
    view: CameraView,
    side: Side,
    a: Bilateral,
    b: Bilateral,
) -> Option<f64> {
    let confident = |joint: Bilateral| match view {
        CameraView::Side => pose.get(joint.on(side)).is_confident(),
        CameraView::Front => {
            pose.get(joint.on(Side::Left)).is_confident()
                && pose.get(joint.on(Side::Right)).is_confident()
        }
    };
    if !confident(a) || !confident(b) {
        return None;
    }
    Some(resolve(pose, view, side, a).distance_to(resolve(pose, view, side, b)))
}

// ---------------------------------------------------------------------------
// The pipeline's geometric gates (SPEC-0044 §2.4 steps 3–4)
// ---------------------------------------------------------------------------

/// The near side: the one whose mean keypoint score over the **whole series**
/// is higher.
///
/// Chosen **once**, not per frame. Per-frame selection injects a step
/// discontinuity into every signal at each frame where the far limb momentarily
/// out-scores the near one — a fabricated jump in the middle of a rep.
pub(crate) fn near_side(series: &LiftSeries) -> Side {
    let total = |side: Side| -> f64 {
        series
            .frames
            .iter()
            .map(|f| {
                [
                    Bilateral::Shoulder,
                    Bilateral::Elbow,
                    Bilateral::Wrist,
                    Bilateral::Hip,
                    Bilateral::Knee,
                    Bilateral::Ankle,
                ]
                .into_iter()
                .map(|j| f64::from(f.pose.get(j.on(side)).score))
                .sum::<f64>()
            })
            .sum()
    };
    if total(Side::Right) > total(Side::Left) {
        Side::Right
    } else {
        Side::Left
    }
}

/// Step 3 — the landmarks this lift needs are confidently in frame.
pub(crate) fn check_framing(series: &LiftSeries, side: Side) -> Result<(), Refusal> {
    let total = series.frames.len();
    if total == 0 {
        return Ok(());
    }
    #[allow(clippy::cast_precision_loss)] // frame counts are bounded by MAX_FRAMES
    let allowed = (1.0 - MIN_FRAME_COVERAGE) * total as f64;

    let mut worst: Option<(Landmark, u32)> = None;
    for landmark in required_landmarks(series, side) {
        let missing = series
            .frames
            .iter()
            .filter(|f| !f.pose.get(landmark).is_confident())
            .count();
        #[allow(clippy::cast_precision_loss)]
        if missing as f64 > allowed {
            let missing = u32::try_from(missing).unwrap_or(u32::MAX);
            if worst.is_none_or(|(_, w)| missing > w) {
                worst = Some((landmark, missing));
            }
        }
    }
    if let Some((landmark, missing_frames)) = worst {
        return Err(Refusal::OutOfFrame {
            landmark,
            missing_frames,
            total_frames: u32::try_from(total).unwrap_or(u32::MAX),
        });
    }
    Ok(())
}

/// Step 4 — the load-bearing joints are confident enough to fit geometry to.
///
/// Confidence precedes everything geometric because the view cues are
/// themselves keypoint-derived: classifying a view from untrustworthy points is
/// fitting noise.
pub(crate) fn check_confidence(series: &LiftSeries, side: Side) -> Result<(), Refusal> {
    let landmarks = required_landmarks(series, side);
    if landmarks.is_empty() || series.frames.is_empty() {
        return Ok(());
    }
    let scores: Vec<f64> = series
        .frames
        .iter()
        .flat_map(|f| {
            landmarks
                .iter()
                .map(move |&l| f64::from(f.pose.get(l).score))
        })
        .collect();
    #[allow(clippy::cast_precision_loss)]
    let mean = scores.iter().sum::<f64>() / scores.len() as f64;
    if mean < MIN_MEAN_CONFIDENCE {
        return Err(Refusal::LowConfidence {
            mean,
            required: MIN_MEAN_CONFIDENCE,
        });
    }
    Ok(())
}

/// This lift's load-bearing landmarks, resolved to concrete COCO-17 names: the
/// near side for a side view, both sides for a front view.
fn required_landmarks(series: &LiftSeries, side: Side) -> Vec<Landmark> {
    let joints: &[Bilateral] = Lift::load_bearing(series.lift, series.view);
    match series.view {
        CameraView::Side => joints.iter().map(|j| j.on(side)).collect(),
        CameraView::Front => joints
            .iter()
            .flat_map(|j| [j.on(Side::Left), j.on(Side::Right)])
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Torso inclination — shared by squat and deadlift (SPEC-0044 §2.7.3)
// ---------------------------------------------------------------------------

/// The unsigned torso inclination from vertical at one frame, in degrees.
///
/// **This is not a back check** (`AC10b`). COCO-17 has no spine landmark: the
/// shoulder→hip line measures torso *inclination*, and a neutral spine and a
/// fully rounded spine at the same hip angle produce an identical line.
pub(crate) fn torso_angle_deg(ctx: &Context<'_>, frame: usize) -> f64 {
    angle_from_vertical_deg(
        ctx.joint(frame, Bilateral::Shoulder),
        ctx.joint(frame, Bilateral::Hip),
    )
}

/// The largest change in torso inclination over `range`, measured against
/// `baseline`, together with the frame it occurred at.
///
/// Both lifts that report it use exactly this shape; only the window differs. A
/// squat's window is the whole rep, since the torso can change on the way down
/// and again on the way up. A deadlift's is the pull, because for a deadlift
/// **the setup *is* the bottom** — comparing "setup versus bottom" would compare
/// a frame with itself and report zero.
pub(crate) fn torso_angle_change(
    ctx: &Context<'_>,
    metric: Metric,
    rep_no: u32,
    baseline: usize,
    range: std::ops::RangeInclusive<usize>,
) -> Measurement {
    let unavailable = Measurement::Unavailable {
        metric,
        rep: Some(rep_no),
        reason: Unmeasurable::LandmarkNotConfident,
    };
    let torso = ctx.norms.segments.torso;
    let (shoulder, hip) = (
        ctx.joint(baseline, Bilateral::Shoulder),
        ctx.joint(baseline, Bilateral::Hip),
    );
    if torso <= DEGENERATE_LENGTH || !shoulder.is_confident() || !hip.is_confident() {
        return unavailable;
    }

    let start_angle = torso_angle_deg(ctx, baseline);
    let mut best = (baseline, 0.0_f64);
    for frame in range {
        if frame >= ctx.series.frames.len() {
            break;
        }
        let change = (torso_angle_deg(ctx, frame) - start_angle).abs();
        if change > best.1 {
            best = (frame, change);
        }
    }
    let (frame, value) = best;

    let peak_shoulder = ctx.joint(frame, Bilateral::Shoulder);
    let peak_hip = ctx.joint(frame, Bilateral::Hip);
    // Two angles, each carrying the same first-order error, subtracted.
    let one = angle_uncertainty_deg(ctx.sigma(shoulder), ctx.sigma(hip), torso);
    let other = angle_uncertainty_deg(ctx.sigma(peak_shoulder), ctx.sigma(peak_hip), torso);
    Measurement::Measured(Finding {
        metric,
        rep: Some(rep_no),
        frame: Some(u32::try_from(frame).unwrap_or(u32::MAX)),
        side: None,
        value,
        unit: Unit::Degrees,
        uncertainty: ctx.norms.widen(value, one.hypot(other)),
        severity: None,
        confidence: mean_score(&[shoulder, hip, peak_shoulder, peak_hip]),
    })
}
