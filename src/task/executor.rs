use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::info;

use super::types::StepDef;
use crate::log::writer::JobLogger;

const AGENT_RUNNER: &str = "heartbeat-agent-runner.sh";

/// Execute one step. Returns Ok on success, Err on failure.
pub async fn execute_step(
    step: &StepDef,
    job_workspace: &str,
    job_name: &str,
    step_label: &str,
    logger: &JobLogger,
) -> Result<()> {
    match step {
        StepDef::Agent { agent, prompt, flags, workspace, .. } => {
            let ws = workspace.as_deref().unwrap_or(job_workspace);
            execute_agent(agent, ws, prompt, flags, job_name, step_label, logger).await
        }
        StepDef::Shell { command, workspace, .. } => {
            let ws = workspace.as_deref().unwrap_or(job_workspace);
            execute_shell(command, ws, job_name, step_label, logger).await
        }
        StepDef::UrlCheck { url, expected_status, .. } => {
            execute_url_check(url, *expected_status, job_name, step_label, logger).await
        }
        StepDef::FileCheck { path, .. } => {
            execute_file_check(path, job_name, step_label, logger).await
        }
    }
}

// ── Agent step ────────────────────────────────────────────────────────────────

async fn execute_agent(
    agent: &str,
    workspace: &str,
    prompt: &str,
    flags: &[String],
    job_name: &str,
    step_label: &str,
    logger: &JobLogger,
) -> Result<()> {
    let runner = find_runner()?;
    let ws = shellexpand::tilde(workspace).to_string();
    let log_path = logger.path().to_string_lossy().to_string();

    info!(
        "[{}][{}] agent={} workspace={}",
        job_name, step_label, agent, ws
    );
    logger
        .write_line(&format!("[{}] agent={} workspace={}", step_label, agent, ws))
        .await;

    // The runner writes its own output to the log file via -l.
    // We only capture exit status here.
    let mut cmd = tokio::process::Command::new(&runner);
    cmd.arg("-l").arg(&log_path);
    cmd.arg(agent);
    cmd.arg(&ws);
    cmd.arg(prompt);
    for f in flags {
        cmd.arg(f);
    }
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    let status = cmd
        .status()
        .await
        .with_context(|| format!("spawning agent runner for step {}", step_label))?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "[{}][{}] agent runner exited {}",
            job_name,
            step_label,
            status
        )
    }
}

// ── Shell step ────────────────────────────────────────────────────────────────

async fn execute_shell(
    command: &str,
    workspace: &str,
    job_name: &str,
    step_label: &str,
    logger: &JobLogger,
) -> Result<()> {
    let ws = shellexpand::tilde(workspace).to_string();
    info!("[{}][{}] shell: {}", job_name, step_label, command);
    logger
        .write_line(&format!("[{}] $ {}", step_label, command))
        .await;

    let mut child = tokio::process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .current_dir(&ws)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("spawning bash for {:?}", command))?;

    // Stream stdout + stderr to log in real time
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");
    let log_out = logger.clone();
    let log_err = logger.clone();

    let t_out = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            log_out.write_line(&line).await;
        }
    });
    let t_err = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            log_err.write_line(&format!("stderr: {}", line)).await;
        }
    });

    let status = child.wait().await?;
    let _ = tokio::join!(t_out, t_err);

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("shell command exited {}", status)
    }
}

// ── URL-check step ────────────────────────────────────────────────────────────

async fn execute_url_check(
    url: &str,
    expected_status: Option<u16>,
    job_name: &str,
    step_label: &str,
    logger: &JobLogger,
) -> Result<()> {
    let want = expected_status.unwrap_or(200);
    info!(
        "[{}][{}] url-check: {} (expect {})",
        job_name, step_label, url, want
    );
    logger
        .write_line(&format!(
            "[{}] checking url: {} (expect {})",
            step_label, url, want
        ))
        .await;

    let resp = reqwest::get(url)
        .await
        .with_context(|| format!("GET {}", url))?;
    let got = resp.status().as_u16();

    if got == want {
        logger
            .write_line(&format!("[{}] url ok ({})", step_label, got))
            .await;
        Ok(())
    } else {
        anyhow::bail!("url returned {} (expected {})", got, want)
    }
}

// ── File-check step ───────────────────────────────────────────────────────────

async fn execute_file_check(
    path: &str,
    job_name: &str,
    step_label: &str,
    logger: &JobLogger,
) -> Result<()> {
    let expanded = shellexpand::tilde(path).to_string();
    info!("[{}][{}] file-check: {}", job_name, step_label, expanded);

    if std::path::Path::new(&expanded).exists() {
        logger
            .write_line(&format!("[{}] file exists: {}", step_label, expanded))
            .await;
        Ok(())
    } else {
        anyhow::bail!("file not found: {}", expanded)
    }
}

// ── Runner discovery ──────────────────────────────────────────────────────────

/// Find heartbeat-agent-runner.sh: binary dir → ~/.heartbeat/ → $PATH.
pub fn find_runner() -> Result<std::path::PathBuf> {
    // 1. Next to the running binary
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join(AGENT_RUNNER);
            if p.exists() {
                return Ok(p);
            }
        }
    }
    // 2. ~/.heartbeat/
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".into());
    let p = std::path::PathBuf::from(&home)
        .join(".heartbeat")
        .join(AGENT_RUNNER);
    if p.exists() {
        return Ok(p);
    }
    // 3. $PATH
    which::which(AGENT_RUNNER).with_context(|| {
        format!(
            "{} not found in PATH or ~/.heartbeat/",
            AGENT_RUNNER
        )
    })
}
