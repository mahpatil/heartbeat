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

    // Validate BEFORE touching the jobs directory — never delete on error.
    let cfg = crate::job::config::JobConfig::load(src)
        .map_err(|e| anyhow::anyhow!("Invalid job file: {}", e))?;

    let jobs_dir = hb_dir.join("jobs");
    tokio::fs::create_dir_all(&jobs_dir).await?;

    let dest = jobs_dir.join(src.file_name().unwrap());

    // Always write to dest — even when src == dest — so the filesystem watcher
    // fires a Modify event and the daemon hot-reloads the job.
    let content = tokio::fs::read(src).await?;
    tokio::fs::write(&dest, &content).await?;
    info!("Job applied: {} -> {}", src.display(), dest.display());

    println!("Applied: {} (schedule: {})", cfg.name, cfg.schedule.display());

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn valid_htb() -> &'static str {
        "---\nname: test-job\nschedule: every 5m\n---\necho hello\n"
    }

    fn invalid_htb_no_closing() -> &'static str {
        "---\nname: test-job\nschedule: every 5m\n"
    }

    async fn write_htb(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        tokio::fs::write(&path, content).await.unwrap();
        path
    }

    // ── Valid file is copied ──────────────────────────────────────────────────

    #[tokio::test]
    async fn valid_file_is_copied_to_jobs_dir() {
        let src_dir = TempDir::new().unwrap();
        let hb_dir = TempDir::new().unwrap();
        let src = write_htb(&src_dir, "test-job.htb", valid_htb()).await;

        run(&src, &hb_dir.path().to_path_buf()).await.unwrap();

        let dest = hb_dir.path().join("jobs").join("test-job.htb");
        assert!(dest.exists(), "job file not copied to jobs dir");
    }

    // ── Invalid file: source is NOT deleted ───────────────────────────────────

    #[tokio::test]
    async fn invalid_file_source_is_never_deleted() {
        let src_dir = TempDir::new().unwrap();
        let hb_dir = TempDir::new().unwrap();
        let src = write_htb(&src_dir, "bad.htb", invalid_htb_no_closing()).await;

        let result = run(&src, &hb_dir.path().to_path_buf()).await;

        assert!(result.is_err(), "expected error on invalid file");
        assert!(src.exists(), "source file was deleted on parse error — must never happen");
    }

    #[tokio::test]
    async fn invalid_file_dest_is_not_created() {
        let src_dir = TempDir::new().unwrap();
        let hb_dir = TempDir::new().unwrap();
        let src = write_htb(&src_dir, "bad.htb", invalid_htb_no_closing()).await;

        let _ = run(&src, &hb_dir.path().to_path_buf()).await;

        let dest = hb_dir.path().join("jobs").join("bad.htb");
        assert!(!dest.exists(), "dest was created despite parse error");
    }

    // ── Applying file already in jobs/ rewrites it (triggering watcher) ─────

    #[tokio::test]
    async fn apply_from_jobs_dir_rewrites_for_watcher() {
        let hb_dir = TempDir::new().unwrap();
        let jobs_dir = hb_dir.path().join("jobs");
        tokio::fs::create_dir_all(&jobs_dir).await.unwrap();

        // Write directly into jobs/
        let src = jobs_dir.join("test-job.htb");
        tokio::fs::write(&src, valid_htb()).await.unwrap();

        // Apply from the same location — must succeed and leave the file intact
        run(&src, &hb_dir.path().to_path_buf()).await.unwrap();
        assert!(src.exists(), "file in jobs/ was deleted by apply");
        let content = tokio::fs::read_to_string(&src).await.unwrap();
        assert_eq!(content, valid_htb(), "file content changed unexpectedly");
    }

    // ── Wrong extension rejected ──────────────────────────────────────────────

    #[tokio::test]
    async fn wrong_extension_is_rejected() {
        let src_dir = TempDir::new().unwrap();
        let hb_dir = TempDir::new().unwrap();
        let src = write_htb(&src_dir, "test-job.yaml", valid_htb()).await;

        let err = run(&src, &hb_dir.path().to_path_buf()).await.unwrap_err();
        assert!(err.to_string().contains("Only .htb"), "got: {}", err);
    }

    // ── Error message mentions the file ──────────────────────────────────────

    #[tokio::test]
    async fn error_message_names_file() {
        let src_dir = TempDir::new().unwrap();
        let hb_dir = TempDir::new().unwrap();
        let src = write_htb(&src_dir, "broken.htb", invalid_htb_no_closing()).await;

        let err = run(&src, &hb_dir.path().to_path_buf()).await.unwrap_err();
        assert!(
            err.to_string().contains("Invalid job file"),
            "error should mention 'Invalid job file', got: {}",
            err
        );
    }
}
