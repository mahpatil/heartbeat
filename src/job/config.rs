/// Parsed job definition loaded from a `.yaml` or `.htb` file.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::task::types::TaskDef;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JobConfig {
    pub name: String,
    /// Working directory for tasks (defaults to `~`)
    #[serde(default)]
    pub folder: Option<String>,
    /// Cron expression, e.g. `"*/5 * * * *"`
    pub frequency: Option<String>,
    /// ISO-8601 datetime — run exactly once at this time
    pub run_once_at: Option<String>,
    pub tasks: Vec<TaskDef>,
    #[serde(default)]
    pub on_fail: Vec<String>,
}

impl JobConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "yaml" | "yml" => Self::load_yaml(path),
            "htb" => Self::load_htb(path),
            other => anyhow::bail!("Unsupported job file extension: {}", other),
        }
    }

    fn load_yaml(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        serde_yaml::from_str(&raw)
            .with_context(|| format!("parsing YAML {}", path.display()))
    }

    /// Minimal natural-language `.htb` parser (subset of Python version).
    fn load_htb(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        parse_htb(&raw, path)
    }
}

fn parse_htb(src: &str, path: &Path) -> Result<JobConfig> {
    use crate::task::types::TaskDef;

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("job")
        .to_string();

    let mut name = stem.clone();
    let mut folder: Option<String> = None;
    let mut frequency: Option<String> = None;
    let mut run_once_at: Option<String> = None;
    let mut tasks: Vec<TaskDef> = Vec::new();
    let mut on_fail: Vec<String> = Vec::new();

    for line in src.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("# Heartbeat:") {
            name = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("# Folder:") {
            folder = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("# Frequency:") {
            frequency = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("# Run once at:") {
            run_once_at = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("URL reachable:") {
            tasks.push(TaskDef::Url { url: v.trim().to_string(), expected_status: None });
        } else if let Some(v) = line.strip_prefix("File exists:") {
            tasks.push(TaskDef::FileExists { path: v.trim().to_string() });
        } else if let Some(v) = line.strip_prefix("Run:") {
            tasks.push(TaskDef::Run { command: v.trim().to_string() });
        } else if let Some(v) = line.strip_prefix("Ask Claude:") {
            tasks.push(TaskDef::Agent {
                kind: "claude".to_string(),
                prompt: v.trim().to_string(),
                cwd: None,
            });
        } else if let Some(v) = line.strip_prefix("On fail:") {
            on_fail.push(v.trim().to_string());
        }
    }

    Ok(JobConfig { name, folder, frequency, run_once_at, tasks, on_fail })
}
