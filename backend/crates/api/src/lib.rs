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
pub mod authored;
pub mod db;
pub mod error;
pub mod goals;
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
pub mod summary;
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

/// Apply pending migrations, tolerating versions the binary does not know.
///
/// # Why `ignore_missing(true)`
///
/// sqlx defaults to `ignore_missing: false`, which makes the migrator return
/// [`MigrateError::VersionMissing`] when the database records a migration that
/// is not embedded in this binary. Since `main` applies migrations at startup
/// and propagates the error, that default means **rolling the image back to any
/// earlier build makes the API refuse to boot** — permanently, until a new
/// image ships. On Cloudflare Containers, where the instance sleeps and
/// re-boots on demand, that turns a routine rollback into an outage with no
/// way out except rolling forward.
///
/// The emergency path has to exist, so a newer-than-me schema is tolerated.
///
/// # The hazard this accepts, stated plainly
///
/// A rolled-back binary now runs against a **newer schema than it was built
/// for**. That risk is not created by this flag — it is inherent to rolling
/// back across any migration — but the flag is what makes it reachable, so it
/// must be weighed per rollback rather than assumed safe. Additive migrations
/// (new nullable columns, new tables) are generally fine; a rollback across a
/// migration that *dropped* a constraint the old code depended on is not.
///
/// Migrations still run in order, still run exactly once, and a genuine
/// failure still aborts startup.
///
/// # Errors
///
/// Returns [`MigrateError`] when a migration fails to apply, when an
/// already-applied migration's checksum no longer matches (the guard that
/// stops an existing migration being rewritten in place), or when the
/// database is unreachable. Startup aborts in all three cases, by design.
pub async fn run_migrations(pool: &sqlx::PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    let mut migrator = sqlx::migrate!("../../migrations");
    migrator.set_ignore_missing(true);
    migrator.run(pool).await
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
        .merge(authored::routes())
        .merge(goals::routes())
        .merge(profile::routes())
        .merge(workout::routes())
        .merge(nutrition::routes())
        .merge(summary::routes())
        .merge(photo::routes())
        .merge(matching::routes())
        .merge(program::routes())
        .merge(measurements::routes())
        .merge(synthetic::routes::routes())
        .merge(voice::routes())
        .with_state(state)
        .layer(CorsLayer::permissive())
}
