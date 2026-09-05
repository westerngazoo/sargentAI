//! Quiet stance, centred smoothing, prominence, reps and tempo (SPEC-0044 §2.6).
//!
//! **Sign convention:** image `y` grows downward, so every signal here is
//! `height = −y` and "up" means increasing.

use std::ops::Range;

use crate::pose::Keypoint;

use super::geometry::{self, Segments};
use super::{
    median, Bilateral, CameraView, Lift, LiftSeries, Refusal, Rep, RepTempo, Side,
    KEYPOINT_SIGMA_FRACTION, MAX_STATIC_DRIFT, MIN_REP_EXCURSION, MIN_REP_PROMINENCE, QUIET_TOL,
    QUIET_WINDOW_FRAMES, SMOOTH_WINDOW_FRAMES,
};

/// Which way a rep travels from its reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    /// Squat and bench: the rep goes down and comes back.
    Down,
    /// Deadlift: the rep goes up from a low setup.
    Up,
}

/// The per-frame signals segmentation works on.
///
/// All three are **smoothed** (architect review finding 5): a turning point, a
/// stillness test and a bar break are all statements about the low-frequency
/// content of the signal, and keypoint noise is not movement. The *reported*
/// geometry is read raw at the index these choose — see the module docs on
/// [`technique`](super).
pub(crate) struct Signals {
    /// The lift's own vertical signal: hip height for squat and deadlift,
    /// wrist height for bench.
    pub(crate) height: Vec<f64>,
    /// The horizontal signal the quiet-stance test uses: ankle `x` for squat
    /// and deadlift, wrist `x` for bench.
    ///
    /// **Per lift** (architect review finding 2): the bench framing guide does
    /// not require the ankles in frame, so an ankle criterion there would read
    /// a landmark the model hallucinated off the bottom of the picture.
    pub(crate) horizontal: Vec<f64>,
    /// Height of the bar proxy — the wrist.
    pub(crate) bar_height: Vec<f64>,
    /// What `horizontal` is measured against.
    pub(crate) horizontal_norm: f64,
    /// Which way this lift's reps travel.
    pub(crate) direction: Direction,
}

impl Signals {
    pub(crate) fn build(series: &LiftSeries, side: Side, scale: &Segments) -> Self {
        let at = |frame: usize, joint: Bilateral| -> Keypoint {
            geometry::resolve(&series.frames[frame].pose, series.view, side, joint)
        };
        let n = series.frames.len();
        let signal_joint = match series.lift {
            Lift::Squat | Lift::Deadlift => Bilateral::Hip,
            Lift::Bench => Bilateral::Wrist,
        };
        let horizontal_joint = match series.lift {
            Lift::Squat | Lift::Deadlift => Bilateral::Ankle,
            Lift::Bench => Bilateral::Wrist,
        };
        let horizontal_norm = match series.lift {
            Lift::Squat | Lift::Deadlift => scale.shank,
            Lift::Bench => scale.forearm,
        };
        Self {
            height: smooth(&collect(n, |i| geometry::height(at(i, signal_joint)))),
            horizontal: smooth(&collect(n, |i| {
                geometry::horizontal(at(i, horizontal_joint))
            })),
            bar_height: smooth(&collect(n, |i| geometry::height(at(i, Bilateral::Wrist)))),
            horizontal_norm,
            direction: match series.lift {
                Lift::Squat | Lift::Bench => Direction::Down,
                Lift::Deadlift => Direction::Up,
            },
        }
    }
}

fn collect(n: usize, f: impl Fn(usize) -> f64) -> Vec<f64> {
    (0..n).map(f).collect()
}

/// A **centred** moving average over [`SMOOTH_WINDOW_FRAMES`], shrinking at the
/// edges rather than padding.
///
/// Centred, not trailing: a trailing average lags by the half-width, which
/// shifts every detected turning point later by a fixed amount. That lag
/// cancels in a *duration* but not in an *index*, and the bottom frame and the
/// touch frame are exactly where the measurements are read. Padding at the
/// edges would invent data at the frames where the lifter is standing still,
/// biasing the standing reference.
pub(crate) fn smooth(signal: &[f64]) -> Vec<f64> {
    let half = SMOOTH_WINDOW_FRAMES / 2;
    (0..signal.len())
        .map(|i| {
            let lo = i.saturating_sub(half);
            let hi = (i + half + 1).min(signal.len());
            let window = &signal[lo..hi];
            #[allow(clippy::cast_precision_loss)] // window is SMOOTH_WINDOW_FRAMES at most
            {
                window.iter().sum::<f64>() / window.len() as f64
            }
        })
        .collect()
}

/// The quiet window and the reference level taken from it.
pub(crate) struct Stance {
    /// Frame range of the window.
    pub(crate) window: Range<usize>,
    /// The reference level: the median of the lift's signal over the window.
    pub(crate) reference: f64,
}

/// Step 5 — find the stretch the whole analysis is referenced against
/// (SPEC-0044 §2.6.1).
///
/// A squat clip begins with a **walkout**, a bench clip with a **rack-out**, and
/// a deadlift clip with the lifter walking up and bending down. All three
/// corrupt a reference taken from the start of the clip.
///
/// **The window chosen is the LAST quiet window before the first excursion in
/// the lift's own direction** (architect review finding 1). Taking the *first*
/// takes the lifter standing before they approach the bar: for a squat that
/// puts the walkout inside the segmentation window, so the camera-static check
/// fires on every clip; for a deadlift it puts the reference at standing height,
/// from which the pull can never rise a rep's worth, so every deadlift refuses
/// with `NoRepsDetected`.
///
/// The "first excursion" is found without a reference, so the definition is not
/// circular: it is the first frame that has departed **every preceding frame**
/// by a rep's worth, in the lift's own direction — a running maximum for a
/// descending lift, a running minimum for an ascending one. A walkout is
/// lateral, so it never trips it; a descent or a pull always does.
pub(crate) fn quiet_stance(
    series: &LiftSeries,
    signals: &Signals,
    scale: &Segments,
) -> Result<Stance, Refusal> {
    let n = series.frames.len();
    if n < QUIET_WINDOW_FRAMES || scale.torso <= geometry::DEGENERATE_LENGTH {
        return Err(Refusal::NoStableStart);
    }
    let height_tol = QUIET_TOL * scale.torso;
    let horizontal_tol = QUIET_TOL * signals.horizontal_norm;

    let cutoff = first_excursion(
        &signals.height,
        signals.direction,
        MIN_REP_EXCURSION * scale.torso,
    )
    .unwrap_or(n);

    let last = (0..=n - QUIET_WINDOW_FRAMES)
        .rfind(|&start| {
            let w = start..start + QUIET_WINDOW_FRAMES;
            start + QUIET_WINDOW_FRAMES - 1 <= cutoff
                && range_of(&signals.height[w.clone()]) <= height_tol
                && range_of(&signals.horizontal[w]) <= horizontal_tol
        })
        .ok_or(Refusal::NoStableStart)?;

    let window = last..last + QUIET_WINDOW_FRAMES;
    Ok(Stance {
        reference: median(&signals.height[window.clone()]),
        window,
    })
}

/// The first frame that has departed every preceding frame by `excursion`, in
/// the lift's own direction.
fn first_excursion(signal: &[f64], direction: Direction, excursion: f64) -> Option<usize> {
    let mut extreme = *signal.first()?;
    for (i, &value) in signal.iter().enumerate() {
        match direction {
            Direction::Down => {
                extreme = extreme.max(value);
                if extreme - value >= excursion {
                    return Some(i);
                }
            }
            Direction::Up => {
                extreme = extreme.min(value);
                if value - extreme >= excursion {
                    return Some(i);
                }
            }
        }
    }
    None
}

fn range_of(values: &[f64]) -> f64 {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for &v in values {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if lo.is_finite() && hi.is_finite() {
        hi - lo
    } else {
        0.0
    }
}

/// Step 7 — the camera (and the lifter's feet) stayed put (SPEC-0044 §2.6.4).
///
/// Standard deviation, not range: one bad frame must not condemn a clip. The
/// scatter is two-dimensional, so a vertical pan is caught as well as a
/// horizontal one. It is measured over the **segmentation window only** —
/// checking the whole clip would report a moved camera on every squat walkout.
///
/// **Honest caveat, and it belongs in the user-facing string:** a lifter whose
/// feet genuinely shift trips the same check. The refusal says the camera *or*
/// the feet moved and does not assert which; asserting would be a guess.
pub(crate) fn check_camera_static(
    series: &LiftSeries,
    side: Side,
    stance: &Stance,
    scale: &Segments,
) -> Result<(), Refusal> {
    let (joint, normalizer) = match series.lift {
        Lift::Squat | Lift::Deadlift => (Bilateral::Ankle, scale.shank),
        Lift::Bench => (Bilateral::Hip, scale.torso),
    };
    if normalizer <= geometry::DEGENERATE_LENGTH {
        return Ok(());
    }
    let window = stance.window.end..series.frames.len();
    if window.len() < 2 {
        return Ok(());
    }
    let sides: Vec<Side> = match series.view {
        CameraView::Side => vec![side],
        CameraView::Front => vec![Side::Left, Side::Right],
    };
    for s in sides {
        let landmark = joint.on(s);
        let xs = smooth(&collect_range(&window, |i| {
            f64::from(series.frames[i].pose.get(landmark).x)
        }));
        let ys = smooth(&collect_range(&window, |i| {
            f64::from(series.frames[i].pose.get(landmark).y)
        }));
        let drift = std_dev(&xs).hypot(std_dev(&ys)) / normalizer;
        if drift > MAX_STATIC_DRIFT {
            return Err(Refusal::CameraMoved {
                landmark,
                drift,
                allowed: MAX_STATIC_DRIFT,
            });
        }
    }
    Ok(())
}

fn collect_range(range: &Range<usize>, f: impl Fn(usize) -> f64) -> Vec<f64> {
    range.clone().map(f).collect()
}

fn std_dev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)] // frame counts are bounded by MAX_FRAMES
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n).sqrt()
}

/// Step 8 — split the series into reps (SPEC-0044 §2.6.2–2.6.5).
///
/// Candidate turning points are filtered by **topographic prominence**, not by
/// bare reversal: a paused squat is a flat minimum full of micro-reversals from
/// keypoint noise, and a grinder reverses slightly at its sticking point. Bare
/// reversal counts the first as three reps and the second as two.
///
/// A surviving extremum must then depart **its own reference** by
/// [`MIN_REP_EXCURSION`], which rejects a weight shift while re-racking. Rep 1's
/// reference is the standing reference; every later rep's is the position it
/// actually started from — the touch-and-go rule, since reps 2..N of a
/// touch-and-go set begin wherever the bar was set down.
pub(crate) fn reps(
    series: &LiftSeries,
    signals: &Signals,
    stance: &Stance,
    scale: &Segments,
) -> Result<Vec<Rep>, Refusal> {
    let n = series.frames.len();
    let search = stance.window.end.saturating_sub(1);
    let candidates: Vec<usize> = extrema(&signals.height, signals.direction, search..n)
        .into_iter()
        .filter(|&i| {
            prominence(&signals.height, i, signals.direction, search..n)
                >= MIN_REP_PROMINENCE * scale.torso
        })
        .collect();

    let excursion = MIN_REP_EXCURSION * scale.torso;
    let mut out: Vec<Rep> = Vec::new();
    let mut previous_end = rep_one_start(signals, stance, scale, candidates.first().copied());

    for (k, &extremum) in candidates.iter().enumerate() {
        let limit = candidates.get(k + 1).copied().unwrap_or(n);
        let end = turning_point(
            &signals.height,
            signals.direction.reversed(),
            extremum..limit,
        );
        let departed = match signals.direction {
            Direction::Down => signals.height[previous_end] - signals.height[extremum],
            Direction::Up => signals.height[extremum] - signals.height[previous_end],
        };
        if departed >= excursion {
            out.push(Rep {
                start: previous_end,
                extremum,
                end,
            });
            previous_end = end;
        }
    }

    if out.is_empty() {
        return Err(Refusal::NoRepsDetected);
    }
    Ok(out)
}

impl Direction {
    fn reversed(self) -> Self {
        match self {
            Direction::Down => Direction::Up,
            Direction::Up => Direction::Down,
        }
    }
}

/// Rep 1's start: **the last frame at or before the first extremum whose
/// smoothed signal is still within keypoint noise of the standing reference** —
/// the last frame before the movement began.
///
/// Architect review finding 3 asks for this explicitly for the deadlift, where
/// it is the last setup frame before the bar breaks; the rule is the same for
/// every lift.
///
/// The tolerance is one [`KEYPOINT_SIGMA_FRACTION`], not [`QUIET_TOL`]: the
/// latter bounds the *range* of a five-frame window and is several times
/// larger, so using it here would discard the first fifteen per cent of a
/// torso length of movement — which for a deadlift is precisely the hip rise
/// the measurement exists to catch.
fn rep_one_start(
    signals: &Signals,
    stance: &Stance,
    scale: &Segments,
    first_extremum: Option<usize>,
) -> usize {
    let tolerance = KEYPOINT_SIGMA_FRACTION * scale.torso;
    let limit = first_extremum.unwrap_or(stance.window.end);
    (stance.window.start..=limit.max(stance.window.start))
        .rfind(|&i| {
            i < signals.height.len() && (signals.height[i] - stance.reference).abs() <= tolerance
        })
        .unwrap_or(stance.window.start)
}

/// Local extrema of the signal in the lift's own direction, taking the middle
/// of any plateau so a held bottom yields one index, not a run of them.
fn extrema(signal: &[f64], direction: Direction, range: Range<usize>) -> Vec<usize> {
    let lo = range.start.min(signal.len());
    let hi = range.end.min(signal.len());
    let extreme_than = |a: f64, b: f64| match direction {
        Direction::Down => a <= b,
        Direction::Up => a >= b,
    };
    let mut out = Vec::new();
    let mut i = lo;
    while i < hi {
        let before = i == lo || extreme_than(signal[i], signal[i - 1]);
        if !before {
            i += 1;
            continue;
        }
        let mut j = i;
        while j + 1 < hi && (signal[j + 1] - signal[i]).abs() < f64::EPSILON {
            j += 1;
        }
        let after = j + 1 >= hi || extreme_than(signal[j], signal[j + 1]);
        if after {
            out.push(usize::midpoint(i, j));
        }
        i = j + 1;
    }
    out
}

/// Topographic prominence of an extremum: how far the signal must travel away
/// from it before reaching a more extreme one of the same kind.
///
/// Noise wiggles and sticking-point bumps have near-zero prominence; the
/// genuine bottom of a rep has the full rep excursion.
///
/// **A side that runs to the end of the range without meeting a more extreme
/// sample does not constrain the prominence.** The textbook definition would
/// take the saddle out to the boundary, which reads zero for a rep the clip
/// cut off mid-lockout — a dropped last deadlift would vanish. A side that
/// terminates at a genuinely more extreme sample *is* a constraint, which is
/// what keeps a grinder's sticking-point dip from counting as its own rep. When
/// neither side is constrained the extremum is the global one, and its
/// prominence is the larger of the two one-sided drops.
fn prominence(signal: &[f64], at: usize, direction: Direction, range: Range<usize>) -> f64 {
    let lo = range.start.min(signal.len());
    let hi = range.end.min(signal.len());
    if at >= hi || at < lo {
        return 0.0;
    }
    let value = signal[at];
    let more_extreme = |v: f64| match direction {
        Direction::Down => v < value,
        Direction::Up => v > value,
    };
    // The saddle is the opposite kind of extremum: the highest point between
    // two minima, the lowest point between two maxima.
    let saddle = |acc: f64, v: f64| match direction {
        Direction::Down => acc.max(v),
        Direction::Up => acc.min(v),
    };
    let seed = match direction {
        Direction::Down => f64::NEG_INFINITY,
        Direction::Up => f64::INFINITY,
    };

    let mut left = seed;
    let mut left_constrained = false;
    for i in (lo..at).rev() {
        if more_extreme(signal[i]) {
            left_constrained = true;
            break;
        }
        left = saddle(left, signal[i]);
    }
    let mut right = seed;
    let mut right_constrained = false;
    for &v in &signal[at + 1..hi] {
        if more_extreme(v) {
            right_constrained = true;
            break;
        }
        right = saddle(right, v);
    }

    let drop = |s: f64| {
        if s.is_finite() {
            (s - value).abs()
        } else {
            0.0
        }
    };
    match (left_constrained, right_constrained) {
        (true, true) => drop(left).min(drop(right)),
        (true, false) => drop(left),
        (false, true) => drop(right),
        (false, false) => drop(left).max(drop(right)),
    }
}

/// The most extreme index in a range, in the given direction. Ties take the
/// first, so a held lockout does not drift the boundary.
fn turning_point(signal: &[f64], direction: Direction, range: Range<usize>) -> usize {
    let lo = range.start.min(signal.len().saturating_sub(1));
    let hi = range.end.min(signal.len()).max(lo + 1);
    let mut best = lo;
    for i in lo..hi {
        let better = match direction {
            Direction::Down => signal[i] < signal[best],
            Direction::Up => signal[i] > signal[best],
        };
        if better {
            best = i;
        }
    }
    best
}

/// Per-rep tempo in seconds — the only place `t_ms` reaches the output
/// (SPEC-0044 §2.7.4).
///
/// The deadlift's eccentric is `None` when the rep's return phase is not in the
/// clip: the bar was dropped. A fabricated `0.0` would read as an instantaneous
/// lowering.
pub(crate) fn tempo(
    series: &LiftSeries,
    signals: &Signals,
    scale: &Segments,
    reps: &[Rep],
) -> Vec<RepTempo> {
    let seconds = |a: usize, b: usize| {
        (f64::from(series.frames[b].t_ms) - f64::from(series.frames[a].t_ms)) / 1000.0
    };
    reps.iter()
        .map(|rep| {
            let returned = (signals.height[rep.end] - signals.height[rep.extremum]).abs()
                >= MIN_REP_EXCURSION * scale.torso;
            match signals.direction {
                Direction::Down => RepTempo {
                    eccentric_s: (rep.extremum > rep.start)
                        .then(|| seconds(rep.start, rep.extremum)),
                    concentric_s: seconds(rep.extremum, rep.end),
                },
                Direction::Up => RepTempo {
                    eccentric_s: returned.then(|| seconds(rep.extremum, rep.end)),
                    concentric_s: seconds(rep.start, rep.extremum),
                },
            }
        })
        .collect()
}
