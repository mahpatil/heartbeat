use anyhow::{Context, Result};
use tracing::{error, info};

use super::types::TaskDef;

const AGENT_RUNNER: &str = "heartbeat-agent-runner.sh";

/// Execute a single task. Returns `Ok(())` on success, `Err` on failure.
pub async fn run_task(task: &TaskDef, folder: &str, job_name: &str) -> Result<()> {
    match task {
        TaskDef::Run { command } => run_shell(command, folder, job_name).await,
        TaskDef::Url { url, expected_status } => {
            check_url(url, *expected_status, job_name).await
        }
        TaskDef::FileExists { path } => {
            let expanded = shellexpand::tilde(path).to_string();
            if std::path::Path::new(&expanded).exists() {
                info!("[{}] file exists: {}", job_name, expanded);
                Ok(())
            } else {
                anyhow::bail!("[{}] file not found: {}", job_name, expanded)
            }
        }
        TaskDef::Agent { kind, prompt, cwd } => {
            let cwd = cwd.as_deref().unwrap_or(folder);
            run_agent(kind, cwd, prompt, job_name).await
        }
        TaskDef::AgentApi { provider, model, prompt } => {
            // Placeholder — implement SDK calls here
            info!(
                "[{}] agent_api ({}/{}) — not yet implemented",
                job_name, provider, model
            );
            let _ = prompt;
            Ok(())
        }
    }
}

async fn run_shell(command: &str, folder: &str, job_name: &str) -> Result<()> {
    info!("[{}] run: {}", job_name, command);
    let output = tokio::process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .current_dir(shellexpand::tilde(folder).as_ref())
        .output()
        .await
        .with_context(|| format!("spawning shell for `{}`", command))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("[{}] command failed: {}", job_name, stderr.trim());
        anyhow::bail!("command exited {}", output.status)
    }
}

async fn check_url(url: &str, expected: Option<u16>, job_name: &str) -> Result<()> {
    info!("[{}] checking url: {}", job_name, url);
    let resp = reqwest::get(url)
        .await
        .with_context(|| format!("GET {}", url))?;

    let status = resp.status().as_u16();
    let want = expected.unwrap_or(200);
    if status == want {
        info!("[{}] url ok ({})", job_name, status);
        Ok(())
    } else {
        anyhow::bail!("[{}] url returned {} (expected {})", job_name, status, want)
    }
}

async fn run_agent(kind: &str, cwd: &str, prompt: &str, job_name: &str) -> Result<()> {
    info!("[{}] agent ({}) cwd={}", job_name, kind, cwd);

    // Find the runner in $PATH or next to the binary
    let runner = find_runner()?;

    let output = tokio::process::Command::new(&runner)
        .arg(kind)
        .arg(cwd)
        .arg(prompt)
        .output()
        .await
        .with_context(|| format!("spawning {}", runner.display()))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("[{}] agent runner failed: {}", job_name, stderr.trim())
    }
}

fn find_runner() -> Result<std::path::PathBuf> {
    // 1. same directory as the running binary
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.parent().unwrap().join(AGENT_RUNNER);
        if sibling.exists() {
            return Ok(sibling);
        }
    }
    // 2. ~/.heartbeat/
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".into());
    let installed = std::path::PathBuf::from(home).join(".heartbeat").join(AGENT_RUNNER);
    if installed.exists() {
        return Ok(installed);
    }
    // 3. $PATH
    which::which(AGENT_RUNNER).with_context(|| format!("{} not found in PATH or ~/.heartbeat/", AGENT_RUNNER))
}
