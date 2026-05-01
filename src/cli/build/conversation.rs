//! Interactive confirm-and-fill dialog that turns a [`ParsedIntent`] into a
//! [`JobDraft`] ready for rendering.

use anyhow::Result;

use crate::cli::new::{JobDraft, StepDraft};
use crate::cli::prompts::{ask_confirm, ask_optional, ask_select, ask_string};

use super::parser::{ParsedIntent, ParsedStep};

// ── Schedule presets (mirrored from wizard so we stay DRY without coupling) ──

const SCHEDULE_PRESETS: &[(&str, &str)] = &[
    ("Every 5 minutes", "every 5m"),
    ("Every 15 minutes", "every 15m"),
    ("Every 30 minutes", "every 30m"),
    ("Every hour", "every 1h"),
    ("Every 6 hours", "every 6h"),
    ("Every day at 9am", "daily at 09:00"),
    ("Every day at midnight", "daily at 00:00"),
    ("Every week", "every 7d"),
    ("Custom (type your own)", ""),
];

// ── Public entry point ────────────────────────────────────────────────────────

/// Display what was parsed, confirm with the user, fill any gaps, return a
/// complete [`JobDraft`].
pub fn confirm_and_fill(intent: ParsedIntent, suggested_name: &str) -> Result<JobDraft> {
    print_summary(&intent, suggested_name);

    let all_present = intent.schedule.is_some() && !intent.steps.is_empty();

    if all_present && ask_confirm("Does this look right?", true)? {
        let name = ask_string("Job name", Some(suggested_name))?;
        return Ok(draft_from(name, intent));
    }

    println!("\n  Let's fill in the details:\n");

    let name = ask_string("Job name (slug)", Some(suggested_name))?;
    let schedule = resolve_schedule(intent.schedule)?;
    let workspace = resolve_workspace(intent.workspace)?;
    let steps = if intent.steps.is_empty() {
        collect_steps_interactively()?
    } else {
        println!();
        if ask_confirm("Keep the detected steps?", true)? {
            intent.steps.into_iter().map(to_step_draft).collect()
        } else {
            collect_steps_interactively()?
        }
    };

    Ok(JobDraft { name, schedule, workspace, steps, on_fail: vec![] })
}

// ── Summary display ───────────────────────────────────────────────────────────

fn print_summary(intent: &ParsedIntent, suggested_name: &str) {
    println!("\n  Here's what I understood:\n");
    println!("  Name:      {}", suggested_name);
    println!(
        "  Schedule:  {}",
        intent.schedule.as_deref().unwrap_or("(not detected — will ask)")
    );
    println!(
        "  Workspace: {}",
        intent.workspace.as_deref().unwrap_or("~")
    );

    if intent.steps.is_empty() {
        println!("  Steps:     (none detected — will ask)");
    } else {
        println!("  Steps:");
        for (i, step) in intent.steps.iter().enumerate() {
            println!("    {}. {}", i + 1, step.summary());
        }
    }
    println!();
}

// ── Field resolvers ───────────────────────────────────────────────────────────

fn resolve_schedule(detected: Option<String>) -> Result<String> {
    match detected {
        Some(s) => {
            if ask_confirm(&format!("Schedule detected: \"{}\" — keep it?", s), true)? {
                Ok(s)
            } else {
                ask_schedule_interactively()
            }
        }
        None => ask_schedule_interactively(),
    }
}

fn ask_schedule_interactively() -> Result<String> {
    let labels: Vec<&str> = SCHEDULE_PRESETS.iter().map(|(l, _)| *l).collect();
    let idx = ask_select("Schedule", &labels)?;
    let (_, preset) = SCHEDULE_PRESETS[idx];
    if preset.is_empty() {
        ask_string(
            "Enter schedule (e.g. \"every 5m\", \"daily at 08:00\", cron \"0 9 * * 1\")",
            None,
        )
    } else {
        Ok(preset.to_string())
    }
}

fn resolve_workspace(detected: Option<String>) -> Result<String> {
    match detected {
        Some(w) => {
            if ask_confirm(&format!("Workspace: \"{}\" — keep it?", w), true)? {
                Ok(w)
            } else {
                Ok(ask_optional("Working directory", "leave blank for ~")?
                    .unwrap_or_else(|| "~".to_string()))
            }
        }
        None => Ok(ask_optional("Working directory", "leave blank for ~")?
            .unwrap_or_else(|| "~".to_string())),
    }
}

// ── Step collection fallback ──────────────────────────────────────────────────

fn collect_steps_interactively() -> Result<Vec<StepDraft>> {
    let agents = ["claude", "opencode", "codex"];
    let kinds = [
        "Agent (ask Claude / opencode)",
        "Shell command",
        "URL check",
        "File check",
    ];
    let mut steps = Vec::new();
    loop {
        println!();
        let kind_idx = ask_select(
            &format!("Step {} — type", steps.len() + 1),
            &kinds,
        )?;
        let step = match kind_idx {
            0 => {
                let agent_idx = ask_select("Agent", &agents)?;
                let prompt = ask_string("Prompt (what should the agent do?)", None)?;
                StepDraft::Agent {
                    name: None,
                    agent: agents[agent_idx].to_string(),
                    prompt,
                    flags: vec![],
                }
            }
            1 => StepDraft::Shell {
                name: None,
                command: ask_string("Shell command", None)?,
            },
            2 => StepDraft::UrlCheck {
                name: None,
                url: ask_string("URL to check", None)?,
                expected_status: None,
            },
            3 => StepDraft::FileCheck {
                name: None,
                path: ask_string("File path to check", None)?,
            },
            _ => unreachable!(),
        };
        steps.push(step);
        if !ask_confirm("Add another step?", false)? {
            break;
        }
    }
    Ok(steps)
}

// ── Conversion helpers ────────────────────────────────────────────────────────

fn to_step_draft(step: ParsedStep) -> StepDraft {
    match step {
        ParsedStep::Agent { agent, prompt } => {
            StepDraft::Agent { name: None, agent, prompt, flags: vec![] }
        }
        ParsedStep::Shell { command } => StepDraft::Shell { name: None, command },
        ParsedStep::UrlCheck { url } => {
            StepDraft::UrlCheck { name: None, url, expected_status: None }
        }
        ParsedStep::FileCheck { path } => StepDraft::FileCheck { name: None, path },
    }
}

fn draft_from(name: String, intent: ParsedIntent) -> JobDraft {
    JobDraft {
        name,
        schedule: intent.schedule.unwrap_or_else(|| "every 1h".to_string()),
        workspace: intent.workspace.unwrap_or_else(|| "~".to_string()),
        steps: intent.steps.into_iter().map(to_step_draft).collect(),
        on_fail: vec![],
    }
}
