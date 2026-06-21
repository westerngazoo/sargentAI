//! Photo→archetype match surface: `POST /photo-sessions/:id/match` (R-0013,
//! SPEC-0013 §2.5).
//!
//! [`estimate_first_usable`] is `pub(crate)` so `api::program` can run the
//! same ONNX pipeline without duplicating logic (SPEC-0014 §2.4.1).

mod handlers;

use axum::{routing::post, Router};

use fitai_core::{pose::PoseKeypoints, Angle};

use crate::{
    db::MatchCandidate,
    error::{ApiError, ApiResult},
    pose::PoseError,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/photo-sessions/:id/match", post(handlers::match_session))
}

/// Try candidates **front-angle first, then stored order**; the first usable
/// pose wins. A `NoPersonDetected` photo falls through to the next; a decode /
/// inference fault is a hard `500`. No usable pose anywhere → `422`.
///
/// `pub(crate)` so `api::program` can call the same ONNX pipeline without a
/// second estimation round-trip (SPEC-0014 §2.4.1).
pub(crate) async fn estimate_first_usable(
    state: &AppState,
    candidates: Vec<MatchCandidate>,
) -> ApiResult<PoseKeypoints> {
    for candidate in front_first(candidates) {
        let bytes = state.store.get(&candidate.storage_key).await?;
        match state.pose.estimate(&bytes, candidate.content_type).await {
            Ok(keypoints) => return Ok(keypoints),
            Err(PoseError::NoPersonDetected) => {}
            Err(other) => return Err(other.into()),
        }
    }
    Err(ApiError::Unprocessable {
        reason: "no_person_detected",
    })
}

fn front_first(mut candidates: Vec<MatchCandidate>) -> Vec<MatchCandidate> {
    candidates.sort_by_key(|c| c.angle != Some(Angle::Front));
    candidates
}
