//! Serialize a collected job definition into `.htb` file content.

use super::wizard::{JobDraft, StepDraft};

pub fn render(draft: &JobDraft) -> String {
    let mut lines: Vec<String> = Vec::new();

    // ── Frontmatter ───────────────────────────────────────────────────────────
    lines.push("---".to_string());
    lines.push(format!("name: {}", draft.name));
    lines.push(format!("schedule: {}", draft.schedule));

    if draft.workspace != "~" {
        lines.push(format!("workspace: {}", draft.workspace));
    }

    if !draft.on_fail.is_empty() {
        lines.push("on_fail:".to_string());
        for cmd in &draft.on_fail {
            lines.push(format!("  - {}", cmd));
        }
    }

    // Determine if we can use body-as-prompt shorthand (single agent step)
    let use_shorthand = draft.steps.len() == 1
        && matches!(&draft.steps[0], StepDraft::Agent { agent, .. } if agent == "claude");

    if !use_shorthand {
        lines.push("steps:".to_string());
        for step in &draft.steps {
            render_step(&mut lines, step);
        }
    }

    lines.push("---".to_string());

    // ── Body (shorthand prompt or blank) ─────────────────────────────────────
    if use_shorthand
        && let StepDraft::Agent { prompt, .. } = &draft.steps[0]
    {
        lines.push(String::new());
        lines.push(prompt.clone());
    }

    lines.push(String::new());
    lines.join("\n")
}

fn render_step(lines: &mut Vec<String>, step: &StepDraft) {
    match step {
        StepDraft::Agent { name, agent, prompt, flags } => {
            lines.push("  - type: agent".to_string());
            if let Some(n) = name {
                lines.push(format!("    name: {}", n));
            }
            lines.push(format!("    agent: {}", agent));
            lines.push(format!("    prompt: \"{}\"", prompt.replace('"', "\\\"")));
            if !flags.is_empty() {
                lines.push("    flags:".to_string());
                for f in flags {
                    lines.push(format!("      - {}", f));
                }
            }
        }
        StepDraft::Shell { name, command } => {
            lines.push("  - type: shell".to_string());
            if let Some(n) = name {
                lines.push(format!("    name: {}", n));
            }
            lines.push(format!("    command: {}", command));
        }
        StepDraft::UrlCheck { name, url, expected_status } => {
            lines.push("  - type: url-check".to_string());
            if let Some(n) = name {
                lines.push(format!("    name: {}", n));
            }
            lines.push(format!("    url: {}", url));
            if let Some(s) = expected_status {
                lines.push(format!("    expected_status: {}", s));
            }
        }
        StepDraft::FileCheck { name, path } => {
            lines.push("  - type: file-check".to_string());
            if let Some(n) = name {
                lines.push(format!("    name: {}", n));
            }
            lines.push(format!("    path: {}", path));
        }
    }
}
