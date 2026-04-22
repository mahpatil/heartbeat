//! `heartbeat new` — interactive wizard that generates a `.htb` job file.

mod prompts;
mod render;
mod wizard;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use self::render::render;
use self::wizard::run as run_wizard;

/// Entry point called from `main.rs`.
pub async fn run(output: Option<PathBuf>, apply_dir: Option<&Path>) -> Result<()> {
    let draft = run_wizard()?;

    let content = render(&draft);

    // Determine output path
    let dest = output.unwrap_or_else(|| PathBuf::from(format!("{}.htb", draft.name)));

    std::fs::write(&dest, &content)
        .with_context(|| format!("writing {}", dest.display()))?;

    println!("\n  Written: {}", dest.display());

    // Optionally copy straight into the jobs dir (like `apply`)
    if let Some(jobs_dir) = apply_dir {
        let job_dest = jobs_dir.join(dest.file_name().unwrap());
        std::fs::create_dir_all(jobs_dir).ok();
        std::fs::copy(&dest, &job_dest)
            .with_context(|| format!("copying to {}", job_dest.display()))?;
        println!("  Applied:  {}", job_dest.display());
    }

    println!("\n  Preview:\n{}", preview(&content));
    Ok(())
}

/// Show the first 20 lines of the file, dimmed.
fn preview(content: &str) -> String {
    content
        .lines()
        .take(20)
        .map(|l| format!("    {}", l))
        .collect::<Vec<_>>()
        .join("\n")
}
