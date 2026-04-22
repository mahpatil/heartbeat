//! `heartbeat new` — interactive wizard or non-interactive flags that generate a `.htb` job file.

mod prompts;
mod render;
mod wizard;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use self::render::render;
use self::wizard::{JobDraft, StepDraft, run as run_wizard};
use crate::job::schedule::Schedule;

// ── Non-interactive args (agent / CI friendly) ────────────────────────────────

pub struct NonInteractiveArgs {
    pub name: String,
    pub schedule: String,
    pub prompt: String,
    pub agent: String,
    pub workspace: Option<String>,
    pub extra_flags: Vec<String>,
}

fn build_draft(args: NonInteractiveArgs) -> Result<JobDraft> {
    // Validate the schedule string before writing anything to disk.
    Schedule::parse(&args.schedule)
        .with_context(|| format!("invalid schedule: {:?}", args.schedule))?;

    Ok(JobDraft {
        name: args.name,
        schedule: args.schedule,
        workspace: args.workspace.unwrap_or_else(|| "~".to_string()),
        steps: vec![StepDraft::Agent {
            name: None,
            agent: args.agent,
            prompt: args.prompt,
            flags: args.extra_flags,
        }],
        on_fail: vec![],
    })
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Entry point called from `main.rs`.
pub async fn run(
    output: Option<PathBuf>,
    apply_dir: Option<&Path>,
    non_interactive: Option<NonInteractiveArgs>,
) -> Result<()> {
    let draft = match non_interactive {
        Some(args) => build_draft(args)?,
        None => run_wizard()?,
    };

    let content = render(&draft);

    // Determine output path
    let dest = output.unwrap_or_else(|| PathBuf::from(format!("{}.htb", draft.name)));

    std::fs::write(&dest, &content)
        .with_context(|| format!("writing {}", dest.display()))?;

    println!("\n  Written: {}", dest.display());

    // Optionally apply straight into the jobs dir.
    // Always write (not copy) so the daemon's fs watcher fires a Modify event.
    if let Some(jobs_dir) = apply_dir {
        let job_dest = jobs_dir.join(dest.file_name().unwrap());
        std::fs::create_dir_all(jobs_dir).ok();
        std::fs::write(&job_dest, &content)
            .with_context(|| format!("applying to {}", job_dest.display()))?;
        println!("  Applied:  {}", job_dest.display());
    }

    println!("\n  Preview:\n{}", preview(&content));
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Show the first 20 lines of the file, indented.
fn preview(content: &str) -> String {
    content
        .lines()
        .take(20)
        .map(|l| format!("    {}", l))
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn args(name: &str, schedule: &str, prompt: &str) -> NonInteractiveArgs {
        NonInteractiveArgs {
            name: name.to_string(),
            schedule: schedule.to_string(),
            prompt: prompt.to_string(),
            agent: "claude".to_string(),
            workspace: None,
            extra_flags: vec![],
        }
    }

    #[test]
    fn build_draft_produces_single_agent_step() {
        let draft = build_draft(args("my-job", "every 5m", "do something")).unwrap();
        assert_eq!(draft.name, "my-job");
        assert_eq!(draft.schedule, "every 5m");
        assert_eq!(draft.workspace, "~");
        assert_eq!(draft.steps.len(), 1);
        match &draft.steps[0] {
            StepDraft::Agent { agent, prompt, .. } => {
                assert_eq!(agent, "claude");
                assert_eq!(prompt, "do something");
            }
            _ => panic!("expected Agent step"),
        }
    }

    #[test]
    fn build_draft_invalid_schedule_is_error() {
        let err = build_draft(args("job", "weekly on Monday", "prompt")).unwrap_err();
        assert!(err.to_string().contains("invalid schedule"), "got: {}", err);
    }

    #[test]
    fn build_draft_custom_agent_and_workspace() {
        let a = NonInteractiveArgs {
            name: "x".to_string(),
            schedule: "every 1h".to_string(),
            prompt: "p".to_string(),
            agent: "opencode".to_string(),
            workspace: Some("~/projects/foo".to_string()),
            extra_flags: vec!["--dangerously-skip-permissions".to_string()],
        };
        let draft = build_draft(a).unwrap();
        assert_eq!(draft.workspace, "~/projects/foo");
        match &draft.steps[0] {
            StepDraft::Agent { agent, flags, .. } => {
                assert_eq!(agent, "opencode");
                assert_eq!(flags, &["--dangerously-skip-permissions"]);
            }
            _ => panic!(),
        }
    }

    #[tokio::test]
    async fn run_non_interactive_writes_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let output = dir.path().join("out.htb");
        run(
            Some(output.clone()),
            None,
            Some(args("test-job", "every 10m", "echo hi")),
        )
        .await
        .unwrap();
        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("name: test-job"));
        assert!(content.contains("every 10m"));
    }

    #[tokio::test]
    async fn run_non_interactive_apply_writes_to_jobs_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let jobs_dir = dir.path().join("jobs");
        run(
            None,
            Some(&jobs_dir),
            Some(args("apply-job", "every 5m", "echo apply")),
        )
        .await
        .unwrap();
        assert!(jobs_dir.join("apply-job.htb").exists());
    }
}
