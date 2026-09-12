//! Deadlift measurements (SPEC-0044 §2.7.3, AC10).
//!
//! **Bar drift is measured from the ankle, and that is a disclosed
//! approximation.** COCO-17 ends at the ankle — there is no heel, no toe, no
//! foot — so mid-foot does not exist and neither does a foot-length normalizer.
//! The ankle sits behind the mid-foot, so this measurement reads systematically
//! large. That is the safe direction: it over-flags a coaching cue rather than
//! clearing a real drift. It is still a bias, and it is stated here and in the
//! output.

use crate::pose::Keypoint;

use super::geometry::{self, Context, Segments, DEGENERATE_LENGTH};
use super::segment::Signals;
use super::{Bilateral, Finding, Measurement, Metric, Rep, Unit, Unmeasurable, BAR_BREAK_EPSILON};

pub(crate) fn measure(
    ctx: &Context<'_>,
    reps: &[Rep],
    signals: &Signals,
    scale: &Segments,
) -> Vec<Measurement> {
    let mut out = Vec::with_capacity(reps.len() * 3);
    for (index, rep) in reps.iter().enumerate() {
        let rep_no = u32::try_from(index + 1).unwrap_or(u32::MAX);
        out.push(hip_rise_before_bar(ctx, rep, rep_no, signals, scale));
        // Only the pull: for a deadlift **the setup is the bottom**, so the
        // rejected "setup versus bottom" comparison compared one frame with
        // itself and was identically zero.
        out.push(geometry::torso_angle_change(
            ctx,
            Metric::DeadliftTorsoAngleChange,
            rep_no,
            rep.start,
            rep.start..=rep.extremum,
        ));
        out.push(bar_drift_from_ankle(ctx, rep, rep_no));
    }
    out
}

/// The bar proxy at one frame — the near-side wrist.
fn bar(ctx: &Context<'_>, frame: usize) -> Keypoint {
    ctx.joint(frame, Bilateral::Wrist)
}

/// `Δhip_height / Δbar_height` over the early pull — a **ratio**, not a
/// correlation.
///
/// ≈ 1 is a clean pull, hips and bar rising together; ≫ 1 is the hips leading.
///
/// Correlation was rejected outright: against a **near-constant** bar signal its
/// denominator collapses, so the statistic is undefined *exactly* in the case it
/// exists to detect, and in floating point it returns whatever the noise
/// happens to be. A ratio has one degenerate case — the bar never moving — and
/// that case gets a typed answer instead of a division.
///
/// The window runs from the rep's start to the first frame at which the bar has
/// risen by [`BAR_BREAK_EPSILON`]. Both displacements are read from the
/// **smoothed** signals: the epsilon is deliberately sized just above three
/// standard deviations of that smoothed signal, so measuring the denominator
/// raw would reintroduce the very degeneracy the epsilon exists to exclude.
fn hip_rise_before_bar(
    ctx: &Context<'_>,
    rep: &Rep,
    rep_no: u32,
    signals: &Signals,
    scale: &Segments,
) -> Measurement {
    let no_break = Measurement::Unavailable {
        metric: Metric::DeadliftHipRiseBeforeBar,
        rep: Some(rep_no),
        reason: Unmeasurable::BarDidNotBreak,
    };
    if scale.torso <= DEGENERATE_LENGTH {
        return Measurement::Unavailable {
            metric: Metric::DeadliftHipRiseBeforeBar,
            rep: Some(rep_no),
            reason: Unmeasurable::LandmarkNotConfident,
        };
    }

    let epsilon = BAR_BREAK_EPSILON * scale.torso;
    let bar_at_start = signals.bar_height[rep.start];
    let Some(broke) =
        (rep.start..=rep.extremum).find(|&f| signals.bar_height[f] - bar_at_start >= epsilon)
    else {
        return no_break;
    };

    let bar_rise = signals.bar_height[broke] - bar_at_start;
    let hip_rise = signals.height[broke] - signals.height[rep.start];
    if bar_rise <= DEGENERATE_LENGTH {
        return no_break;
    }

    let value = hip_rise / bar_rise;
    let hip_sigma = ctx.sigma(ctx.joint(broke, Bilateral::Hip));
    let bar_sigma = ctx.sigma(bar(ctx, broke));
    let random = geometry::ratio_uncertainty(
        hip_rise,
        hip_sigma * std::f64::consts::SQRT_2,
        bar_rise,
        bar_sigma * std::f64::consts::SQRT_2,
    );

    Measurement::Measured(Finding {
        metric: Metric::DeadliftHipRiseBeforeBar,
        rep: Some(rep_no),
        frame: Some(u32::try_from(broke).unwrap_or(u32::MAX)),
        side: None,
        value,
        unit: Unit::Ratio,
        uncertainty: ctx.norms.widen(value, random),
        severity: None,
        confidence: geometry::mean_score(&[ctx.joint(broke, Bilateral::Hip), bar(ctx, broke)]),
    })
}

/// The peak horizontal distance of the bar proxy from the near-side ankle over
/// the rep, normalized by **shank length** — see the module note on why the
/// ankle, and why there is no foot-length normalizer to use instead.
fn bar_drift_from_ankle(ctx: &Context<'_>, rep: &Rep, rep_no: u32) -> Measurement {
    let shank = ctx.norms.segments.shank;
    let unavailable = Measurement::Unavailable {
        metric: Metric::DeadliftBarDriftFromAnkle,
        rep: Some(rep_no),
        reason: Unmeasurable::LandmarkNotConfident,
    };
    if shank <= DEGENERATE_LENGTH {
        return unavailable;
    }

    let last = rep.end.min(ctx.series.frames.len() - 1);
    let mut best = (rep.start, 0.0_f64);
    let mut points = Vec::with_capacity((last - rep.start + 1) * 2);
    for frame in rep.start..=last {
        let wrist = bar(ctx, frame);
        let ankle = ctx.joint(frame, Bilateral::Ankle);
        if !wrist.is_confident() || !ankle.is_confident() {
            return unavailable;
        }
        points.push(wrist);
        points.push(ankle);
        let drift = (geometry::horizontal(wrist) - geometry::horizontal(ankle)).abs() / shank;
        if drift > best.1 {
            best = (frame, drift);
        }
    }
    let (frame, value) = best;
    let random = geometry::offset_uncertainty(
        ctx.sigma(bar(ctx, frame)),
        ctx.sigma(ctx.joint(frame, Bilateral::Ankle)),
        shank,
    );

    Measurement::Measured(Finding {
        metric: Metric::DeadliftBarDriftFromAnkle,
        rep: Some(rep_no),
        frame: Some(u32::try_from(frame).unwrap_or(u32::MAX)),
        side: None,
        value,
        unit: Unit::NormalizedLength,
        uncertainty: ctx.norms.widen(value, random),
        severity: None,
        confidence: geometry::mean_score(&points),
    })
}
