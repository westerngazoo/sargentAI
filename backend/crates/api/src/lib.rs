//! fitai-api library entry. Exposes the router so tests don't bind a port.
//!
//! Inside `#[cfg(test)]` (unit tests in this crate) the strict
//! `clippy::unwrap_used`/`expect_used`/`panic` lints are relaxed — test
//! code is the conventional place for those. Integration tests under
//! `tests/` are separate crates and each opt out at file top.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod health;

use axum::Router;

/// Build the application router.
///
/// `main.rs` wraps this with `axum::serve`. Tests call it directly via
/// `tower::ServiceExt::oneshot` or boot a real server in a task.
///
/// (`Router` is itself `#[must_use]`, so no attribute here — `clippy::double_must_use`.)
pub fn app() -> Router {
    Router::new().merge(health::router())
}
