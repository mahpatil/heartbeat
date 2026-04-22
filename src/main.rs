mod cli;
mod daemon;
mod ipc;
mod job;
mod log;
mod task;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Parser)]
#[command(
    name = "heartbeat",
    version,
    about = "Persistent agent/task scheduler running in your user context"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Start the daemon (foreground; use --autostart for background)
    Daemon,

    /// Apply a .htb job file to the jobs directory
    Apply {
        /// Path to the .htb file
        file: PathBuf,
    },

    /// Interactively create a new .htb job file
    New {
        /// Output file path (defaults to <name>.htb in current directory)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Also apply the generated file to the jobs directory
        #[arg(long)]
        apply: bool,
    },

    /// List all running jobs
    List,

    /// Trigger an immediate run of a job
    Run {
        /// Job name
        name: String,
    },

    /// Stop a running job
    Stop {
        /// Job name
        name: String,
    },

    /// Tail the log for a job
    Logs {
        /// Job name
        name: String,
        /// Follow the log in real time (like tail -F)
        #[arg(short, long)]
        follow: bool,
    },

    /// Install helpers (e.g. macOS LaunchAgent for auto-start)
    Install {
        /// Register a LaunchAgent so the daemon starts on login
        #[arg(long)]
        autostart: bool,
    },

    /// Remove installed helpers
    Uninstall {
        /// Remove the LaunchAgent (disables auto-start on login)
        #[arg(long)]
        autostart: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // ── Logging ───────────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("heartbeat=info".parse()?),
        )
        .init();

    let hb_dir = hb_dir();

    // ── Load .env (best-effort) ───────────────────────────────────────────────
    let env_path = hb_dir.join(".env");
    if env_path.exists() {
        dotenvy::from_path(&env_path).ok();
        info!("Loaded env from {}", env_path.display());
    }

    let cli = Cli::parse();

    match cli.command {
        Cmd::Daemon => {
            cli::daemon_cmd::run(hb_dir).await?;
        }

        Cmd::Apply { file } => {
            cli::apply::run(&file, &hb_dir).await?;
        }

        Cmd::New { output, apply } => {
            let apply_dir = if apply { Some(hb_dir.join("jobs")) } else { None };
            cli::new::run(output, apply_dir.as_deref()).await?;
        }

        Cmd::List => {
            cli::list::run(&hb_dir).await?;
        }

        Cmd::Run { name } => {
            cli::run_cmd::run(&name, &hb_dir).await?;
        }

        Cmd::Stop { name } => {
            cli::stop::run(&name, &hb_dir).await?;
        }

        Cmd::Logs { name, follow } => {
            cli::logs::run(&name, &hb_dir, follow).await?;
        }

        Cmd::Install { autostart } => {
            cli::install::run(&hb_dir, autostart).await?;
        }

        Cmd::Uninstall { autostart } => {
            if autostart {
                cli::install::uninstall().await?;
            } else {
                eprintln!("Use `heartbeat uninstall --autostart` to remove the LaunchAgent.");
            }
        }
    }

    Ok(())
}

fn hb_dir() -> PathBuf {
    // Respect an explicit override first, then fall back to $HOME/.heartbeat
    if let Ok(d) = std::env::var("HEARTBEAT_DIR") {
        return PathBuf::from(d);
    }
    home_dir().join(".heartbeat")
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

// Keep this so tests can reference it
#[allow(dead_code)]
fn _hb_dir_from(home: &Path) -> PathBuf {
    home.join(".heartbeat")
}
