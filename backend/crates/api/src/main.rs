//! fitai-api binary: load config, build pool, run migrations, serve.
//!
//! No `.unwrap()` / `.expect()`. Signal-handler install failures propagate
//! via `?` (the same way port-bind failures do); the `ctrl_c` future's own
//! `io::Result` is logged and shutdown proceeds (we'd rather shut down
//! cleanly than abort the process on a `ctrl_c` handler hiccup).

use std::{net::SocketAddr, sync::Arc, time::Duration};

use sqlx::postgres::PgPoolOptions;
use tokio::signal::ctrl_c;

use fitai_api::{app, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let db_url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set")?;
    let jwt_secret = std::env::var("JWT_SECRET").map_err(|_| "JWT_SECRET must be set")?;

    let pool = PgPoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_url)
        .await?;

    sqlx::migrate!("../../migrations").run(&pool).await?;
    tracing::info!("migrations up to date");

    let state = AppState {
        pool,
        jwt_secret: Arc::from(jwt_secret.into_bytes().into_boxed_slice()),
        jwt_ttl: Duration::from_hours(24),
    };

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "fitai-api listening");

    let shutdown = build_shutdown()?;

    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown)
        .await?;

    Ok(())
}

/// Install signal handlers up-front and return a future that resolves when
/// any of them fires. Returning `Err` here is unrecoverable — without
/// signal handling the process cannot gracefully drain, which would corrupt
/// shutdown semantics for `docker stop` / k8s rolling deploys.
fn build_shutdown() -> Result<impl std::future::Future<Output = ()>, std::io::Error> {
    #[cfg(unix)]
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

    Ok(async move {
        #[cfg(unix)]
        {
            tokio::select! {
                r = ctrl_c() => log_ctrl_c_error(r),
                _ = sigterm.recv() => {},
            }
        }
        #[cfg(not(unix))]
        {
            log_ctrl_c_error(ctrl_c().await);
        }
        tracing::info!("shutdown signal received");
    })
}

fn log_ctrl_c_error(r: std::io::Result<()>) {
    if let Err(e) = r {
        tracing::warn!(error = %e, "ctrl_c handler error; shutting down anyway");
    }
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
}
