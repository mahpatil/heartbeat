mod daemon;
mod job;
mod log;
mod task;

use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

use daemon::{controller::Controller, pid::PidFile};

#[tokio::main]
async fn main() -> Result<()> {
    // ── Logging ───────────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("heartbeat=info".parse()?),
        )
        .init();

    let hb_dir = hb_dir();

    // ── Load .env ─────────────────────────────────────────────────────────────
    let env_path = hb_dir.join(".env");
    if env_path.exists() {
        dotenvy::from_path(&env_path).ok();
        info!("Loaded env from {}", env_path.display());
    } else {
        tracing::debug!("No .env file at {}", env_path.display());
    }

    // ── PID file (prevents double-start) ─────────────────────────────────────
    let _pid = PidFile::acquire(hb_dir.join("heartbeat.pid"))?;

    // ── Graceful shutdown channel ─────────────────────────────────────────────
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl-C");
            }
            _ = async {
                tokio::signal::unix::signal(
                    tokio::signal::unix::SignalKind::terminate(),
                )
                .expect("failed to install SIGTERM handler")
                .recv()
                .await
            } => {
                info!("Received SIGTERM");
            }
        }
        let _ = shutdown_tx.send(());
    });

    // ── Start controller ──────────────────────────────────────────────────────
    info!("heartbeat daemon starting (jobs: {})", hb_dir.join("jobs").display());
    let controller = Controller::new(hb_dir);
    controller.run(shutdown_rx).await?;

    info!("heartbeat daemon stopped");
    Ok(())
}

fn hb_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("~"))
        .join(".heartbeat")
}
