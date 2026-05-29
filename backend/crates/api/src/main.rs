//! fitai-api binary: bind, serve, shut down gracefully.
//!
//! No `.unwrap()` / `.expect()`. Signal-handler install failures propagate
//! via `?` (the same way port-bind failures do); the `ctrl_c` future's own
//! `io::Result` is logged and shutdown proceeds (we'd rather shut down
//! cleanly than abort the process on a `ctrl_c` handler hiccup).

use std::net::SocketAddr;
use tokio::signal::ctrl_c;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "fitai-api listening");

    let shutdown = build_shutdown()?;

    axum::serve(listener, fitai_api::app())
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
