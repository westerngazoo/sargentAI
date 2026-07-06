//! Synthetic body-type matching (R-0030, SPEC-0030).
//!
//! `POST /match/synthetic` — accepts a coarse body shape + fat-band selection
//! and returns the top-3 archetype proposals without a photo or a stored
//! photo-session row. A lookup table maps the 9 shape×band combinations to a
//! valid [`FrameFeatures`]; `rank()` and `instantiate()` are called unchanged.
//!
//! `POST /programs/synthetic` — chooses one of the top-3 proposals from a
//! synthetic selection, re-derives features to verify the chosen archetype was
//! actually in the top-3, then persists the program (`source_session_id = NULL`).

pub(crate) mod routes;

use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use fitai_core::archetype::{library, LengthBand, WidthBand};
use fitai_core::matching::rank;
use fitai_core::pose::FrameFeatures;
use fitai_core::program::{instantiate, ProgramProposal};

use crate::{
    auth::AuthenticatedUser,
    db,
    error::{ApiError, ApiResult},
    program::handlers::UserProgramResponse,
    AppState,
};

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

/// The three coarse body shapes shown in the picker grid.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BodyShape {
    Ectomorph,
    Mesomorph,
    Endomorph,
}

/// The three coarse body-fat bands shown as chips after shape selection.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FatBand {
    Lean,
    Moderate,
    Bulky,
}

#[derive(Deserialize)]
pub(crate) struct SyntheticRequest {
    pub shape: BodyShape,
    pub fat_band: FatBand,
}

#[derive(Serialize)]
pub(crate) struct SyntheticMatchResponse {
    pub shape: BodyShape,
    pub fat_band: FatBand,
    pub proposals: Vec<ProgramProposal>,
}

#[derive(Deserialize)]
pub(crate) struct SyntheticChooseRequest {
    pub archetype_id: String,
    pub shape: BodyShape,
    pub fat_band: FatBand,
}

// ---------------------------------------------------------------------------
// Lookup table  (9 entries — 3 shapes × 3 fat bands)
// ---------------------------------------------------------------------------

/// Map a (`shape`, `fat_band`) pair to the synthetic [`FrameFeatures`] used for
/// ranking. Values are calibrated so each combination's closest archetype
/// matches the bodybuilding phenotype the user selected.
///
/// The `confidence` field is set to `1.0` — synthetic features are exact by
/// definition; no measurement uncertainty exists.
fn synthetic_features(shape: BodyShape, fat_band: FatBand) -> FrameFeatures {
    use BodyShape::{Ectomorph, Endomorph, Mesomorph};
    use FatBand::{Bulky, Lean, Moderate};
    use LengthBand::{Average as Avg, Long, Short};
    use WidthBand::{Average as AvgW, Narrow, Wide};

    let (stw, clav, limb) = match (shape, fat_band) {
        // Ectomorph: narrow, long-limbed — stw suppressed by narrower shoulder span
        (Ectomorph, Lean) => (1.25, Narrow, Long),
        (Ectomorph, Moderate) => (1.20, Narrow, Long),
        (Ectomorph, Bulky) => (1.15, Narrow, Avg),
        // Mesomorph: broad, well-proportioned — classic V-taper
        (Mesomorph, Lean) => (1.65, Wide, Long),
        (Mesomorph, Moderate) => (1.55, Wide, Avg),
        (Mesomorph, Bulky) => (1.50, AvgW, Avg),
        // Endomorph: stocky, short-limbed — lower stw due to wider hip span
        (Endomorph, Lean) => (1.40, AvgW, Short),
        (Endomorph, Moderate) => (1.30, AvgW, Short),
        (Endomorph, Bulky) => (1.15, Narrow, Short),
    };

    FrameFeatures {
        shoulder_to_waist: stw,
        clavicle_width: Some(clav),
        limb_length: Some(limb),
        confidence: 1.0,
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /match/synthetic`
///
/// Returns the top-3 program proposals for the chosen body shape + fat band.
/// No photo is required; no session row is created.
pub(crate) async fn synthetic_match(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<SyntheticRequest>,
) -> ApiResult<Json<SyntheticMatchResponse>> {
    let profile = db::find_profile_by_user(&state.pool, user.user_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    let features = synthetic_features(body.shape, body.fat_band);
    let today = Utc::now().date_naive();

    let proposals: Vec<ProgramProposal> = rank(&features, library())
        .into_iter()
        .take(3)
        .map(|m| {
            let score = 1.0 - m.distance;
            instantiate(m.archetype, &profile, score, m.distance, today)
        })
        .collect();

    Ok(Json(SyntheticMatchResponse {
        shape: body.shape,
        fat_band: body.fat_band,
        proposals,
    }))
}

/// `POST /programs/synthetic`
///
/// Chooses one of the top-3 proposals from a synthetic body-type selection.
/// Re-derives the features from the stored shape+band to verify the chosen
/// archetype was actually in the top-3, then persists the program with
/// `source_session_id = NULL`.
pub(crate) async fn choose_synthetic(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<SyntheticChooseRequest>,
) -> ApiResult<(StatusCode, Json<UserProgramResponse>)> {
    let profile = db::find_profile_by_user(&state.pool, user.user_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    let features = synthetic_features(body.shape, body.fat_band);
    let today = Utc::now().date_naive();

    let top3: Vec<ProgramProposal> = rank(&features, library())
        .into_iter()
        .take(3)
        .map(|m| {
            let score = 1.0 - m.distance;
            instantiate(m.archetype, &profile, score, m.distance, today)
        })
        .collect();

    let chosen = top3
        .into_iter()
        .find(|p| p.archetype_id == body.archetype_id)
        .ok_or(ApiError::Conflict {
            reason: "archetype_not_in_proposals",
        })?;

    let row = db::insert_program(
        &state.pool,
        user.user_id,
        &chosen.archetype_id,
        None, // no photo session
        &chosen.program,
        &chosen.diet,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(UserProgramResponse::try_from(row)?),
    ))
}

// ---------------------------------------------------------------------------
// Unit tests — the pure lookup table (R-0030 backfill).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{synthetic_features, BodyShape, FatBand};
    use fitai_core::archetype::library;
    use fitai_core::matching::rank;

    /// The nine (shape, band) combinations the picker can emit.
    const COMBINATIONS: [(BodyShape, FatBand); 9] = [
        (BodyShape::Ectomorph, FatBand::Lean),
        (BodyShape::Ectomorph, FatBand::Moderate),
        (BodyShape::Ectomorph, FatBand::Bulky),
        (BodyShape::Mesomorph, FatBand::Lean),
        (BodyShape::Mesomorph, FatBand::Moderate),
        (BodyShape::Mesomorph, FatBand::Bulky),
        (BodyShape::Endomorph, FatBand::Lean),
        (BodyShape::Endomorph, FatBand::Moderate),
        (BodyShape::Endomorph, FatBand::Bulky),
    ];

    /// Every (shape, band) arm yields a `FrameFeatures` whose numeric ratio sits
    /// in the matcher's valid `1.0..=2.5` span, whose banded descriptors are
    /// present, and whose confidence is the exact `1.0` the doc-comment promises.
    #[test]
    fn every_combination_yields_valid_frame_features() {
        for (shape, band) in COMBINATIONS {
            let features = synthetic_features(shape, band);
            assert!(
                (1.0..=2.5).contains(&features.shoulder_to_waist),
                "{shape:?}/{band:?} ratio {} out of the 1.0..=2.5 matcher span",
                features.shoulder_to_waist
            );
            assert!(
                features.clavicle_width.is_some(),
                "{shape:?}/{band:?} must carry a clavicle band"
            );
            assert!(
                features.limb_length.is_some(),
                "{shape:?}/{band:?} must carry a limb band"
            );
            assert!(
                (features.confidence - 1.0).abs() < f64::EPSILON,
                "{shape:?}/{band:?} confidence must be exactly 1.0"
            );
        }
    }

    /// `rank` accepts every synthetic feature set and returns a ranked, ordered
    /// top-3 with finite distances — the exact shape `synthetic_match` takes.
    #[test]
    fn every_combination_ranks_a_non_empty_top3() {
        for (shape, band) in COMBINATIONS {
            let features = synthetic_features(shape, band);
            let ranked = rank(&features, library());

            let top3: Vec<_> = ranked.iter().take(3).collect();
            assert_eq!(
                top3.len(),
                3,
                "{shape:?}/{band:?} must produce a full top-3 from the six-archetype library"
            );

            for m in &top3 {
                assert!(
                    m.distance.is_finite() && (0.0..=1.0).contains(&m.distance),
                    "{shape:?}/{band:?} distance {} must be a finite score in 0.0..=1.0",
                    m.distance
                );
            }

            // Nearest-first: distances are non-decreasing across the top-3.
            for pair in top3.windows(2) {
                assert!(
                    pair[0].distance <= pair[1].distance,
                    "{shape:?}/{band:?} top-3 must be ordered nearest-first"
                );
            }
        }
    }
}
