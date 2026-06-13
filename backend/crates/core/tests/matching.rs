//! Unit tests for the `fitai_core::matching` domain — the **pure** weighted
//! nearest-neighbor matcher (SPEC-0013 §2.3): `RankedMatch<'a>` and
//! `rank(&FrameFeatures, &[Archetype]) -> Vec<RankedMatch>`.
//!
//! Authored by the qa agent during R-0013 step 3 (test planning), BEFORE the
//! `core::matching` module exists. Pre-implementation red state = compile failure
//! (the module / `RankedMatch` / `rank` are absent, and `core::pose::FrameFeatures`
//! the matcher consumes is also absent). Implementation step 5 makes these green.
//! No model, no DB, no HTTP — `rank` is exercised over the real
//! `core::archetype::library()` with hand-authored `FrameFeatures`.
//!
//! SAC → AC → test traceability (the full table lives in the qa sign-off report):
//! - SAC2 → AC2: `rank` is a documented weighted nearest-neighbor over the
//!   library: a `FrameFeatures` authored near a specific archetype's
//!   `frame_profile` ranks that archetype first; the weights are ratio 0.6 /
//!   clavicle 0.2 / limb 0.2; **absent** banded fields are skipped and the
//!   remaining weights renormalized (an absent field never penalizes); ranking
//!   is deterministic, total (`f64::total_cmp`, NaN-safe — never a panicking
//!   sort), stable (ties broken by library order); `distance`/`score` are
//!   bounded and `score == 1.0 - distance`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Distances round-trip authored f64 ratios through the matcher; epsilon-based
// comparisons are used for the derived numbers and exact equality for the
// score == 1.0 - distance identity.
#![allow(clippy::float_cmp)]
// Test doc comments quote field names and archetype slugs as prose.
#![allow(clippy::doc_markdown)]

use fitai_core::archetype::{library, Archetype, LengthBand, WidthBand};
use fitai_core::matching::{rank, RankedMatch};
use fitai_core::pose::FrameFeatures;

// ===========================================================================
// Builders — a `FrameFeatures` query authored to sit near a chosen archetype.
// The library's most distinctive frame is `classic-aesthetic-taper` (the
// V-taper: shoulder_to_waist 1.65, clavicle Wide, limb Long). A query authored
// at exactly that profile must rank it first.
// ===========================================================================

/// A `FrameFeatures` with both banded fields populated.
fn features(
    shoulder_to_waist: f64,
    clavicle_width: Option<WidthBand>,
    limb_length: Option<LengthBand>,
) -> FrameFeatures {
    FrameFeatures {
        shoulder_to_waist,
        clavicle_width,
        limb_length,
        confidence: 0.9,
    }
}

/// The `frame_profile` of a library archetype by slug — the target a near-query
/// is authored against.
fn frame_of(slug: &str) -> &'static fitai_core::archetype::FrameProfile {
    let a: &Archetype = library()
        .iter()
        .find(|a| a.id == slug)
        .unwrap_or_else(|| panic!("library must contain {slug}"));
    &a.frame_profile
}

fn ids(matches: &[RankedMatch]) -> Vec<&'static str> {
    matches.iter().map(|m| m.archetype.id).collect()
}

// ===========================================================================
// SAC2 / AC2: a query near a known archetype ranks it first.
// ===========================================================================

#[test]
fn rank_returns_every_library_archetype_once() {
    let f = features(1.65, Some(WidthBand::Wide), Some(LengthBand::Long));
    let matches = rank(&f, library());

    assert_eq!(
        matches.len(),
        library().len(),
        "rank must return one RankedMatch per library archetype"
    );
    let mut seen = std::collections::HashSet::new();
    for m in &matches {
        assert!(
            seen.insert(m.archetype.id),
            "each archetype must appear exactly once; {:?} repeated",
            m.archetype.id
        );
    }
}

#[test]
fn rank_places_the_nearest_archetype_first() {
    // Author the query AT the classic-aesthetic-taper profile (ratio 1.65,
    // clavicle Wide, limb Long). It is the unique high-taper/wide/long record,
    // so an exact-on-its-profile query must rank it first with a near-zero
    // distance.
    let target = frame_of("classic-aesthetic-taper");
    let f = features(
        target.shoulder_to_waist,
        Some(target.clavicle_width),
        Some(target.limb_length),
    );
    let matches = rank(&f, library());

    assert_eq!(
        matches[0].archetype.id,
        "classic-aesthetic-taper",
        "a query authored at the V-taper profile must rank it first; got order {:?}",
        ids(&matches)
    );
    assert!(
        matches[0].distance <= 1e-9,
        "an exact-on-profile query must have ~zero distance, got {}",
        matches[0].distance
    );
}

#[test]
fn rank_is_sorted_ascending_by_distance() {
    let f = features(1.5, Some(WidthBand::Average), Some(LengthBand::Average));
    let matches = rank(&f, library());

    for pair in matches.windows(2) {
        assert!(
            pair[0].distance <= pair[1].distance,
            "rank output must be sorted nearest-first: {} then {} out of order",
            pair[0].distance,
            pair[1].distance
        );
    }
}

#[test]
fn rank_prefers_a_compact_low_ratio_query_for_the_compact_archetype() {
    // The lowest-ratio library record is `powerbuilder-leverage` (1.45, short
    // limbs). A compact, low-ratio, short-limbed query must rank it above the
    // high-taper V-taper record — the numeric ratio (weight 0.6) dominates.
    let f = features(1.45, Some(WidthBand::Average), Some(LengthBand::Short));
    let matches = rank(&f, library());

    let order = ids(&matches);
    let pos = |slug: &str| order.iter().position(|id| *id == slug).unwrap();
    assert!(
        pos("powerbuilder-leverage") < pos("classic-aesthetic-taper"),
        "a compact low-ratio query must rank the compact archetype above the V-taper; order {order:?}"
    );
}

// ===========================================================================
// SAC2 / AC2: absent banded fields are skipped + renormalized, never penalized.
// ===========================================================================

#[test]
fn rank_with_both_bands_absent_collapses_to_the_numeric_term_without_panic() {
    // The all-categorical-absent case: only the numeric ratio is present, so its
    // weight renormalizes to 1.0. This must not panic (no divide-by-zero in the
    // renormalization) and must still produce a full, sorted ranking.
    let f = features(1.65, None, None);
    let matches = rank(&f, library());

    assert_eq!(matches.len(), library().len());
    for pair in matches.windows(2) {
        assert!(pair[0].distance <= pair[1].distance);
    }
    // With only the ratio in play, the record whose ratio is nearest 1.65 must
    // rank first — that is the V-taper record (1.65).
    assert_eq!(
        matches[0].archetype.id,
        "classic-aesthetic-taper",
        "with both bands absent the nearest-ratio record (1.65) must win; got {:?}",
        ids(&matches)
    );
}

#[test]
fn rank_does_not_penalize_an_absent_band() {
    // Two queries identical on the (dominant) ratio: one supplies a matching
    // clavicle band, the other omits it. Omitting a field must NOT inflate the
    // distance to the target beyond the supplied-and-matching case — an absent
    // field is skipped, never counted as a mismatch.
    let target = frame_of("classic-aesthetic-taper");

    let with_band = features(
        target.shoulder_to_waist,
        Some(target.clavicle_width),
        Some(target.limb_length),
    );
    let without_clavicle = features(target.shoulder_to_waist, None, Some(target.limb_length));

    let d_with = rank(&with_band, library())
        .into_iter()
        .find(|m| m.archetype.id == "classic-aesthetic-taper")
        .unwrap()
        .distance;
    let d_without = rank(&without_clavicle, library())
        .into_iter()
        .find(|m| m.archetype.id == "classic-aesthetic-taper")
        .unwrap()
        .distance;

    // Both bands matched the target exactly in the `with_band` case (distance
    // contribution 0), so dropping one cannot make the distance LARGER.
    assert!(
        d_without <= d_with + 1e-9,
        "omitting a matching band must not penalize: with={d_with} without={d_without}"
    );
}

// ===========================================================================
// SAC2 / AC2: distance / score bounds and the score = 1 - distance identity.
// ===========================================================================

#[test]
fn rank_distance_is_bounded_and_score_is_its_complement() {
    let f = features(2.0, Some(WidthBand::Narrow), Some(LengthBand::Long));
    let matches = rank(&f, library());

    for m in &matches {
        assert!(
            (0.0..=1.0).contains(&m.distance),
            "distance {} must be in [0.0, 1.0]",
            m.distance
        );
        let score = 1.0 - m.distance;
        assert!(
            (0.0..=1.0).contains(&score),
            "score {score} (= 1 - distance) must be in [0.0, 1.0]"
        );
    }
}

// ===========================================================================
// SAC2 / AC2: determinism, totality, and stable tie-breaking.
// ===========================================================================

#[test]
fn rank_is_deterministic_across_repeated_calls() {
    let f = features(1.55, Some(WidthBand::Average), Some(LengthBand::Average));
    let first = ids(&rank(&f, library()));
    let second = ids(&rank(&f, library()));
    assert_eq!(
        first, second,
        "rank must be deterministic — identical input yields identical order"
    );
}

#[test]
fn rank_breaks_ties_by_library_order() {
    // Several library records share the ratio 1.5 (heavy-duty-mass,
    // high-intensity-minimalist, mass-monster-volume). A query whose ONLY signal
    // is the ratio (both bands absent) makes those records tie on the numeric
    // term; a stable sort must keep them in library order relative to each other.
    let f = features(1.5, None, None);
    let matches = rank(&f, library());
    let order = ids(&matches);

    // The library's authored order among the ratio-1.5 records.
    let lib_order: Vec<&str> = library()
        .iter()
        .filter(|a| (a.frame_profile.shoulder_to_waist - 1.5).abs() < 1e-9)
        .map(|a| a.id)
        .collect();
    assert!(
        lib_order.len() >= 2,
        "this stability test needs at least two ratio-1.5 records; library has {}",
        lib_order.len()
    );

    let tied_in_output: Vec<&str> = order
        .iter()
        .copied()
        .filter(|id| lib_order.contains(id))
        .collect();
    assert_eq!(
        tied_in_output, lib_order,
        "tied (equal-distance) records must keep their relative library order (stable sort)"
    );
}

#[test]
fn rank_does_not_panic_on_a_non_finite_query_ratio() {
    // Totality (SPEC-0013 §2.3): even a NaN/inf query ratio — which the API path
    // guards against upstream, but the pure matcher must still be total — must
    // sort with `f64::total_cmp` and never panic. (We assert it returns a full
    // ranking without unwinding; the exact order of NaN entries is unspecified.)
    let f = features(f64::NAN, Some(WidthBand::Wide), Some(LengthBand::Long));
    let matches = rank(&f, library());
    assert_eq!(
        matches.len(),
        library().len(),
        "a non-finite query ratio must still yield a total, panic-free ranking"
    );
}

#[test]
fn rank_over_an_empty_library_is_empty() {
    // A total function over no candidates is the empty ranking, not a panic.
    let f = features(1.6, Some(WidthBand::Wide), Some(LengthBand::Long));
    let matches = rank(&f, &[]);
    assert!(
        matches.is_empty(),
        "ranking against no archetypes must be the empty Vec"
    );
}
