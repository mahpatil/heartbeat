use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
use tracing::info;

/// Copy a `.htb` job file into `~/.heartbeat/jobs/`.
/// The daemon's filesystem watcher picks it up automatically.
pub async fn run(src: &Path, hb_dir: &PathBuf) -> Result<()> {
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();

    if ext != "htb" {
        bail!("Only .htb files are supported (got {:?})", src);
    }

    if !src.exists() {
        bail!("File not found: {}", src.display());
    }

    let jobs_dir = hb_dir.join("jobs");
    tokio::fs::create_dir_all(&jobs_dir).await?;

    let dest = jobs_dir.join(src.file_name().unwrap());
    tokio::fs::copy(src, &dest).await?;

    // Verify the file parses correctly
    match crate::job::config::JobConfig::load(&dest) {
        Ok(cfg) => {
            println!("Applied: {} (schedule: {})", cfg.name, cfg.schedule.display());
            info!("Job applied: {} -> {}", src.display(), dest.display());
        }
        Err(e) => {
            // Remove the bad file so the daemon doesn't choke on it
            tokio::fs::remove_file(&dest).await.ok();
            bail!("Invalid job file: {}", e);
        }
    }

    Ok(())
}
