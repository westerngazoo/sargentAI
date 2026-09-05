//! Squat measurements (SPEC-0044 §2.7.3, AC8).
//!
//! Depth and torso inclination need the sagittal plane; knee travel needs the
//! frontal plane. **A clip cannot supply both**, which is why the framing guide
//! asks for two clips rather than a 45° compromise that measures neither — and
//! why the view that cannot see a quantity reports it as unavailable rather
//! than guessing.

use super::geometry::{self, angle_above_horizontal_deg, Context, DEGENERATE_LENGTH};
use super::{
    Bilateral, CameraView, Finding, FindingSeverity, Measurement, Metric, Rep, Side, Unit,
    Unmeasurable,
};

/// Squat depth passes when the hip is at or below the knee, i.e. at or below
/// zero degrees above parallel.
///
/// **The one threshold v1 ships.** Depth has an objective standard — the hip
/// crease level with the top of the knee — so a verdict is possible where for
/// every other metric it would be invented. What the standard does *not* fix is
/// this measurement's relationship to it: the model reports the hip joint
/// centre, not the crease, and that offset is carried in the interval rather
/// than corrected for. A value whose interval straddles zero is `Borderline`,
/// and on a measurement with a bias comparable to the decision margin that is
/// the honest answer, not a number to tune away.
const DEPTH_THRESHOLD_DEG: f64 = 0.0;

pub(crate) fn measure(ctx: &Context<'_>, reps: &[Rep]) -> Vec<Measurement> {
    match ctx.series.view {
        CameraView::Side => sagittal(ctx, reps),
        CameraView::Front => frontal(ctx, reps),
    }
}

/// Side view: depth and torso inclination; knee travel is not visible.
fn sagittal(ctx: &Context<'_>, reps: &[Rep]) -> Vec<Measurement> {
    let mut out = Vec::with_capacity(reps.len() * 2 + 1);
    for (index, rep) in reps.iter().enumerate() {
        let rep_no = u32::try_from(index + 1).unwrap_or(u32::MAX);
        out.push(depth(ctx, rep, rep_no));
        // Through the whole rep: a squat's torso can change on the way down
        // and again on the way up.
        out.push(geometry::torso_angle_change(
            ctx,
            Metric::SquatTorsoAngleChange,
            rep_no,
            rep.start,
            rep.start..=rep.end,
        ));
    }
    out.push(Measurement::Unavailable {
        metric: Metric::KneeTravelInward,
        rep: None,
        reason: Unmeasurable::NotThisView,
    });
    out
}

/// Front view: knee travel, per side; the sagittal quantities are not visible.
fn frontal(ctx: &Context<'_>, reps: &[Rep]) -> Vec<Measurement> {
    let mut out = Vec::with_capacity(reps.len() * 2 + 2);
    for (index, rep) in reps.iter().enumerate() {
        let rep_no = u32::try_from(index + 1).unwrap_or(u32::MAX);
        for side in [Side::Left, Side::Right] {
            out.push(knee_travel(ctx, rep, rep_no, side));
        }
    }
    for metric in [Metric::SquatDepth, Metric::SquatTorsoAngleChange] {
        out.push(Measurement::Unavailable {
            metric,
            rep: None,
            reason: Unmeasurable::NotThisView,
        });
    }
    out
}

/// The signed angle of the hip→knee segment from horizontal at the bottom of
/// the rep, **positive when the hip is above the knee** — so `+8` reads as
/// "hip 8° above parallel", and negative is below.
fn depth(ctx: &Context<'_>, rep: &Rep, rep_no: u32) -> Measurement {
    let hip = ctx.joint(rep.extremum, Bilateral::Hip);
    let knee = ctx.joint(rep.extremum, Bilateral::Knee);
    let thigh = ctx.norms.segments.thigh;
    if thigh <= DEGENERATE_LENGTH || !hip.is_confident() || !knee.is_confident() {
        return Measurement::Unavailable {
            metric: Metric::SquatDepth,
            rep: Some(rep_no),
            reason: Unmeasurable::LandmarkNotConfident,
        };
    }

    let value = angle_above_horizontal_deg(hip, knee);
    let random = geometry::angle_uncertainty_deg(ctx.sigma(hip), ctx.sigma(knee), thigh)
        .hypot(ctx.norms.crease_bound_deg());
    let uncertainty = ctx.norms.widen(value, random);

    Measurement::Measured(Finding {
        metric: Metric::SquatDepth,
        rep: Some(rep_no),
        frame: Some(u32::try_from(rep.extremum).unwrap_or(u32::MAX)),
        side: None,
        value,
        unit: Unit::Degrees,
        uncertainty,
        severity: Some(FindingSeverity::below_threshold(
            value,
            uncertainty,
            DEPTH_THRESHOLD_DEG,
        )),
        confidence: geometry::mean_score(&[hip, knee]),
    })
}

/// The knee's horizontal offset from the ankle→hip line at the bottom of the
/// rep, normalized by shank length, **positive toward the midline**.
///
/// Named for the movement, not the clinic: "valgus" is a diagnosis in a
/// lifter's ears, and the number describes where the knee went (AC14).
fn knee_travel(ctx: &Context<'_>, rep: &Rep, rep_no: u32, side: Side) -> Measurement {
    let unavailable = |reason| Measurement::Unavailable {
        metric: Metric::KneeTravelInward,
        rep: Some(rep_no),
        reason,
    };
    let ankle = ctx.joint_on(rep.extremum, Bilateral::Ankle, side);
    let knee = ctx.joint_on(rep.extremum, Bilateral::Knee, side);
    let hip = ctx.joint_on(rep.extremum, Bilateral::Hip, side);
    let other_ankle = ctx.joint_on(rep.extremum, Bilateral::Ankle, side.other());
    let shank = ctx.norms.segments.shank;

    if shank <= DEGENERATE_LENGTH
        || ![ankle, knee, hip, other_ankle]
            .iter()
            .all(|k| k.is_confident())
    {
        return unavailable(Unmeasurable::LandmarkNotConfident);
    }
    let span = geometry::height(hip) - geometry::height(ankle);
    if span.abs() <= DEGENERATE_LENGTH {
        return unavailable(Unmeasurable::LandmarkNotConfident);
    }

    // Where the ankle→hip line sits at the knee's own height.
    let t = (geometry::height(knee) - geometry::height(ankle)) / span;
    let line_x = (geometry::horizontal(hip) - geometry::horizontal(ankle))
        .mul_add(t, geometry::horizontal(ankle));
    // Inward is toward the other foot.
    let inward = (geometry::horizontal(other_ankle) - geometry::horizontal(ankle)).signum();
    let value = (geometry::horizontal(knee) - line_x) * inward / shank;

    let random = geometry::offset_uncertainty(ctx.sigma(knee), ctx.sigma(hip), shank);
    Measurement::Measured(Finding {
        metric: Metric::KneeTravelInward,
        rep: Some(rep_no),
        frame: Some(u32::try_from(rep.extremum).unwrap_or(u32::MAX)),
        side: Some(side),
        value,
        unit: Unit::NormalizedLength,
        uncertainty: ctx.norms.widen(value, random),
        severity: None,
        confidence: geometry::mean_score(&[ankle, knee, hip]),
    })
}
