use anyhow::Result;
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024; // 10 MB
const MAX_ROTATIONS: u32 = 5;

// ── Public API ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct JobLogger {
    inner: Arc<Mutex<LogInner>>,
    job_name: String,
}

struct LogInner {
    file: tokio::fs::File,
    log_path: PathBuf,
    bytes_written: u64,
}

impl JobLogger {
    pub async fn new(job_name: &str, log_dir: &Path) -> Result<Self> {
        tokio::fs::create_dir_all(log_dir).await?;
        let log_path = log_dir.join(format!("{}.log", job_name));
        let (file, bytes_written) = open_log(&log_path).await?;
        Ok(Self {
            inner: Arc::new(Mutex::new(LogInner { file, log_path, bytes_written })),
            job_name: job_name.to_string(),
        })
    }

    pub fn path(&self) -> PathBuf {
        // Best-effort path without awaiting — callers use this synchronously.
        // Because all writes go through inner.lock(), path is stable.
        futures_path_hack(&self.inner)
    }

    /// Write a timestamped line. Rotates if the file exceeds MAX_LOG_BYTES.
    pub async fn write_line(&self, line: &str) {
        let entry = format!(
            "{} [{}] {}\n",
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
            self.job_name,
            line
        );
        let mut inner = self.inner.lock().await;
        if inner.file.write_all(entry.as_bytes()).await.is_ok() {
            inner.file.flush().await.ok();
            inner.bytes_written += entry.len() as u64;
            if inner.bytes_written >= MAX_LOG_BYTES {
                rotate(&mut inner).await;
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Get the log path without holding an async lock (reads the path synchronously).
/// Safe because log_path is written once at creation and never changes.
fn futures_path_hack(inner: &Arc<Mutex<LogInner>>) -> PathBuf {
    // Use try_lock — if the logger is currently being written to, we still
    // know the path (it never changes), so fall back to a safe default.
    if let Ok(guard) = inner.try_lock() {
        guard.log_path.clone()
    } else {
        // Should not happen in normal use; return a placeholder.
        PathBuf::from("/tmp/heartbeat.log")
    }
}

async fn open_log(path: &PathBuf) -> Result<(tokio::fs::File, u64)> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    let bytes_written = file.metadata().await.map(|m| m.len()).unwrap_or(0);
    Ok((file, bytes_written))
}

/// Rotate log files: .log.4→.log.5, .log.3→.log.4, …, .log→.log.1, open new .log
async fn rotate(inner: &mut LogInner) {
    // Drop the current file handle before renaming
    // We can't drop it directly, but we'll reopen after renaming.

    let base = inner.log_path.clone();

    // Shift existing rotations (oldest first to avoid clobbering)
    for i in (1..MAX_ROTATIONS).rev() {
        let from = rotated_path(&base, i);
        let to = rotated_path(&base, i + 1);
        if from.exists() {
            tokio::fs::rename(&from, &to).await.ok();
        }
    }

    // Move current log to .log.1
    if base.exists() {
        let to = rotated_path(&base, 1);
        tokio::fs::rename(&base, &to).await.ok();
    }

    // Open fresh log file
    match open_log(&base).await {
        Ok((file, _)) => {
            inner.file = file;
            inner.bytes_written = 0;
        }
        Err(e) => {
            tracing::error!("Failed to open new log after rotation: {}", e);
        }
    }
}

fn rotated_path(base: &Path, n: u32) -> PathBuf {
    let mut p = base.to_path_buf();
    let name = format!(
        "{}.{}",
        p.file_name().unwrap_or_default().to_string_lossy(),
        n
    );
    p.set_file_name(name);
    p
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_write_creates_log() {
        let dir = TempDir::new().unwrap();
        let logger = JobLogger::new("test-job", dir.path()).await.unwrap();
        logger.write_line("hello world").await;
        let content = tokio::fs::read_to_string(logger.path()).await.unwrap();
        assert!(content.contains("[test-job] hello world"));
    }

    #[tokio::test]
    async fn test_timestamp_format() {
        let dir = TempDir::new().unwrap();
        let logger = JobLogger::new("ts-test", dir.path()).await.unwrap();
        logger.write_line("check").await;
        let content = tokio::fs::read_to_string(logger.path()).await.unwrap();
        // Should start with ISO-8601 UTC timestamp
        assert!(content.contains('T'));
        assert!(content.contains('Z'));
    }

    #[tokio::test]
    async fn test_rotation_triggers() {
        let dir = TempDir::new().unwrap();
        let logger = JobLogger::new("rot-test", dir.path()).await.unwrap();

        // Write slightly over MAX_LOG_BYTES worth of data
        let chunk = "x".repeat(1024); // 1 KB line
        for _ in 0..=10_240 {
            // 10MB+
            logger.write_line(&chunk).await;
        }

        let rotated = dir.path().join("rot-test.log.1");
        assert!(
            rotated.exists(),
            "expected rot-test.log.1 to exist after rotation"
        );
    }

    #[tokio::test]
    async fn test_max_5_rotations() {
        let dir = TempDir::new().unwrap();
        let logger = JobLogger::new("maxrot", dir.path()).await.unwrap();
        let chunk = "y".repeat(1024);

        // Trigger 6 rotations
        for _ in 0..=61_440 {
            logger.write_line(&chunk).await;
        }

        // .log.5 should exist, but no .log.6
        assert!(dir.path().join("maxrot.log.5").exists());
        assert!(!dir.path().join("maxrot.log.6").exists());
    }
}
