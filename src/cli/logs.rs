use anyhow::{bail, Result};
use std::path::PathBuf;
use std::process::Stdio;

pub async fn run(name: &str, hb_dir: &PathBuf, follow: bool) -> Result<()> {
    let log_path = hb_dir.join("logs").join(format!("{}.log", name));

    if !log_path.exists() {
        bail!(
            "No log file found for job {:?} at {}",
            name,
            log_path.display()
        );
    }

    if follow {
        // tail -F follows log rotations by tracking the filename
        let status = tokio::process::Command::new("tail")
            .args(["-F", log_path.to_str().unwrap()])
            .stdin(Stdio::null())
            .status()
            .await?;

        if !status.success() {
            bail!("tail exited with status {}", status);
        }
    } else {
        // Print the current log to stdout
        let content = tokio::fs::read_to_string(&log_path).await?;
        print!("{}", content);
    }

    Ok(())
}
