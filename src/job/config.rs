use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::Path;

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

fn raw_step_to_def(rs: &RawStep, default_agent: Option<&str>) -> Result<StepDef> {
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
}
