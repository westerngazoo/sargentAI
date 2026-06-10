//! Photo-session surface: session CRUD + multipart photo upload / byte download
//! under `/photo-sessions`.

mod handlers;

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};

use crate::AppState;

/// The upload route accepts bodies up to `MAX_BYTES` plus a small multipart
/// slack; a body beyond this is rejected at the layer as `413` before the
/// handler, while a body within the slack but over `MAX_BYTES` is buffered and
/// rejected as `400` by the size validator (SPEC-0006 §2.3).
// `fitai_core::MAX_BYTES` (10 MiB) plus 1 MiB of multipart-framing slack.
const BODY_LIMIT: usize = 11 * 1024 * 1024;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/photo-sessions",
            post(handlers::create_session).get(handlers::list_sessions),
        )
        .route(
            "/photo-sessions/:id",
            get(handlers::get_session).delete(handlers::delete_session),
        )
        .route("/photo-sessions/:id/photos", post(handlers::upload_photo))
        .route(
            "/photo-sessions/:id/photos/:photo_id",
            get(handlers::download_photo).delete(handlers::delete_photo),
        )
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
}
