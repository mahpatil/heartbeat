mod job;
mod task;

use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("heartbeat=info".parse()?),
        )
        .init();

    // Load .env from ~/.heartbeat/ if present
    let home = home_dir();
    let env_path = home.join(".heartbeat").join(".env");
    if env_path.exists() {
        dotenvy::from_path(&env_path).ok();
        info!("Loaded env from {}", env_path.display());
    }

    let jobs_dir = home.join(".heartbeat").join("jobs");

    info!("Starting heartbeat service, watching {}", jobs_dir.display());

    job::scheduler::run(jobs_dir).await
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("~"))
}
