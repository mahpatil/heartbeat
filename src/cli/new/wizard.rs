//! Interactive wizard: collects job metadata and steps from the user.

use anyhow::Result;

use super::prompts::{ask_confirm, ask_optional, ask_select, ask_string};

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct JobDraft {
    pub name: String,
    pub schedule: String,
    pub workspace: String,
    pub steps: Vec<StepDraft>,
    pub on_fail: Vec<String>,
}

#[derive(Debug)]
pub enum StepDraft {
    Agent {
        name: Option<String>,
        agent: String,
        prompt: String,
        flags: Vec<String>,
    },
    Shell {
        name: Option<String>,
        command: String,
    },
    UrlCheck {
        name: Option<String>,
        url: String,
        expected_status: Option<u16>,
    },
    FileCheck {
        name: Option<String>,
        path: String,
    },
}

// ── Schedule presets ──────────────────────────────────────────────────────────

const SCHEDULE_PRESETS: &[(&str, &str)] = &[
    ("Every 5 minutes", "every 5m"),
    ("Every 15 minutes", "every 15m"),
    ("Every 30 minutes", "every 30m"),
    ("Every hour", "every 1h"),
    ("Every 6 hours", "every 6h"),
    ("Every day at midnight", "daily at 00:00"),
    ("Every day at 9am", "daily at 09:00"),
    ("Every week", "every 7d"),
    ("Custom (type your own)", ""),
];

// ── Public entry point ────────────────────────────────────────────────────────

pub fn run() -> Result<JobDraft> {
    println!("\n  Heartbeat Job Wizard\n  {}\n", "─".repeat(40));

    let name = collect_name()?;
    let workspace = collect_workspace()?;
    let schedule = collect_schedule()?;
    let steps = collect_steps()?;
    let on_fail = collect_on_fail()?;

    Ok(JobDraft { name, schedule, workspace, steps, on_fail })
}

// ── Section collectors ────────────────────────────────────────────────────────

fn collect_name() -> Result<String> {
    ask_string("Job name (slug, e.g. daily-digest)", None)
}

fn collect_workspace() -> Result<String> {
    Ok(ask_optional("Working directory", "leave blank for ~")?
        .unwrap_or_else(|| "~".to_string()))
}

fn collect_schedule() -> Result<String> {
    let labels: Vec<&str> = SCHEDULE_PRESETS.iter().map(|(l, _)| *l).collect();
    let idx = ask_select("Schedule", &labels)?;
    let (_, preset) = SCHEDULE_PRESETS[idx];
    if preset.is_empty() {
        // custom
        ask_string(
            "Enter schedule (e.g. \"every 5m\", \"daily at 08:00\", cron \"0 9 * * 1\")",
            None,
        )
    } else {
        Ok(preset.to_string())
    }
}

fn collect_steps() -> Result<Vec<StepDraft>> {
    let mut steps: Vec<StepDraft> = Vec::new();
    loop {
        println!();
        let step = collect_one_step(steps.len() + 1)?;
        steps.push(step);
        if !ask_confirm("Add another step?", false)? {
            break;
        }
    }
    Ok(steps)
}

fn collect_one_step(idx: usize) -> Result<StepDraft> {
    let kinds = ["Agent (ask Claude / opencode)", "Shell command", "URL check", "File check"];
    let kind_idx = ask_select(&format!("Step {} — type", idx), &kinds)?;
    match kind_idx {
        0 => collect_agent_step(),
        1 => collect_shell_step(),
        2 => collect_url_step(),
        3 => collect_file_step(),
        _ => unreachable!(),
    }
}

fn collect_agent_step() -> Result<StepDraft> {
    let name = ask_optional("Step name", "optional")?;
    let agents = ["claude", "opencode", "codex"];
    let agent_idx = ask_select("Agent", &agents)?;
    let agent = agents[agent_idx].to_string();
    let prompt = ask_string("Prompt (what should the agent do?)", None)?;
    let flags_raw = ask_optional("Extra flags", "e.g. --no-cache  or leave blank")?;
    let flags = flags_raw
        .map(|s| s.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default();
    Ok(StepDraft::Agent { name, agent, prompt, flags })
}

fn collect_shell_step() -> Result<StepDraft> {
    let name = ask_optional("Step name", "optional")?;
    let command = ask_string("Shell command", None)?;
    Ok(StepDraft::Shell { name, command })
}

fn collect_url_step() -> Result<StepDraft> {
    let name = ask_optional("Step name", "optional")?;
    let url = ask_string("URL to check", None)?;
    let status_str = ask_optional("Expected HTTP status", "leave blank for any 2xx")?;
    let expected_status = status_str.and_then(|s| s.parse::<u16>().ok());
    Ok(StepDraft::UrlCheck { name, url, expected_status })
}

fn collect_file_step() -> Result<StepDraft> {
    let name = ask_optional("Step name", "optional")?;
    let path = ask_string("File path to check", None)?;
    Ok(StepDraft::FileCheck { name, path })
}

fn collect_on_fail() -> Result<Vec<String>> {
    if !ask_confirm("\nAdd on-fail commands (e.g. notify script)?", false)? {
        return Ok(vec![]);
    }
    let mut cmds = Vec::new();
    loop {
        let cmd = ask_string("On-fail command", None)?;
        cmds.push(cmd);
        if !ask_confirm("Add another?", false)? {
            break;
        }
    }
    Ok(cmds)
}
