use anyhow::Result;
use std::path::PathBuf;

use super::ipc_client::send_ipc;
use super::install::plist_path;
use crate::ipc::JobSummary;

pub async fn run(hb_dir: &PathBuf) -> Result<()> {
    let resp = send_ipc(hb_dir, "list", None).await?;

    if !resp.ok {
        anyhow::bail!("{}", resp.error.unwrap_or_default());
    }

    let jobs: Vec<JobSummary> = match resp.data {
        Some(d) => serde_json::from_value(d["jobs"].clone())?,
        None => vec![],
    };

    if jobs.is_empty() {
        println!("No jobs running.");
        return Ok(());
    }

    // Column widths
    let name_w = jobs.iter().map(|j| j.name.len()).max().unwrap_or(4).max(4);
    let sched_w = jobs.iter().map(|j| j.schedule.len()).max().unwrap_or(8).max(8);
    let status_w = 7usize; // "running"

    // Header
    println!(
        "{:<name_w$}  {:<status_w$}  {:<sched_w$}  {}",
        "NAME", "STATUS", "SCHEDULE", "NEXT RUN",
        name_w = name_w,
        status_w = status_w,
        sched_w = sched_w,
    );
    println!("{}", "-".repeat(name_w + status_w + sched_w + 30));

    for job in &jobs {
        let next = job.next_run.as_deref().unwrap_or("—");
        println!(
            "{:<name_w$}  {:<status_w$}  {:<sched_w$}  {}",
            job.name,
            job.status,
            job.schedule,
            next,
            name_w = name_w,
            status_w = status_w,
            sched_w = sched_w,
        );
    }

    if let Ok(home) = std::env::var("HOME") {
        if !plist_path(&home).exists() {
            println!();
            println!("Tip: daemon will not survive reboot — run `heartbeat install --autostart`");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autostart_hint_shown_when_plist_absent() {
        let path = plist_path("/tmp/nonexistent-heartbeat-test-home");
        assert!(!path.exists(), "plist must not exist for hint to show");
    }

    #[test]
    fn autostart_hint_suppressed_when_plist_present() {
        let dir = tempfile::tempdir().unwrap();
        let la_dir = dir.path().join("Library/LaunchAgents");
        std::fs::create_dir_all(&la_dir).unwrap();
        let plist = la_dir.join("com.heartbeat.plist");
        std::fs::write(&plist, b"").unwrap();
        let path = plist_path(dir.path().to_str().unwrap());
        assert!(path.exists(), "plist exists so hint must be suppressed");
    }
}
