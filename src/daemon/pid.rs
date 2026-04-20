use anyhow::{bail, Result};
use std::path::PathBuf;
use tracing::warn;

/// RAII guard for `~/.heartbeat/heartbeat.pid`.
/// Removed on drop (clean shutdown). Prevents double-start.
pub struct PidFile {
    path: PathBuf,
}

impl PidFile {
    /// Create or overwrite the PID file.
    /// Returns `Err` if another live process already holds it.
    pub fn acquire(path: PathBuf) -> Result<Self> {
        if path.exists() {
            let existing_pid = std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok());

            if let Some(pid) = existing_pid {
                if process_is_alive(pid) {
                    bail!("heartbeat is already running (PID {})", pid);
                }
                warn!("Removing stale PID file (PID {} is not running)", pid);
            }
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let pid = std::process::id();
        std::fs::write(&path, format!("{}\n", pid))?;

        Ok(PidFile { path })
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Check if a process with the given PID is alive using `kill -0`.
fn process_is_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
