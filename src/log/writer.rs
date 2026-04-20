use anyhow::Result;
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct JobLogger {
    inner: Arc<Mutex<LogInner>>,
    log_path: PathBuf,
    job_name: String,
}

struct LogInner {
    file: tokio::fs::File,
}

impl JobLogger {
    pub async fn new(job_name: &str, log_dir: &Path) -> Result<Self> {
        tokio::fs::create_dir_all(log_dir).await?;
        let log_path = log_dir.join(format!("{}.log", job_name));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .await?;
        Ok(Self {
            inner: Arc::new(Mutex::new(LogInner { file })),
            log_path,
            job_name: job_name.to_string(),
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.log_path
    }

    /// Write a line prefixed with timestamp and job name.
    pub async fn write_line(&self, line: &str) {
        let entry = format!(
            "{} [{}] {}\n",
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
            self.job_name,
            line
        );
        let mut inner = self.inner.lock().await;
        inner.file.write_all(entry.as_bytes()).await.ok();
        inner.file.flush().await.ok();
    }
}
