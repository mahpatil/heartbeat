//! `heartbeat build` — generate a `.htb` job file from a natural-language
//! description through a guided, conversational prompt flow.
//!
//! Usage:
//!   heartbeat build                          # prompts for description
//!   heartbeat build "Every morning …"        # description from arg
//!   heartbeat build "…" --output out.htb     # custom output path
//!   heartbeat build "…" --apply              # write + apply to jobs dir

mod conversation;
mod parser;
mod slugify;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::cli::new::render_draft;
use crate::cli::prompts::ask_string;

use self::conversation::confirm_and_fill;
use self::parser::parse;
use self::slugify::suggest;

// ── Public entry point ────────────────────────────────────────────────────────

pub async fn run(
    description: Option<String>,
    output: Option<PathBuf>,
    apply_dir: Option<&Path>,
) -> Result<()> {
    println!("\n  Heartbeat Builder — describe your job in plain English.");
    println!("  Examples:");
    println!("    \"Every morning ask Claude to summarise my GitHub PRs in ~/projects/app\"");
    println!("    \"Every 5 minutes check https://api.example.com, then ask Claude to alert me\"");
    println!();

    // ── Collect description ───────────────────────────────────────────────────
    let desc = match description {
        Some(d) => d,
        None => ask_string("Describe your job", None)?,
    };

    // ── Parse NL → intent ─────────────────────────────────────────────────────
    let intent = parse(&desc);
    let suggested_name = suggest(&desc);

    // ── Confirm and fill any gaps ─────────────────────────────────────────────
    let draft = confirm_and_fill(intent, &suggested_name)?;

    // ── Render and write ──────────────────────────────────────────────────────
    let content = render_draft(&draft);
    let dest = output.unwrap_or_else(|| PathBuf::from(format!("{}.htb", draft.name)));

    std::fs::write(&dest, &content)
        .with_context(|| format!("writing {}", dest.display()))?;

    println!("\n  Written: {}", dest.display());

    if let Some(jobs_dir) = apply_dir {
        let job_dest = jobs_dir.join(dest.file_name().unwrap());
        std::fs::create_dir_all(jobs_dir).ok();
        std::fs::write(&job_dest, &content)
            .with_context(|| format!("applying to {}", job_dest.display()))?;
        println!("  Applied: {}", job_dest.display());
    }

    println!("\n  Preview:\n{}", preview(&content));
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn preview(content: &str) -> String {
    content
        .lines()
        .take(20)
        .map(|l| format!("    {}", l))
        .collect::<Vec<_>>()
        .join("\n")
}
