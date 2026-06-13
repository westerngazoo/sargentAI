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
pub mod nutrition;
pub mod photo;
pub mod profile;
pub mod storage;
pub mod workout;

use std::{sync::Arc, time::Duration};

use axum::Router;
use sqlx::PgPool;

use crate::storage::ObjectStore;

/// Application state shared across handlers via `Router::with_state`.
///
/// `Clone` is cheap: `PgPool` is `Arc`-internal, `jwt_secret` is `Arc<[u8]>`,
/// `Duration` is `Copy`, and `store` is an `Arc` over the object-store seam.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: Arc<[u8]>,
    pub jwt_ttl: Duration,
    pub store: Arc<dyn ObjectStore>,
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
        .with_state(state)
}
