use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use super::schedule::Schedule;
use crate::task::types::StepDef;

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct JobConfig {
    pub name: String,
    pub schedule: Schedule,
    /// Expanded working directory (tilde not yet expanded — done at runtime).
    pub workspace: String,
    pub steps: Vec<StepDef>,
    pub on_fail: Vec<String>,
}

// ── Raw YAML deserialization types ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct Frontmatter {
    name: Option<String>,
    schedule: Option<String>,
    workspace: Option<String>,
    /// Default agent for single-step shorthand (body-as-prompt).
    agent: Option<String>,
    /// Extra flags for single-step shorthand.
    #[serde(default)]
    flags: Vec<String>,
    #[serde(default)]
    on_fail: Vec<String>,
    /// Explicit multi-step pipeline (chained-steps spec, Milestone 3).
    #[serde(default)]
    steps: Vec<RawStep>,
}

#[derive(Debug, Deserialize, Clone)]
struct RawStep {
    name: Option<String>,
    #[serde(rename = "type")]
    kind: String,
    // agent step
    agent: Option<String>,
    prompt: Option<String>,
    #[serde(default)]
    flags: Vec<String>,
    workspace: Option<String>,
    // shell step
    command: Option<String>,
    // url-check step
    url: Option<String>,
    expected_status: Option<u16>,
    // file-check step
    path: Option<String>,
    // optional step-level timeout (e.g. "10m", "30s")
    timeout: Option<String>,
    // per-step environment variables
    #[serde(default)]
    env: HashMap<String, String>,
}

// ── impl JobConfig ─────────────────────────────────────────────────────────────

impl JobConfig {
    /// Load and parse a `.htb` file.
    pub fn load(path: &Path) -> Result<Self> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "htb" {
            bail!("not a .htb file: {}", path.display());
        }
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        Self::parse(&raw)
            .with_context(|| format!("parsing {}", path.display()))
    }

    fn parse(src: &str) -> Result<Self> {
        let (fm_str, body) = split_frontmatter(src)?;

        let fm: Frontmatter = serde_yaml::from_str(&fm_str)
            .context("invalid YAML frontmatter")?;

        // Required fields
        let name = fm
            .name
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("missing required field: name"))?;

        let schedule_str = fm
            .schedule
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("missing required field: schedule"))?;

        let schedule = Schedule::parse(&schedule_str)
            .with_context(|| format!("invalid schedule {:?}", schedule_str))?;

        let workspace = fm.workspace.unwrap_or_else(|| "~".to_string());

        // Build steps
        let default_agent = fm.agent.as_deref();
        let steps = if !fm.steps.is_empty() {
            // Explicit steps array
            fm.steps
                .iter()
                .map(|rs| raw_step_to_def(rs, default_agent))
                .collect::<Result<Vec<_>>>()?
        } else {
            // Body-as-prompt shorthand
            let prompt = body.trim().to_string();
            if prompt.is_empty() {
                bail!("job has no steps and no prompt body");
            }
            vec![StepDef::Agent {
                name: None,
                agent: fm
                    .agent
                    .unwrap_or_else(|| "claude".to_string()),
                prompt,
                flags: fm.flags,
                workspace: None,
                timeout: None,
                env: Default::default(),
            }]
        };

        Ok(JobConfig {
            name,
            schedule,
            workspace,
            steps,
            on_fail: fm.on_fail,
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Split a document into (frontmatter_yaml, body).
/// The document must start with `---` and have a closing `---`.
fn split_frontmatter(src: &str) -> Result<(String, String)> {
    let mut lines = src.lines();

    match lines.next() {
        Some(l) if l.trim() == "---" => {}
        _ => bail!("file does not start with ---"),
    }

    let mut fm_lines: Vec<&str> = Vec::new();
    let mut found_close = false;

    for line in lines.by_ref() {
        if line.trim() == "---" {
            found_close = true;
            break;
        }
        fm_lines.push(line);
    }

    if !found_close {
        bail!("missing closing --- in frontmatter");
    }

    let body: Vec<&str> = lines.collect();
    Ok((fm_lines.join("\n"), body.join("\n")))
}

fn parse_timeout(s: &str) -> Result<Duration> {
    humantime::parse_duration(s)
        .with_context(|| format!("invalid timeout {:?} (use e.g. \"10m\", \"30s\", \"1h\")", s))
}

fn raw_step_to_def(rs: &RawStep, default_agent: Option<&str>) -> Result<StepDef> {
    let timeout = rs
        .timeout
        .as_deref()
        .map(parse_timeout)
        .transpose()?;

    match rs.kind.as_str() {
        "agent" => {
            let agent = rs
                .agent
                .clone()
                .or_else(|| default_agent.map(str::to_string))
                .unwrap_or_else(|| "claude".to_string());
            let prompt = rs
                .prompt
                .clone()
                .ok_or_else(|| anyhow::anyhow!("agent step missing 'prompt'"))?;
            Ok(StepDef::Agent {
                name: rs.name.clone(),
                agent,
                prompt,
                flags: rs.flags.clone(),
                workspace: rs.workspace.clone(),
                timeout,
                env: rs.env.clone(),
            })
        }
        "shell" => {
            let command = rs
                .command
                .clone()
                .ok_or_else(|| anyhow::anyhow!("shell step missing 'command'"))?;
            Ok(StepDef::Shell {
                name: rs.name.clone(),
                command,
                workspace: rs.workspace.clone(),
                timeout,
                env: rs.env.clone(),
            })
        }
        "url-check" => {
            let url = rs
                .url
                .clone()
                .ok_or_else(|| anyhow::anyhow!("url-check step missing 'url'"))?;
            Ok(StepDef::UrlCheck {
                name: rs.name.clone(),
                url,
                expected_status: rs.expected_status,
            })
        }
        "file-check" => {
            let path = rs
                .path
                .clone()
                .ok_or_else(|| anyhow::anyhow!("file-check step missing 'path'"))?;
            Ok(StepDef::FileCheck {
                name: rs.name.clone(),
                path,
            })
        }
        other => bail!("unknown step type: {}", other),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_body_as_prompt() {
        let src = "---\nname: test\nschedule: every 5m\n---\nHello agent\n";
        let cfg = JobConfig::parse(src).unwrap();
        assert_eq!(cfg.name, "test");
        assert_eq!(cfg.steps.len(), 1);
        match &cfg.steps[0] {
            StepDef::Agent { prompt, agent, .. } => {
                assert_eq!(prompt, "Hello agent");
                assert_eq!(agent, "claude");
            }
            _ => panic!("expected Agent step"),
        }
    }

    #[test]
    fn missing_closing_delimiter() {
        let src = "---\nname: test\nschedule: every 5m\n";
        assert!(JobConfig::parse(src).is_err());
    }

    #[test]
    fn missing_name_field() {
        let src = "---\nschedule: every 5m\n---\nprompt\n";
        let err = JobConfig::parse(src).unwrap_err().to_string();
        assert!(err.contains("missing required field: name"), "got: {}", err);
    }

    #[test]
    fn empty_body_no_steps_is_error() {
        let src = "---\nname: test\nschedule: every 5m\n---\n";
        let err = JobConfig::parse(src).unwrap_err().to_string();
        assert!(err.contains("no steps and no prompt body"), "got: {}", err);
    }

    #[test]
    fn workspace_defaults_to_tilde() {
        let src = "---\nname: test\nschedule: every 5m\n---\nhello\n";
        let cfg = JobConfig::parse(src).unwrap();
        assert_eq!(cfg.workspace, "~");
    }

    #[test]
    fn parse_explicit_steps() {
        let src = "\
---
name: pipeline
schedule: every 1h
steps:
  - name: check
    type: shell
    command: echo hi
---
";
        let cfg = JobConfig::parse(src).unwrap();
        assert_eq!(cfg.steps.len(), 1);
        assert!(matches!(&cfg.steps[0], StepDef::Shell { .. }));
    }

    // ── Chained-steps (M3) ────────────────────────────────────────────────────

    #[test]
    fn parse_multi_step_pipeline() {
        let src = "\
---
name: nightly
schedule: daily at 01:30
workspace: ~/projects/myapp
steps:
  - name: run-tests
    type: shell
    command: cargo test
  - name: summarise
    type: agent
    agent: claude
    prompt: Summarise the test output.
  - name: health-check
    type: url-check
    url: https://example.com/health
---
";
        let cfg = JobConfig::parse(src).unwrap();
        assert_eq!(cfg.steps.len(), 3);
        assert!(matches!(&cfg.steps[0], StepDef::Shell { name: Some(n), .. } if n == "run-tests"));
        assert!(matches!(&cfg.steps[1], StepDef::Agent { name: Some(n), agent, .. }
            if n == "summarise" && agent == "claude"));
        assert!(matches!(&cfg.steps[2], StepDef::UrlCheck { name: Some(n), .. } if n == "health-check"));
    }

    #[test]
    fn step_agent_override() {
        let src = "\
---
name: multi-agent
schedule: every 1h
agent: claude
steps:
  - name: step1
    type: agent
    prompt: Do something with claude.
  - name: step2
    type: agent
    agent: opencode
    prompt: Fix the issues.
---
";
        let cfg = JobConfig::parse(src).unwrap();
        assert_eq!(cfg.steps.len(), 2);
        // step1 inherits job-level agent
        match &cfg.steps[0] {
            StepDef::Agent { agent, .. } => assert_eq!(agent, "claude"),
            _ => panic!("expected Agent"),
        }
        // step2 uses its own agent override
        match &cfg.steps[1] {
            StepDef::Agent { agent, .. } => assert_eq!(agent, "opencode"),
            _ => panic!("expected Agent"),
        }
    }

    #[test]
    fn step_workspace_override() {
        let src = "\
---
name: ws-test
schedule: every 5m
workspace: ~/projects/myapp
steps:
  - name: backend
    type: shell
    command: cargo test
    workspace: ~/projects/myapp/backend
  - name: frontend
    type: shell
    command: npm test
---
";
        let cfg = JobConfig::parse(src).unwrap();
        // step with explicit workspace
        match &cfg.steps[0] {
            StepDef::Shell { workspace: Some(ws), .. } => {
                assert_eq!(ws, "~/projects/myapp/backend");
            }
            _ => panic!("expected Shell with workspace"),
        }
        // step without workspace (inherits at runtime)
        match &cfg.steps[1] {
            StepDef::Shell { workspace: None, .. } => {}
            _ => panic!("expected Shell with no workspace"),
        }
    }

    #[test]
    fn unknown_step_type_error() {
        let src = "\
---
name: bad
schedule: every 5m
steps:
  - type: banana
    command: echo hi
---
";
        let err = JobConfig::parse(src).unwrap_err().to_string();
        assert!(err.contains("unknown step type: banana"), "got: {}", err);
    }

    #[test]
    fn agent_step_inherits_job_agent() {
        let src = "\
---
name: inherit-test
schedule: every 5m
agent: opencode
steps:
  - type: agent
    prompt: Do something.
---
";
        let cfg = JobConfig::parse(src).unwrap();
        match &cfg.steps[0] {
            StepDef::Agent { agent, .. } => assert_eq!(agent, "opencode"),
            _ => panic!("expected Agent"),
        }
    }

    #[test]
    fn agent_step_defaults_to_claude_when_no_job_agent() {
        let src = "\
---
name: default-agent
schedule: every 5m
steps:
  - type: agent
    prompt: Do something.
---
";
        let cfg = JobConfig::parse(src).unwrap();
        match &cfg.steps[0] {
            StepDef::Agent { agent, .. } => assert_eq!(agent, "claude"),
            _ => panic!("expected Agent"),
        }
    }

    #[test]
    fn url_check_and_file_check_steps() {
        let src = "\
---
name: checks
schedule: every 10m
steps:
  - name: url
    type: url-check
    url: https://example.com
    expected_status: 200
  - name: presence
    type: file-check
    path: /tmp/marker.txt
---
";
        let cfg = JobConfig::parse(src).unwrap();
        assert_eq!(cfg.steps.len(), 2);
        match &cfg.steps[0] {
            StepDef::UrlCheck { url, expected_status: Some(200), .. } => {
                assert_eq!(url, "https://example.com");
            }
            _ => panic!("expected UrlCheck(200)"),
        }
        match &cfg.steps[1] {
            StepDef::FileCheck { path, .. } => assert_eq!(path, "/tmp/marker.txt"),
            _ => panic!("expected FileCheck"),
        }
    }

    #[test]
    fn on_fail_commands_parsed() {
        let src = "\
---
name: with-on-fail
schedule: every 5m
on_fail:
  - notify-slack.sh \"pipeline failed\"
  - echo fallback
steps:
  - type: shell
    command: cargo test
---
";
        let cfg = JobConfig::parse(src).unwrap();
        assert_eq!(cfg.on_fail.len(), 2);
        assert!(cfg.on_fail[0].contains("notify-slack.sh"));
    }

    #[test]
    fn agent_step_missing_prompt_error() {
        let src = "\
---
name: bad-agent
schedule: every 5m
steps:
  - type: agent
    agent: claude
---
";
        let err = JobConfig::parse(src).unwrap_err().to_string();
        assert!(err.contains("missing 'prompt'"), "got: {}", err);
    }

    #[test]
    fn shell_step_missing_command_error() {
        let src = "\
---
name: bad-shell
schedule: every 5m
steps:
  - type: shell
---
";
        let err = JobConfig::parse(src).unwrap_err().to_string();
        assert!(err.contains("missing 'command'"), "got: {}", err);
    }

    // ── Step timeout parsing ──────────────────────────────────────────────────

    #[test]
    fn step_timeout_minutes_parsed() {
        let src = "\
---
name: timed
schedule: every 5m
steps:
  - type: shell
    command: echo hi
    timeout: 10m
---
";
        let cfg = JobConfig::parse(src).unwrap();
        match &cfg.steps[0] {
            StepDef::Shell { timeout: Some(d), .. } => {
                assert_eq!(d.as_secs(), 600);
            }
            _ => panic!("expected Shell with timeout"),
        }
    }

    #[test]
    fn step_timeout_seconds_parsed() {
        let src = "\
---
name: timed
schedule: every 5m
steps:
  - type: shell
    command: echo hi
    timeout: 30s
---
";
        let cfg = JobConfig::parse(src).unwrap();
        match &cfg.steps[0] {
            StepDef::Shell { timeout: Some(d), .. } => {
                assert_eq!(d.as_secs(), 30);
            }
            _ => panic!("expected Shell with timeout"),
        }
    }

    #[test]
    fn step_timeout_hours_parsed() {
        let src = "\
---
name: timed
schedule: every 5m
steps:
  - type: shell
    command: echo hi
    timeout: 1h
---
";
        let cfg = JobConfig::parse(src).unwrap();
        match &cfg.steps[0] {
            StepDef::Shell { timeout: Some(d), .. } => {
                assert_eq!(d.as_secs(), 3600);
            }
            _ => panic!("expected Shell with timeout"),
        }
    }

    #[test]
    fn step_timeout_absent_is_none() {
        let src = "\
---
name: notimed
schedule: every 5m
steps:
  - type: shell
    command: echo hi
---
";
        let cfg = JobConfig::parse(src).unwrap();
        match &cfg.steps[0] {
            StepDef::Shell { timeout: None, .. } => {}
            _ => panic!("expected Shell with no timeout"),
        }
    }

    // ── Per-step env vars ─────────────────────────────────────────────────────

    #[test]
    fn env_vars_parsed_from_yaml() {
        let src = "\
---
name: env-test
schedule: every 5m
steps:
  - type: shell
    command: echo hi
    env:
      FOO: bar
      NUM: \"42\"
---
";
        let cfg = JobConfig::parse(src).unwrap();
        match &cfg.steps[0] {
            StepDef::Shell { env, .. } => {
                assert_eq!(env.get("FOO").map(String::as_str), Some("bar"));
                assert_eq!(env.get("NUM").map(String::as_str), Some("42"));
            }
            _ => panic!("expected Shell step"),
        }
    }

    #[test]
    fn env_absent_is_empty_map() {
        let src = "\
---
name: no-env
schedule: every 5m
steps:
  - type: shell
    command: echo hi
---
";
        let cfg = JobConfig::parse(src).unwrap();
        match &cfg.steps[0] {
            StepDef::Shell { env, .. } => assert!(env.is_empty()),
            _ => panic!("expected Shell step"),
        }
    }

    #[test]
    fn step_timeout_invalid_value_is_error() {
        let src = "\
---
name: bad-timeout
schedule: every 5m
steps:
  - type: shell
    command: echo hi
    timeout: banana
---
";
        let err = JobConfig::parse(src).unwrap_err().to_string();
        assert!(err.contains("invalid timeout"), "got: {}", err);
    }
}
