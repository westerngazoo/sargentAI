//! The **pure** weighted nearest-neighbor matcher (R-0013, SPEC-0013 §2.3).
//!
//! [`rank`] scores a photo-derived [`FrameFeatures`] against every archetype's
//! [`FrameProfile`] and returns them nearest-first. The numeric V-taper ratio
//! dominates (weight `0.6`); the banded clavicle/limb descriptors contribute
//! (`0.2` each) **only when present** on the query — an absent field is skipped
//! and the remaining weights renormalized, so it never penalizes a match. Pure,
//! total, and deterministic: no model, no I/O, NaN-safe ordering.

use crate::archetype::{Archetype, FrameProfile, LengthBand, WidthBand};
use crate::pose::FrameFeatures;

/// Weight of the numeric shoulder-to-waist term (the dominant, most reliable
/// single-photo signal).
const W_RATIO: f64 = 0.6;
/// Weight of each present banded categorical term.
const W_BAND: f64 = 0.2;

/// One archetype scored against a query, by weighted distance (`0.0..=1.0`,
/// lower is nearer). Borrows the archetype from the `'static` curated library
/// (zero-copy — the library is process-lifetime reference data).
#[derive(Clone, Copy, Debug)]
pub struct RankedMatch {
    pub archetype: &'static Archetype,
    pub distance: f64,
}

/// Rank the archetype `library` against a photo-derived query, nearest-first
/// (R-0013 AC2). Stable: equal-distance archetypes keep their library order.
#[must_use]
pub fn rank(features: &FrameFeatures, library: &'static [Archetype]) -> Vec<RankedMatch> {
    let mut matches: Vec<RankedMatch> = library
        .iter()
        .map(|archetype| RankedMatch {
            archetype,
            distance: distance(features, &archetype.frame_profile),
        })
        .collect();
    // Stable sort + `total_cmp` → deterministic, NaN-safe, library-order ties.
    matches.sort_by(|a, b| a.distance.total_cmp(&b.distance));
    matches
}

/// The renormalized weighted distance from a query to one archetype profile.
fn distance(features: &FrameFeatures, profile: &FrameProfile) -> f64 {
    let mut weighted = W_RATIO * ratio_term(features.shoulder_to_waist, profile.shoulder_to_waist);
    let mut total_weight = W_RATIO;

    if let Some(clavicle) = features.clavicle_width {
        weighted += W_BAND * band_term(width_level(clavicle), width_level(profile.clavicle_width));
        total_weight += W_BAND;
    }
    if let Some(limb) = features.limb_length {
        weighted += W_BAND * band_term(length_level(limb), length_level(profile.limb_length));
        total_weight += W_BAND;
    }

    // `total_weight` is always ≥ `W_RATIO` (the ratio is always present), so this
    // never divides by zero.
    weighted / total_weight
}

/// The numeric term: absolute ratio gap normalized by the `1.0..=2.5` span,
/// clamped to `0.0..=1.0`.
fn ratio_term(query: f64, profile: f64) -> f64 {
    const SPAN: f64 = 2.5 - 1.0;
    ((query - profile).abs() / SPAN).clamp(0.0, 1.0)
}

/// An ordinal band term: band steps apart, normalized by the 2-step maximum →
/// `0.0` / `0.5` / `1.0`.
fn band_term(query: u8, profile: u8) -> f64 {
    f64::from(query.abs_diff(profile)) / 2.0
}

fn width_level(band: WidthBand) -> u8 {
    match band {
        WidthBand::Narrow => 0,
        WidthBand::Average => 1,
        WidthBand::Wide => 2,
    }
}

fn length_level(band: LengthBand) -> u8 {
    match band {
        LengthBand::Short => 0,
        LengthBand::Average => 1,
        LengthBand::Long => 2,
    }
}
