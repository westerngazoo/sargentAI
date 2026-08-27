//! Bench press measurements (SPEC-0044 §2.7.3, AC9).
//!
//! The bar itself is never detected — **the wrist stands in for it**, and for a
//! side view that is the *near* wrist, because the far one is behind the
//! lifter's own body.
//!
//! Elbow flare is deliberately absent: it is humeral abduction, a frontal-plane
//! angle, and from a camera perpendicular to the bench the upper arm points
//! along the lens axis while the far arm is occluded. The forearm's angle under
//! the bar is the sagittal-plane equivalent and a real coaching cue.

use crate::pose::Keypoint;

use super::geometry::{self, Context, DEGENERATE_LENGTH};
use super::{Bilateral, Finding, Measurement, Metric, Rep, Unit, Unmeasurable};

pub(crate) fn measure(ctx: &Context<'_>, reps: &[Rep]) -> Vec<Measurement> {
    let mut out = Vec::with_capacity(reps.len() * 2 + 1);
    for (index, rep) in reps.iter().enumerate() {
        let rep_no = u32::try_from(index + 1).unwrap_or(u32::MAX);
        out.push(bar_path(ctx, rep, rep_no));
        out.push(forearm_angle(ctx, rep, rep_no));
    }
    out.push(touch_point_consistency(ctx, reps));
    out
}

/// The bar proxy at one frame.
fn bar(ctx: &Context<'_>, frame: usize) -> Keypoint {
    ctx.joint(frame, Bilateral::Wrist)
}

/// The **touch frame** is `rep.extremum` — the bottom of the rep, where the bar
/// meets the chest (architect review finding 25 asks for this stated once).
fn touch_frame(rep: &Rep) -> usize {
    rep.extremum
}

/// Horizontal excursion of the bar proxy over the rep, normalized by forearm
/// length.
fn bar_path(ctx: &Context<'_>, rep: &Rep, rep_no: u32) -> Measurement {
    let forearm = ctx.norms.segments.forearm;
    if forearm <= DEGENERATE_LENGTH {
        return Measurement::Unavailable {
            metric: Metric::BenchBarPathDeviation,
            rep: Some(rep_no),
            reason: Unmeasurable::LandmarkNotConfident,
        };
    }
    let frames: Vec<Keypoint> = (rep.start..=rep.end.min(ctx.series.frames.len() - 1))
        .map(|f| bar(ctx, f))
        .collect();
    if frames.iter().any(|k| !k.is_confident()) {
        return Measurement::Unavailable {
            metric: Metric::BenchBarPathDeviation,
            rep: Some(rep_no),
            reason: Unmeasurable::LandmarkNotConfident,
        };
    }
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for k in &frames {
        lo = lo.min(geometry::horizontal(*k));
        hi = hi.max(geometry::horizontal(*k));
    }
    let value = (hi - lo) / forearm;
    let sigma = frames.iter().map(|k| ctx.sigma(*k)).fold(0.0, f64::max);
    let random = geometry::offset_uncertainty(sigma, sigma, forearm);

    Measurement::Measured(Finding {
        metric: Metric::BenchBarPathDeviation,
        rep: Some(rep_no),
        frame: None,
        side: None,
        value,
        unit: Unit::NormalizedLength,
        uncertainty: ctx.norms.widen(value, random),
        severity: None,
        confidence: geometry::mean_score(&frames),
    })
}

/// The angle of the wrist→elbow segment from vertical at the touch frame,
/// **positive when the elbow is caudal** — toward the feet — of the wrist.
///
/// The lifter is supine, so "caudal" is read from the body's own long axis:
/// the shoulder is at the head end and the hip at the foot end, and both are
/// required by the bench framing guide.
fn forearm_angle(ctx: &Context<'_>, rep: &Rep, rep_no: u32) -> Measurement {
    let frame = touch_frame(rep);
    let unavailable = Measurement::Unavailable {
        metric: Metric::BenchForearmAngleAtTouch,
        rep: Some(rep_no),
        reason: Unmeasurable::LandmarkNotConfident,
    };
    let forearm = ctx.norms.segments.forearm;
    let wrist = bar(ctx, frame);
    let elbow = ctx.joint(frame, Bilateral::Elbow);
    let shoulder = ctx.joint(frame, Bilateral::Shoulder);
    let hip = ctx.joint(frame, Bilateral::Hip);
    if forearm <= DEGENERATE_LENGTH
        || ![wrist, elbow, shoulder, hip]
            .iter()
            .all(|k| k.is_confident())
    {
        return unavailable;
    }
    let caudal = (geometry::horizontal(hip) - geometry::horizontal(shoulder)).signum();
    if caudal == 0.0 {
        return unavailable;
    }

    let value = geometry::signed_angle_from_vertical_deg(wrist, elbow, caudal);
    let random = geometry::angle_uncertainty_deg(ctx.sigma(wrist), ctx.sigma(elbow), forearm);

    Measurement::Measured(Finding {
        metric: Metric::BenchForearmAngleAtTouch,
        rep: Some(rep_no),
        frame: Some(u32::try_from(frame).unwrap_or(u32::MAX)),
        side: None,
        value,
        unit: Unit::Degrees,
        uncertainty: ctx.norms.widen(value, random),
        severity: None,
        confidence: geometry::mean_score(&[wrist, elbow]),
    })
}

/// Spread of the bar proxy's position at the touch frame across the set,
/// normalized by forearm length.
///
/// **At a single rep this yields no finding at all.** A variance over one
/// sample is zero, and a reported `0.0` reads as perfect consistency — a
/// fabricated compliment on a set that never tested it.
fn touch_point_consistency(ctx: &Context<'_>, reps: &[Rep]) -> Measurement {
    if reps.len() < 2 {
        return Measurement::Unavailable {
            metric: Metric::BenchTouchPointConsistency,
            rep: None,
            reason: Unmeasurable::SingleRep,
        };
    }
    let forearm = ctx.norms.segments.forearm;
    let touches: Vec<Keypoint> = reps.iter().map(|r| bar(ctx, touch_frame(r))).collect();
    if forearm <= DEGENERATE_LENGTH || touches.iter().any(|k| !k.is_confident()) {
        return Measurement::Unavailable {
            metric: Metric::BenchTouchPointConsistency,
            rep: None,
            reason: Unmeasurable::LandmarkNotConfident,
        };
    }

    #[allow(clippy::cast_precision_loss)] // rep counts are small
    let n = touches.len() as f64;
    let xs: Vec<f64> = touches.iter().map(|k| geometry::horizontal(*k)).collect();
    let mean = xs.iter().sum::<f64>() / n;
    let value = (xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n).sqrt() / forearm;

    // The spread is estimated from `n` positions, each uncertain by its own σ,
    // so the estimate inherits that scale reduced by √n.
    let sigma = touches.iter().map(|k| ctx.sigma(*k)).fold(0.0, f64::max);
    let random = geometry::offset_uncertainty(sigma, 0.0, forearm) / n.sqrt();

    Measurement::Measured(Finding {
        metric: Metric::BenchTouchPointConsistency,
        rep: None,
        frame: None,
        side: None,
        value,
        unit: Unit::NormalizedLength,
        uncertainty: ctx.norms.widen(value, random),
        severity: None,
        confidence: geometry::mean_score(&touches),
    })
}
