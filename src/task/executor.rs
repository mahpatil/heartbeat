use anyhow::{Context, Result};
use std::process::Stdio;
use std::time::Duration;
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
        StepDef::Agent { agent, prompt, flags, workspace, timeout, .. } => {
            let ws = workspace.as_deref().unwrap_or(job_workspace);
            execute_agent(agent, ws, prompt, flags, *timeout, job_name, step_label, logger).await
        }
        StepDef::Shell { command, workspace, timeout, .. } => {
            let ws = workspace.as_deref().unwrap_or(job_workspace);
            execute_shell(command, ws, *timeout, job_name, step_label, logger).await
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
    timeout: Option<Duration>,
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

    let mut child = cmd
        .spawn()
        .with_context(|| format!("spawning agent runner for step {}", step_label))?;

    let status = match timeout {
        None => child.wait().await?,
        Some(limit) => {
            match tokio::time::timeout(limit, child.wait()).await {
                Ok(Ok(s)) => s,
                Ok(Err(e)) => return Err(e.into()),
                Err(_) => {
                    let _ = child.kill().await;
                    anyhow::bail!(
                        "step timed out after {}s",
                        limit.as_secs()
                    )
                }
            }
        }
    };

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
    timeout: Option<Duration>,
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

    let status = match timeout {
        None => child.wait().await?,
        Some(limit) => {
            match tokio::time::timeout(limit, child.wait()).await {
                Ok(Ok(s)) => s,
                Ok(Err(e)) => return Err(e.into()),
                Err(_) => {
                    let _ = child.kill().await;
                    let _ = tokio::join!(t_out, t_err);
                    anyhow::bail!("step timed out after {}s", limit.as_secs())
                }
            }
        }
    };

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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::types::StepDef;
    use tempfile::TempDir;

    async fn test_logger(dir: &TempDir) -> JobLogger {
        JobLogger::new("test-job", dir.path()).await.unwrap()
    }

    // ── Shell step ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_shell_step_success() {
        let dir = TempDir::new().unwrap();
        let logger = test_logger(&dir).await;
        let step = StepDef::Shell {
            name: Some("echo".into()),
            command: "echo hello".into(),
            workspace: None,
            timeout: None,
        };
        let result = execute_step(&step, "/tmp", "test-job", "echo", &logger).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_shell_step_failure() {
        let dir = TempDir::new().unwrap();
        let logger = test_logger(&dir).await;
        let step = StepDef::Shell {
            name: None,
            command: "exit 1".into(),
            workspace: None,
            timeout: None,
        };
        let result = execute_step(&step, "/tmp", "test-job", "step[0]", &logger).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exited"));
    }

    #[tokio::test]
    async fn test_shell_step_streams_output_to_log() {
        let dir = TempDir::new().unwrap();
        let logger = test_logger(&dir).await;
        let step = StepDef::Shell {
            name: Some("greet".into()),
            command: "echo 'hello from step'".into(),
            workspace: None,
            timeout: None,
        };
        execute_step(&step, "/tmp", "test-job", "greet", &logger).await.unwrap();

        let log_content = tokio::fs::read_to_string(logger.path()).await.unwrap();
        assert!(log_content.contains("hello from step"), "log: {}", log_content);
    }

    #[tokio::test]
    async fn test_shell_step_workspace_override() {
        let dir = TempDir::new().unwrap();
        let logger = test_logger(&dir).await;
        let workspace = dir.path().to_str().unwrap().to_string();
        let step = StepDef::Shell {
            name: None,
            command: "pwd".into(),
            workspace: Some(workspace.clone()),
            timeout: None,
        };
        let result = execute_step(&step, "/tmp", "test-job", "step[0]", &logger).await;
        assert!(result.is_ok());

        let log_content = tokio::fs::read_to_string(logger.path()).await.unwrap();
        assert!(log_content.contains(&workspace), "log: {}", log_content);
    }

    // ── Step timeout ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_shell_step_killed_on_timeout() {
        let dir = TempDir::new().unwrap();
        let logger = test_logger(&dir).await;
        let step = StepDef::Shell {
            name: None,
            command: "sleep 60".into(),
            workspace: None,
            timeout: Some(Duration::from_secs(1)),
        };

        let start = std::time::Instant::now();
        let result = execute_step(&step, "/tmp", "test-job", "step[0]", &logger).await;
        let elapsed = start.elapsed();

        assert!(result.is_err(), "expected timeout error");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("timed out"), "error should mention timeout: {}", msg);
        assert!(elapsed < Duration::from_secs(3), "took too long: {:?}", elapsed);
    }

    #[tokio::test]
    async fn test_shell_step_completes_before_timeout() {
        let dir = TempDir::new().unwrap();
        let logger = test_logger(&dir).await;
        let step = StepDef::Shell {
            name: None,
            command: "echo hello".into(),
            workspace: None,
            timeout: Some(Duration::from_secs(10)),
        };
        let result = execute_step(&step, "/tmp", "test-job", "step[0]", &logger).await;
        assert!(result.is_ok());
    }

    // ── File-check step ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_file_check_found() {
        let dir = TempDir::new().unwrap();
        let marker = dir.path().join("marker.txt");
        std::fs::write(&marker, b"ok").unwrap();
        let logger = test_logger(&dir).await;

        let step = StepDef::FileCheck {
            name: Some("check-marker".into()),
            path: marker.to_str().unwrap().to_string(),
        };
        let result = execute_step(&step, "/tmp", "test-job", "check-marker", &logger).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_file_check_missing() {
        let dir = TempDir::new().unwrap();
        let logger = test_logger(&dir).await;
        let step = StepDef::FileCheck {
            name: None,
            path: "/nonexistent/path/file.txt".into(),
        };
        let result = execute_step(&step, "/tmp", "test-job", "step[0]", &logger).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("file not found"));
    }

    // ── display_name ──────────────────────────────────────────────────────────

    #[test]
    fn display_name_indexed() {
        let step = StepDef::Shell { name: None, command: "echo".into(), workspace: None, timeout: None };
        assert_eq!(step.display_name(2), "step[2]");
    }

    #[test]
    fn display_name_named() {
        let step = StepDef::Shell {
            name: Some("run-tests".into()),
            command: "cargo test".into(),
            workspace: None,
            timeout: None,
        };
        assert_eq!(step.display_name(0), "run-tests");
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
