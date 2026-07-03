//! fitai-api library entry. Hosts the `AppState`, the router builder, and
//! re-exports for tests / integration code.
//!
//! Inside `#[cfg(test)]` (unit tests in this crate) the strict
//! `clippy::unwrap_used`/`expect_used`/`panic` lints are relaxed — test
//! code is the conventional place for those. Integration tests under
//! `tests/` are separate crates and each opt out at file top.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod archetype;
pub mod auth;
pub mod db;
pub mod error;
mod health;
pub(crate) mod http;
pub mod matching;
pub mod measurements;
pub mod nutrition;
pub mod photo;
pub mod pose;
pub mod profile;
pub mod program;
pub mod storage;
pub(crate) mod synthetic;
pub mod voice;
pub mod workout;

use std::{sync::Arc, time::Duration};

use axum::Router;
use sqlx::PgPool;
use tower_http::cors::CorsLayer;

use crate::{
    auth::GoogleAuthSettings, pose::PoseEstimator, storage::ObjectStore, voice::VoiceIntentSettings,
};

/// Application state shared across handlers via `Router::with_state`.
///
/// `Clone` is cheap: `PgPool` is `Arc`-internal, `jwt_secret` is `Arc<[u8]>`,
/// `Duration` is `Copy`, and `store`/`pose` are `Arc`s over their seams.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: Arc<[u8]>,
    pub jwt_ttl: Duration,
    pub store: Arc<dyn ObjectStore>,
    pub pose: Arc<dyn PoseEstimator>,
    pub google: GoogleAuthSettings,
    pub voice: VoiceIntentSettings,
}

/// Build the application router with all routes mounted.
///
/// `main.rs` wraps this with `axum::serve`. Tests call it directly via
/// `tower::ServiceExt::oneshot` or boot a real server in a task.
pub fn app(state: AppState) -> Router {
    Router::new()
        .merge(health::router())
        .merge(archetype::routes())
        .merge(auth::routes())
        .merge(profile::routes())
        .merge(workout::routes())
        .merge(nutrition::routes())
        .merge(photo::routes())
        .merge(matching::routes())
        .merge(program::routes())
        .merge(measurements::routes())
        .merge(synthetic::routes::routes())
        .merge(voice::routes())
        .with_state(state)
        .layer(CorsLayer::permissive())
}
