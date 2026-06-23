use axum::{routing::post, Router};

use crate::AppState;

use super::{choose_synthetic, synthetic_match};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/match/synthetic", post(synthetic_match))
        .route("/programs/synthetic", post(choose_synthetic))
}
