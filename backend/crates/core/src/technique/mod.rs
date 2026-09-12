//! R-0044 / SPEC-0044 — lift technique analysis over a sampled keypoint series.
//!
//! The "video converted to its basics": a [`LiftSeries`] of per-frame COCO-17
//! [`PoseKeypoints`](crate::pose::PoseKeypoints) in, a typed [`AnalysisOutcome`]
//! out. Pure — no I/O, no model, no clock, no randomness. The ML answers *where
//! are the joints*; everything after that is deterministic trigonometry, which
//! is testable, explainable, and fixable without retraining (AC15).
//!
//! ## Honesty is the design (AC12, `AC10b`, `AC10c`)
//!
//! - **Refusal precedes verdict.** [`analyze`] runs eight gates before it
//!   measures anything, and each returns a [`Refusal`] carrying its evidence.
//!   A confident wrong verdict on technique causes the injury the feature
//!   exists to prevent.
//! - **Absence is reported, never fabricated.** A quantity that cannot be
//!   produced becomes [`Measurement::Unavailable`] with a typed reason, not a
//!   silent omission and not a fabricated `0.0`.
//! - **What COCO-17 cannot see is stated in the output.** [`NOT_MEASURABLE`]
//!   travels with every analysis so the UI cannot drop it. **Back rounding is
//!   not detectable** by this model; torso inclination is not a spine check.
//! - **Uncertainty, not confidence.** Every [`Finding`] carries an uncertainty
//!   in its own unit, and [`FindingSeverity::Borderline`] means the interval
//!   *straddles* the threshold — not "close to it".
//!
//! ## Coordinate space (SPEC-0044 §2.2.2) — load-bearing
//!
//! Keypoints live in the model's **letterboxed** canvas: one uniform scale plus
//! a translation from source pixels. That space is *isotropic*, so angles and
//! length ratios are correct computed directly in it, with no aspect
//! correction. Every angle and every normalized length below depends on it.
//!
//! ## Sign convention, stated once
//!
//! Image `y` grows **downward**. Every signal in this module works on
//! `height = −y`, so "up" means increasing. Every claim depends on it.
//!
//! ## Which series each stage reads (architect review finding 5)
//!
//! | Stage | Reads |
//! |---|---|
//! | frame count, timestamps | `t_ms` metadata only |
//! | framing coverage, mean confidence, near-side choice | raw per-keypoint scores |
//! | quiet stance, standing reference | **smoothed** height + horizontal signals |
//! | view classification | **raw** keypoints (the cues are within-frame ratios; the median over the quiet window does the averaging) |
//! | camera-static check | **smoothed** landmark position (camera translation is low-frequency; keypoint noise is not camera motion) |
//! | rep segmentation, extrema, bar break | **smoothed** signal |
//! | every reported measurement | **raw** keypoints at the chosen frame |
//!
//! The last row is the one that matters most: an index is chosen on the
//! smoothed signal because *phase* must not shift, but the geometry at that
//! instant is read raw, because smoothing across ±0.2 s at a turning point
//! drags every extreme value toward the mean — for squat depth, in the
//! flattering direction.
//!
//! ## Owner decisions applied over the spec text
//!
//! 1. **Only squat depth ships a threshold.** Depth has an objective standard
//!    (hip crease level with the top of the knee), so [`FindingSeverity`] is
//!    reachable for it — including `Borderline`, by the straddling-interval
//!    rule. Every other metric reports a measured value with `severity: None`
//!    until a citable threshold exists (SPEC-0044 §2.14's contract for a
//!    severity-less finding still governs those).
//! 2. **AC7 is the seam only.** [`analyze`] takes `Option<&Calibration>`.
//!    Absent it, [`Normalizers::population`] supplies the population priors
//!    *and* a [`POPULATION_RATIO_SPREAD`] term that widens every reported
//!    uncertainty; [`Normalizers::from_calibration`] drops that term. Absence
//!    changes the numbers and the interval, never a code path.

#[cfg(test)]
mod tests;

mod bench;
mod deadlift;
mod geometry;
mod metrics;
mod segment;
mod squat;
mod view;

use serde::{Deserialize, Serialize};

use crate::periodize::lift_key;
use crate::pose::{Landmark, PoseKeypoints};

pub use geometry::{Calibration, Normalizers, Segments};
pub use metrics::Metric;

// ---------------------------------------------------------------------------
// Bounds (SPEC-0044 §2.3)
//
// The whole table lives here so a future edit cannot change one number without
// seeing the others, and so the §2.3.1 invariant below can be asserted at
// compile time. `MAX_FRAME_PIXELS` and `MAX_CLIP_MS` are declared here and
// *enforced at the api edge*, which is the only layer that sees encoded bytes
// and has a request to reject; splitting the table across crates to reflect
// that would cost more than it explains. Transport-only bounds (`BODY_LIMIT`,
// `MAX_CONCURRENT_ANALYSES`, `ANALYSIS_TIMEOUT`) are not domain values and stay
// with the edge entirely.
// ---------------------------------------------------------------------------

/// The rate a client samples frames at. A rep lasts 2–4 s; 10 Hz resolves the
/// turnaround. Too coarse for double differentiation — see the note on
/// [`RepTempo`].
pub const SAMPLE_HZ: u32 = 10;

/// 40 s at [`SAMPLE_HZ`]. Each frame is one inference; this is the expansion an
/// attacker controls.
pub const MAX_FRAMES: usize = 400;

/// 2 s — below this no rep can be segmented.
pub const MIN_FRAMES: usize = 20;

/// ~5× the expected 480 px JPEG. A pathology bound, not a target.
pub const MAX_FRAME_BYTES: usize = 256 * 1024;

/// Request-level ceiling: [`MAX_FRAMES`] at a generous 80 KiB average.
pub const MAX_TOTAL_BYTES: usize = 32 * 1024 * 1024;

/// Decoded `width × height` ceiling — 4 Mi pixels is 12 MiB of RGB8. A byte bound
/// does not bound a decode.
pub const MAX_FRAME_PIXELS: u64 = 4 * 1024 * 1024;

/// `t_ms` span ceiling, with slack over `MAX_FRAMES / SAMPLE_HZ`.
pub const MAX_CLIP_MS: u32 = 45_000;

// The §2.3.1 invariant, pinned where it cannot be skipped: the shortest legal
// request at the largest legal frame must fit under the total cap, and the
// total cap must be reachable. A previous draft failed the left-hand side.
const _: () = assert!(MAX_FRAME_BYTES * MIN_FRAMES <= MAX_TOTAL_BYTES);
const _: () = assert!(MAX_TOTAL_BYTES <= MAX_FRAME_BYTES * MAX_FRAMES);

// ---------------------------------------------------------------------------
// Gate constants (SPEC-0044 §2.2.1, §2.4, §2.5, §2.6)
//
// These are **initial values with stated derivations, not measurements**
// (SPEC-0044 §5). Unlike a severity threshold, a wrong value here produces a
// *refusal* rather than a wrong verdict — the safe direction.
// ---------------------------------------------------------------------------

/// The median inter-frame gap may differ from `1000 / sample_hz` by this
/// fraction before the series is refused as irregularly sampled.
pub const MEDIAN_GAP_TOLERANCE: f64 = 0.25;

/// No single gap may exceed this multiple of the median gap.
pub const MAX_GAP_FACTOR: f64 = 2.0;

/// A lift's load-bearing landmarks must clear the confidence floor in at least
/// this fraction of frames, or the lifter was out of frame.
pub const MIN_FRAME_COVERAGE: f64 = 0.90;

/// Mean keypoint score over *this lift's* load-bearing joints must reach this.
pub const MIN_MEAN_CONFIDENCE: f64 = 0.5;

/// `shoulder_span / torso_length` at or below this reads as a side view
/// (nominal `S/T ≈ 0.85` ⇒ yaw ≳ 69°).
pub const VIEW_SIDE_MAX: f64 = 0.30;

/// …and at or above this, a front view (⇒ yaw ≲ 28°). Everything between is
/// `Indeterminate` and refused: `S/T` varies between people by roughly
/// [`POPULATION_RATIO_SPREAD`], so a narrow band would trade a real refusal for
/// a person-dependent guess.
pub const VIEW_FRONT_MIN: f64 = 0.75;

/// Corroborating cosine cue: `hip_span / torso_length`, side ceiling.
pub const HIP_VIEW_SIDE_MAX: f64 = 0.22;

/// Corroborating cosine cue: `hip_span / torso_length`, front floor.
pub const HIP_VIEW_FRONT_MIN: f64 = 0.55;

/// Orthogonal cue: `min(ear score) / max(ear score)`, side ceiling. Side-on the
/// far ear is behind the head and scores near zero.
pub const EAR_RATIO_SIDE_MAX: f64 = 0.35;

/// Orthogonal cue: `min(ear score) / max(ear score)`, front floor.
pub const EAR_RATIO_FRONT_MIN: f64 = 0.70;

/// Orthogonal cue: `|nose.x − shoulder_mid.x| / torso_length`, side floor.
pub const NOSE_OFFSET_SIDE_MIN: f64 = 0.12;

/// Orthogonal cue: `|nose.x − shoulder_mid.x| / torso_length`, front ceiling.
pub const NOSE_OFFSET_FRONT_MAX: f64 = 0.06;

/// More than this fraction of frames disagreeing with the median view
/// classification is [`Refusal::UnstableView`] — a panned camera or a rotating
/// lifter, either way no single view the geometry may assume.
pub const MAX_UNSTABLE_FRACTION: f64 = 0.10;

/// Full width of the centred moving average, in frames.
///
/// **Pinned, not derived** (architect review finding 20). SPEC-0044 §2.6.2
/// wrote the half-width as `round(SMOOTH_SECONDS × SAMPLE_HZ / 2)` and
/// annotated it "2 at 10 Hz"; Rust's `f64::round` is half-away-from-zero, so
/// `round(2.5) == 3.0` and the formula yields a 7-frame window, not the 5-frame
/// one the spec intended. The intent — 0.5 s at [`SAMPLE_HZ`] — is pinned here
/// as the odd full width, and the half-width is derived from it.
pub const SMOOTH_WINDOW_FRAMES: usize = 5;

/// Consecutive frames that must be still to count as a quiet stance — 0.5 s at
/// [`SAMPLE_HZ`].
pub const QUIET_WINDOW_FRAMES: usize = 5;

/// A quiet window's signal range, as a fraction of the normalizer.
///
/// **Re-derived** (architect review finding 5). SPEC-0044 §2.6.2 used 0.08 with
/// the reasoning that a 5-frame range of a ±4 % process "runs to ~8 %" — but the
/// *expected* range of five draws from a σ = 4 % process is ≈ 2.33 σ ≈ 9.3 %, so
/// 0.08 sits below the mean of the very distribution it must clear and fails on
/// genuinely still lifters. 0.15 clears it with headroom while staying far below
/// a walkout step (~0.3 m ≈ 60 % of a torso length). It is deliberately sized
/// against the *unsmoothed* noise, so it stays valid whichever way the
/// smoothing choice is later revisited.
pub const QUIET_TOL: f64 = 0.15;

/// Topographic prominence a turning point must have to be a rep, as a fraction
/// of torso length. Well above the noise floor, well below a real rep's ~0.75.
pub const MIN_REP_PROMINENCE: f64 = 0.25;

/// A rep's extremum must also depart its own reference by this fraction of
/// torso length, in the lift's own direction. A floor, not a technique
/// threshold.
pub const MIN_REP_EXCURSION: f64 = 0.20;

/// Standard deviation (not range — one bad frame must not condemn a clip) of
/// the landmarks that must be still, over the segmentation window.
pub const MAX_STATIC_DRIFT: f64 = 0.15;

/// Nominal keypoint position scale, as a fraction of torso length, at a score
/// of 1.0.
///
/// **A modelling choice, not a published figure** (architect review finding
/// 11): `MoveNet` reports no per-keypoint error, and quoting one would be exactly
/// the invented number AC12 forbids. It is an initial value pending the
/// labelled sample SPEC-0044 §5 calls for, and it is *inflated per landmark* in
/// inverse proportion to that landmark's own score — a point the model is unsure
/// of contributes more uncertainty than a point it is sure of.
pub const KEYPOINT_SIGMA_FRACTION: f64 = 0.04;

/// Relative uncertainty carried by standing in for the lifter's own segment
/// ratios with population means (AC7).
///
/// The residual out-of-plane yaw is recoverable only as
/// `view_ratio / (shoulder_span / torso_length)`, so an unknown `S/T` — which
/// varies between people by roughly this fraction — leaves the foreshortening
/// factor uncertain by the same *relative* amount, and every in-plane
/// measurement inherits it proportionally. This is the term
/// [`Normalizers::from_calibration`] drops: it is what supplying the lifter's
/// own ratios actually buys.
pub const POPULATION_RATIO_SPREAD: f64 = 0.15;

/// Bound on the hip crease's offset from the hip **joint centre**, as a
/// fraction of thigh length.
///
/// `MoveNet` reports the joint centre; the depth standard is the crease. The
/// **sign and magnitude of the resulting angular bias are unestablished** — at
/// the extremum the thigh is near horizontal and the offset is close to
/// parallel with it, which contributes little angular error, and the residual
/// sign is not known. It is therefore carried as a *bound* on the uncertainty
/// interval rather than as a correction, pending the per-person calibration of
/// AC7 or a labelled validation set.
pub const HIP_CREASE_OFFSET_THIGH_FRACTION: f64 = 0.10;

/// Rise of the bar proxy that counts as the bar breaking the floor, as a
/// fraction of torso length.
///
/// **Sized at ≥ 3 σ of the smoothed signal** (architect review finding 5).
/// One smoothed sample of a σ = [`KEYPOINT_SIGMA_FRACTION`] process over
/// [`SMOOTH_WINDOW_FRAMES`] has σ ≈ 0.04/√5 ≈ 0.018 torso lengths, so 3 σ ≈
/// 0.054; 0.06 clears it. Expressed in torso lengths rather than SPEC-0044
/// §2.7.3's shank lengths so the derivation needs no shank/torso conversion —
/// the σ it must clear is itself defined in torso lengths.
pub const BAR_BREAK_EPSILON: f64 = 0.06;

/// The `AC10b` copy, carried with every analysis so the UI cannot omit it
/// (SPEC-0044 §2.7.5).
pub const NOT_MEASURABLE: &[&str] = &[
    "Back rounding is not detectable. This pose model has no spine landmark, so \
     a neutral spine and a rounded spine at the same hip angle look identical. \
     Torso angle change is not a back check and must not be read as one.",
    "Mid-foot position is not measurable. The pose model ends at the ankle — \
     there is no heel, toe, or foot — so bar drift is reported from the ankle, \
     which sits behind the mid-foot and therefore reads slightly large.",
    "Elbow flare is not measurable from a side view. Flare is a front-plane \
     angle; from a camera perpendicular to the bench the upper arm points along \
     the lens axis and the far arm is hidden.",
    "Bar position, grip width, stance width and foot rotation are not measured. \
     There are no foot landmarks and the bar itself is never detected — the \
     wrist stands in for it.",
    "Head and neck position are not reported. A neck angle derived from the \
     nose, eyes and ears is dominated by their own noise.",
    "These numbers assume a conventional deadlift, a back squat and a flat \
     bench. Sumo, front squat and their kin change the mechanics enough that \
     they are not comparable.",
];

// ---------------------------------------------------------------------------
// The vector structure (SPEC-0044 §2.2, AC5)
// ---------------------------------------------------------------------------

/// One sampled frame: when it was taken relative to clip start, and where the
/// joints were.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoseFrame {
    /// Milliseconds since clip start. `frames[0].t_ms == 0` always.
    pub t_ms: u32,
    /// The frame's COCO-17 pose, in the isotropic canvas space of §2.2.2.
    pub pose: PoseKeypoints,
}

/// A lift's full sampled series — the "video converted to its basics".
///
/// This is a **stable on-disk format** (SPEC-0044 §2.10.2). Within a
/// `schema_version`, changes are additive only; any change to the meaning,
/// unit, coordinate space or name of an existing field bumps the version.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiftSeries {
    /// Format version. `1` today.
    pub schema_version: u16,
    /// Which lift the clip shows.
    pub lift: Lift,
    /// The view the client was told to film from, and which the analysis will
    /// verify before assuming it.
    pub view: CameraView,
    /// Source frame width in pixels — stored so a consumer can invert the
    /// letterbox back to source pixels.
    pub frame_width: u32,
    /// Source frame height in pixels.
    pub frame_height: u32,
    /// The nominal rate the series was captured at.
    pub sample_hz: u32,
    /// The sampled frames, in capture order.
    pub frames: Vec<PoseFrame>,
}

/// The three lifts v1 analyses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lift {
    /// Back squat.
    Squat,
    /// Flat barbell bench press.
    Bench,
    /// Conventional deadlift.
    Deadlift,
}

/// Where the phone was (AC4). Always restated in the output (AC13).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraView {
    /// Perpendicular to the bar — the sagittal plane is in the image.
    Side,
    /// Straight on — the frontal plane is in the image.
    Front,
}

/// Which side of the body a per-side value belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// The lifter's left.
    Left,
    /// The lifter's right.
    Right,
}

impl Side {
    /// The other side.
    #[must_use]
    pub fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

impl Lift {
    /// The [`lift_key`]-normalized form of this lift's canonical name — the
    /// bridge to the workout log's free-text exercise names (SPEC-0044 §2.10.5).
    #[must_use]
    pub fn canonical_key(self) -> &'static str {
        match self {
            Lift::Squat => "squat",
            Lift::Bench => "bench press",
            Lift::Deadlift => "deadlift",
        }
    }

    /// Free-text exercise names that resolve to this lift, already in
    /// [`lift_key`] form. Includes Spanish, for the target market.
    ///
    /// `sumo deadlift` is **deliberately absent**: its mechanics differ enough
    /// that bar drift and torso angle are not comparable to a conventional
    /// pull, and matching it would be a silent substitution.
    #[must_use]
    pub fn aliases(self) -> &'static [&'static str] {
        match self {
            Lift::Squat => &[
                "squat",
                "back squat",
                "barbell squat",
                "high bar squat",
                "low bar squat",
                "sentadilla",
            ],
            Lift::Bench => &[
                "bench press",
                "bench",
                "barbell bench press",
                "flat bench press",
                "press de banca",
            ],
            Lift::Deadlift => &[
                "deadlift",
                "conventional deadlift",
                "barbell deadlift",
                "peso muerto",
            ],
        }
    }

    /// Resolve a logged exercise name to a lift — **exact lookup after
    /// [`lift_key`] normalization**, never fuzzy matching. `None` means "no
    /// logged load found", which the consumer is expected to handle.
    #[must_use]
    pub fn from_exercise_name(name: &str) -> Option<Self> {
        let key = lift_key(name);
        [Lift::Squat, Lift::Bench, Lift::Deadlift]
            .into_iter()
            .find(|lift| lift.aliases().contains(&key.as_str()))
    }

    /// The landmarks this lift's measurements rest on, for the framing and
    /// confidence gates. Bilateral entries are resolved to `side` for a
    /// [`CameraView::Side`] clip and taken on both sides for a front view.
    fn load_bearing(self, view: CameraView) -> &'static [Bilateral] {
        match (self, view) {
            (Lift::Squat, CameraView::Side) => &[
                Bilateral::Shoulder,
                Bilateral::Hip,
                Bilateral::Knee,
                Bilateral::Ankle,
            ],
            (Lift::Squat, CameraView::Front) => {
                &[Bilateral::Hip, Bilateral::Knee, Bilateral::Ankle]
            }
            (Lift::Bench, _) => &[
                Bilateral::Shoulder,
                Bilateral::Elbow,
                Bilateral::Wrist,
                Bilateral::Hip,
            ],
            (Lift::Deadlift, _) => &[
                Bilateral::Shoulder,
                Bilateral::Wrist,
                Bilateral::Hip,
                Bilateral::Knee,
                Bilateral::Ankle,
            ],
        }
    }
}

/// A joint that exists on both sides, addressed without committing to one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bilateral {
    Shoulder,
    Elbow,
    Wrist,
    Hip,
    Knee,
    Ankle,
}

impl Bilateral {
    /// The COCO-17 landmark for this joint on a given side.
    pub(crate) fn on(self, side: Side) -> Landmark {
        match (self, side) {
            (Bilateral::Shoulder, Side::Left) => Landmark::LeftShoulder,
            (Bilateral::Shoulder, Side::Right) => Landmark::RightShoulder,
            (Bilateral::Elbow, Side::Left) => Landmark::LeftElbow,
            (Bilateral::Elbow, Side::Right) => Landmark::RightElbow,
            (Bilateral::Wrist, Side::Left) => Landmark::LeftWrist,
            (Bilateral::Wrist, Side::Right) => Landmark::RightWrist,
            (Bilateral::Hip, Side::Left) => Landmark::LeftHip,
            (Bilateral::Hip, Side::Right) => Landmark::RightHip,
            (Bilateral::Knee, Side::Left) => Landmark::LeftKnee,
            (Bilateral::Knee, Side::Right) => Landmark::RightKnee,
            (Bilateral::Ankle, Side::Left) => Landmark::LeftAnkle,
            (Bilateral::Ankle, Side::Right) => Landmark::RightAnkle,
        }
    }
}

// ---------------------------------------------------------------------------
// Reps and tempo (SPEC-0044 §2.6.5, §2.7.4)
// ---------------------------------------------------------------------------

/// One rep, as frame indices into [`LiftSeries::frames`] (SPEC-0044 §2.2.1 —
/// all geometry is in index space; `t_ms` only validates and converts tempo).
///
/// **Construction** (architect review finding 3), from the prominent extrema of
/// the smoothed signal, walked in order:
///
/// - `extremum` is the accepted prominent extremum itself (a minimum for squat
///   and bench, a maximum for the deadlift).
/// - `end` is the opposite turning point that follows it — the argmax (squat,
///   bench) or argmin (deadlift) between this extremum and the next accepted
///   one, or between it and the last frame for the final rep.
/// - `start` is `end` of the **previous** rep, so reps are contiguous and each
///   rep 2..N references the position it actually began from (the touch-and-go
///   rule). For **rep 1** it is the last frame at or before `extremum` whose
///   smoothed signal is still within [`QUIET_TOL`] of the standing reference —
///   "the last frame before the movement started", which for a deadlift is the
///   last setup frame before the bar breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rep {
    /// First frame of the rep.
    pub start: usize,
    /// The turning point — bottom of a squat or bench, lockout of a deadlift.
    pub extremum: usize,
    /// Last frame of the rep.
    pub end: usize,
}

/// Per-rep tempo in seconds — the only place `t_ms` reaches the output.
///
/// Squat and bench are eccentric-first; the deadlift's pull is concentric-first
/// and its eccentric may simply not exist because the bar was dropped. `None`
/// says so; a fabricated `0.0` would not.
///
/// > **Note for consumers doing mechanics on this series:** [`SAMPLE_HZ`] is
/// > too coarse for double differentiation. A squat turnaround lasts ~200 ms —
/// > two samples — and the second-difference noise `√6·σ/Δt²` exceeds the true
/// > turnaround acceleration. Smoothing cannot rescue it, because the window is
/// > wider than the event. Accept a quasi-static model, or raise the sample
/// > rate and everything that follows from it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RepTempo {
    /// Squat/bench: start → extremum. Deadlift: lockout → the rep's end.
    /// `None` when the phase is not in the clip.
    pub eccentric_s: Option<f64>,
    /// Squat/bench: extremum → end. Deadlift: start → lockout.
    pub concentric_s: f64,
}

// ---------------------------------------------------------------------------
// Findings (SPEC-0044 §2.8, AC10c/AC11/AC13)
// ---------------------------------------------------------------------------

/// The unit a [`Finding::value`] and its [`Finding::uncertainty`] are in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    /// Degrees of arc.
    Degrees,
    /// A length divided by one of the lifter's own segment lengths.
    NormalizedLength,
    /// A dimensionless ratio of two like quantities.
    Ratio,
}

/// Where a measured value sits relative to its threshold — **only** where a
/// citable threshold exists (owner decision: squat depth in v1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    /// The whole uncertainty interval is on the pass side.
    Ok,
    /// The interval **straddles** the threshold (`AC10c`) — not "close to it".
    /// A straddling interval is a statement about what the measurement can
    /// support; "close to" would be a second arbitrary constant on top of the
    /// first.
    Borderline,
    /// The whole interval is on the fail side.
    Flagged,
}

impl FindingSeverity {
    /// Classify a value whose **pass side is at or below** `threshold`, by the
    /// `AC10c` straddling-interval rule.
    ///
    /// Not "close to the threshold": that would be a second arbitrary constant
    /// layered on the first. A straddling interval is a statement about what
    /// the measurement can actually support, which is why a metric with a
    /// systematic bias comparable to its decision margin reads `Borderline`
    /// most of the time — and why that is the honest answer rather than a bug.
    #[must_use]
    pub fn below_threshold(value: f64, uncertainty: f64, threshold: f64) -> Self {
        let half = uncertainty.abs();
        if value + half <= threshold {
            FindingSeverity::Ok
        } else if value - half > threshold {
            FindingSeverity::Flagged
        } else {
            FindingSeverity::Borderline
        }
    }
}

/// One measured quantity, with everything needed to read it honestly (AC11).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// What was measured.
    pub metric: Metric,
    /// Which rep, 1-indexed. `None` for a statistic over the whole set.
    pub rep: Option<u32>,
    /// The frame the value was read at, when it is read at one instant.
    pub frame: Option<u32>,
    /// Which side, for per-side metrics.
    pub side: Option<Side>,
    /// The number.
    pub value: f64,
    /// The unit of `value` **and** of `uncertainty`.
    pub unit: Unit,
    /// Half-width of the uncertainty interval, in the same unit as `value` —
    /// not a percentage.
    pub uncertainty: f64,
    /// `None` = no accepted threshold exists for this metric yet. The UI
    /// renders such a value neutrally: no colour, no icon, no pass/fail word,
    /// and it counts toward no summary badge (SPEC-0044 §2.14).
    pub severity: Option<FindingSeverity>,
    /// Mean keypoint score of the landmarks the value rests on.
    pub confidence: f64,
}

/// Why a measurement could not be produced. Reported, never omitted and never
/// fabricated (`AC10b` at the level of a single measurement).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unmeasurable {
    /// Touch-point consistency needs at least two reps; one rep would yield a
    /// variance of 0.0, which reads as perfect consistency.
    SingleRep,
    /// The hip-rise ratio has no denominator — the bar never left the floor
    /// within the rep.
    BarDidNotBreak,
    /// The landmarks this value rests on are below the confidence floor.
    LandmarkNotConfident,
    /// This view cannot see the quantity — e.g. depth from a front clip.
    NotThisView,
}

/// A measurement slot: either a value, or the reason there is none.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Measurement {
    /// The quantity was measured.
    Measured(Finding),
    /// The quantity could not be measured, and here is why.
    Unavailable {
        /// What could not be measured.
        metric: Metric,
        /// Which rep, 1-indexed; `None` when the reason is not per-rep.
        rep: Option<u32>,
        /// Why not.
        reason: Unmeasurable,
    },
}

/// The analysis of one clip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiftAnalysis {
    /// The view the geometry assumed — always stated (AC4/AC13).
    pub view: CameraView,
    /// How many reps were segmented.
    pub rep_count: u32,
    /// The rep boundaries, as frame indices. Exposed so a consumer can slice
    /// the same series without re-running segmentation.
    pub reps: Vec<Rep>,
    /// One entry per rep.
    pub tempo: Vec<RepTempo>,
    /// Every metric this lift and view can speak to — measured or explicitly
    /// unavailable.
    pub measurements: Vec<Measurement>,
    /// The `AC10b` copy, carried with the result so the UI cannot omit it.
    pub not_measurable: Vec<String>,
}

/// Why the analysis declined to score (AC12). Every variant carries its
/// evidence, so the user is told *why*, with numbers.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "reason")]
pub enum Refusal {
    /// Fewer frames than a rep can be segmented from. Unreachable over HTTP —
    /// the api edge rejects an under-[`MIN_FRAMES`] upload with a 4xx before a
    /// series is built — but [`analyze`] must be **total**, so a direct call
    /// with a 3-frame series gets a typed answer rather than a panic.
    TooFewFrames {
        /// Frames supplied.
        have: usize,
        /// Frames needed.
        required: usize,
    },
    /// The clip was not sampled at the rate it claims: too slow, too fast, or
    /// jittery. The three fields say which.
    IrregularSampling {
        /// Observed median inter-frame gap.
        median_gap_ms: f64,
        /// The gap `sample_hz` implies.
        expected_gap_ms: f64,
        /// The largest single gap observed.
        max_gap_ms: u32,
    },
    /// A landmark the lift needs was missing or unconfident in too many frames.
    OutOfFrame {
        /// The worst landmark.
        landmark: Landmark,
        /// Frames in which it was below the floor.
        missing_frames: u32,
        /// Frames in the series.
        total_frames: u32,
    },
    /// Mean keypoint confidence over this lift's load-bearing joints was too
    /// low to classify a view or fit geometry to.
    LowConfidence {
        /// Observed mean.
        mean: f64,
        /// Floor.
        required: f64,
    },
    /// The camera was not where the lift needs it.
    WrongView {
        /// The view the client declared.
        expected: CameraView,
        /// What the cues actually read as.
        looks_like: ViewClass,
        /// `shoulder_span / torso_length`, median over the quiet window.
        shoulder_ratio: f64,
        /// `hip_span / torso_length`.
        hip_ratio: f64,
        /// `min(ear score) / max(ear score)`.
        ear_score_ratio: f64,
    },
    /// The view changed during the clip — a panned camera or a rotating lifter.
    UnstableView {
        /// Frames classified side-on.
        side: u32,
        /// Frames classified front-on.
        front: u32,
        /// Frames that matched neither.
        indeterminate: u32,
    },
    /// **The camera or the lifter's feet moved during the set.** Which of the
    /// two is not asserted — asserting would be a guess.
    CameraMoved {
        /// The landmark that drifted.
        landmark: Landmark,
        /// Its standard deviation over the segmentation window, normalized.
        drift: f64,
        /// [`MAX_STATIC_DRIFT`].
        allowed: f64,
    },
    /// No stretch of the clip was still enough to take a reference from — the
    /// lifter was never still, or the clip starts mid-set.
    NoStableStart,
    /// Nothing in the clip after the reference looked like a rep.
    NoRepsDetected,
}

/// What the view cues read as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewClass {
    /// Side-on.
    Side,
    /// Front-on.
    Front,
    /// Neither, within the deliberately wide refusal band.
    Indeterminate,
}

/// The single persisted and wire representation of an analysis.
///
/// A tagged enum rather than a serialized `Result`: serde encodes `Result` as
/// `{"Ok": …}` / `{"Err": …}`, leaking a Rust type name into a stable on-disk
/// format and onto a Dart client's wire, where it means nothing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum AnalysisOutcome {
    /// The analysis ran and produced measurements.
    Analyzed(LiftAnalysis),
    /// The analysis ran and honestly declined to score.
    Refused(Refusal),
}

// ---------------------------------------------------------------------------
// The pipeline (SPEC-0044 §2.4, §3)
// ---------------------------------------------------------------------------

/// Analyse a sampled keypoint series (AC15).
///
/// Pure, total and deterministic: no I/O, no model, no clock, no randomness,
/// and no input produces a panic. `calibration` is [`None`] until the capture
/// flow ships; its absence widens every uncertainty and changes no code path
/// (AC7).
///
/// Refusal precedes verdict — the eight gates of SPEC-0044 §2.4 run in order,
/// each gated on the last, and **no measurement is computed until all eight
/// pass** (AC12).
#[must_use]
pub fn analyze(series: &LiftSeries, calibration: Option<&Calibration>) -> AnalysisOutcome {
    match run(series, calibration) {
        Ok(analysis) => AnalysisOutcome::Analyzed(analysis),
        Err(refusal) => AnalysisOutcome::Refused(refusal),
    }
}

fn run(series: &LiftSeries, calibration: Option<&Calibration>) -> Result<LiftAnalysis, Refusal> {
    check_frame_count(series)?; // step 1
    check_timestamps(series)?; // step 2

    // Near-side resolution is a pure argmax over keypoint scores: it needs no
    // view and cannot fail, so it is available to the framing and confidence
    // gates that follow. (SPEC-0044 §3 sketches it after step 4; moving a
    // total helper earlier changes no behaviour.)
    let side = geometry::near_side(series);

    geometry::check_framing(series, side)?; // step 3
    geometry::check_confidence(series, side)?; // step 4

    // Thresholds are sized against the median segment lengths over the whole
    // clip (robust, and available before a reference frame exists); reported
    // values are normalized at the reference window (SPEC-0044 §2.7.2).
    let scale = geometry::Segments::median(series, side, 0..series.frames.len());

    let signals = segment::Signals::build(series, side, &scale);
    let stance = segment::quiet_stance(series, &signals, &scale)?; // step 5
    view::classify_and_check_stability(series, &stance)?; // step 6
    segment::check_camera_static(series, side, &stance, &scale)?; // step 7
    let reps = segment::reps(series, &signals, &stance, &scale)?; // step 8

    let norms = Normalizers::resolve(
        geometry::Segments::median(series, side, stance.window.clone()),
        calibration,
    );
    let ctx = geometry::Context {
        series,
        side,
        norms,
    };

    let measurements = match series.lift {
        Lift::Squat => squat::measure(&ctx, &reps),
        Lift::Bench => bench::measure(&ctx, &reps),
        Lift::Deadlift => deadlift::measure(&ctx, &reps, &signals, &scale),
    };

    Ok(LiftAnalysis {
        view: series.view,
        rep_count: u32::try_from(reps.len()).unwrap_or(u32::MAX),
        tempo: segment::tempo(series, &signals, &scale, &reps),
        reps,
        measurements,
        not_measurable: NOT_MEASURABLE.iter().map(|s| (*s).to_string()).collect(),
    })
}

/// Step 1 — enough frames for a rep to exist at all.
fn check_frame_count(series: &LiftSeries) -> Result<(), Refusal> {
    if series.frames.len() < MIN_FRAMES {
        return Err(Refusal::TooFewFrames {
            have: series.frames.len(),
            required: MIN_FRAMES,
        });
    }
    Ok(())
}

/// Step 2 — the clip really was sampled at the rate it claims (SPEC-0044
/// §2.2.1).
///
/// Well-formedness of the timestamps (`frames[0].t_ms == 0`, strictly
/// increasing, span under [`MAX_CLIP_MS`]) is a request contract the api edge
/// checks with a 4xx before a series is ever built. Core still has to answer
/// *something* for a malformed series, because [`analyze`] is total; a
/// non-monotonic series produces a zero gap, which trips the same regularity
/// check and yields [`Refusal::IrregularSampling`] rather than a panic.
fn check_timestamps(series: &LiftSeries) -> Result<(), Refusal> {
    let gaps: Vec<u32> = series
        .frames
        .windows(2)
        .map(|w| w[1].t_ms.saturating_sub(w[0].t_ms))
        .collect();
    let Some(&max_gap_ms) = gaps.iter().max() else {
        return Ok(());
    };
    let median_gap_ms = median(&gaps.iter().copied().map(f64::from).collect::<Vec<_>>());
    let expected_gap_ms = if series.sample_hz == 0 {
        f64::INFINITY
    } else {
        1000.0 / f64::from(series.sample_hz)
    };

    let off_rate = !expected_gap_ms.is_finite()
        || (median_gap_ms - expected_gap_ms).abs() > MEDIAN_GAP_TOLERANCE * expected_gap_ms;
    let jittery = f64::from(max_gap_ms) >= MAX_GAP_FACTOR * median_gap_ms;

    if off_rate || jittery {
        return Err(Refusal::IrregularSampling {
            median_gap_ms,
            expected_gap_ms,
            max_gap_ms,
        });
    }
    Ok(())
}

/// Median of a slice, averaging the middle pair when the count is even.
///
/// Returns `0.0` for an empty slice: every caller has already established that
/// the slice is non-empty, and a total helper beats a panic in a module whose
/// whole contract is that it never panics.
pub(crate) fn median(values: &[f64]) -> f64 {
    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    match sorted.len() {
        0 => 0.0,
        n if n % 2 == 1 => sorted[n / 2],
        n => f64::midpoint(sorted[n / 2 - 1], sorted[n / 2]),
    }
}
