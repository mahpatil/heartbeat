use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

use crate::cli::install::plist_path;
use crate::daemon::{controller::Controller, pid::PidFile};

pub async fn run(hb_dir: PathBuf) -> Result<()> {
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

    info!(
        "heartbeat daemon starting (jobs: {})",
        hb_dir.join("jobs").display()
    );

    if let Ok(home) = std::env::var("HOME") {
        if !plist_path(&home).exists() {
            info!("Tip: run `heartbeat install --autostart` to start automatically at login");
        }
    }

    let controller = Controller::new(hb_dir);
    controller.run(shutdown_rx).await?;

    info!("heartbeat daemon stopped");
    Ok(())
}
