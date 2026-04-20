/// Loads all job files from a directory, spawns a tokio task per job,
/// and hot-reloads when files are added / changed / removed.
use anyhow::Result;
use chrono::Utc;
use cron::Schedule;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tokio::{sync::Mutex, task::JoinHandle, time::sleep};
use tracing::{error, info, warn};

use super::config::JobConfig;
use crate::task::{runner::run_task, types::TaskDef};

type JobMap = Arc<Mutex<HashMap<String, JoinHandle<()>>>>;

pub async fn run(jobs_dir: PathBuf) -> Result<()> {
    if !jobs_dir.exists() {
        std::fs::create_dir_all(&jobs_dir)?;
        info!("Created jobs directory: {}", jobs_dir.display());
    }

    let handles: JobMap = Arc::new(Mutex::new(HashMap::new()));

    // Initial load
    load_all(&jobs_dir, Arc::clone(&handles)).await;

    // Poll for file changes every 30 s (simple; swap for notify watcher later)
    loop {
        sleep(Duration::from_secs(30)).await;
        load_all(&jobs_dir, Arc::clone(&handles)).await;
    }
}

async fn load_all(dir: &Path, handles: JobMap) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(err) => {
            error!("Cannot read jobs dir: {}", err);
            return;
        }
    };

    let mut seen: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "yaml" | "yml" | "htb") {
            continue;
        }

        match JobConfig::load(&path) {
            Ok(cfg) => {
                seen.push(cfg.name.clone());
                let mut map = handles.lock().await;
                if !map.contains_key(&cfg.name) {
                    info!("Scheduling job: {}", cfg.name);
                    let handle = tokio::spawn(run_job(cfg.clone()));
                    map.insert(cfg.name.clone(), handle);
                }
            }
            Err(err) => {
                warn!("Failed to load {}: {}", path.display(), err);
            }
        }
    }

    // Cancel jobs whose files have disappeared
    let mut map = handles.lock().await;
    let removed: Vec<String> = map
        .keys()
        .filter(|k| !seen.contains(k))
        .cloned()
        .collect();
    for name in removed {
        info!("Job removed: {}", name);
        if let Some(h) = map.remove(&name) {
            h.abort();
        }
    }
}

async fn run_job(cfg: JobConfig) {
    let folder = cfg
        .folder
        .clone()
        .unwrap_or_else(|| std::env::var("HOME").unwrap_or_else(|_| "~".into()));

    if let Some(ref iso) = cfg.run_once_at {
        run_once_at_time(&cfg, &folder, iso).await;
        return;
    }

    match cfg.frequency.as_deref() {
        Some(expr) => match Schedule::from_str(expr) {
            Ok(schedule) => loop {
                let now = Utc::now();
                if let Some(next) = schedule.upcoming(chrono::Utc).next() {
                    let delta = (next - now).to_std().unwrap_or(Duration::ZERO);
                    sleep(delta).await;
                    execute_all_tasks(&cfg, &folder).await;
                }
            },
            Err(err) => {
                error!("[{}] invalid cron expression '{}': {}", cfg.name, expr, err);
            }
        },
        None => {
            warn!("[{}] no frequency or run_once_at set — running once", cfg.name);
            execute_all_tasks(&cfg, &folder).await;
        }
    }
}

async fn run_once_at_time(cfg: &JobConfig, folder: &str, iso: &str) {
    use chrono::DateTime;
    match DateTime::parse_from_rfc3339(iso) {
        Ok(target) => {
            let now = Utc::now();
            let target_utc = target.with_timezone(&Utc);
            if target_utc > now {
                let delta = (target_utc - now).to_std().unwrap_or(Duration::ZERO);
                info!("[{}] waiting until {} to run", cfg.name, iso);
                sleep(delta).await;
            }
            execute_all_tasks(cfg, folder).await;
        }
        Err(err) => {
            error!("[{}] invalid run_once_at '{}': {}", cfg.name, iso, err);
        }
    }
}

async fn execute_all_tasks(cfg: &JobConfig, folder: &str) {
    info!("[{}] running {} task(s)", cfg.name, cfg.tasks.len());
    for task in &cfg.tasks {
        if let Err(err) = run_task(task, folder, &cfg.name).await {
            error!("[{}] task failed: {}", cfg.name, err);
            for cmd in &cfg.on_fail {
                if let Err(e) = run_task(
                    &TaskDef::Run { command: cmd.clone() },
                    folder,
                    &cfg.name,
                )
                .await
                {
                    error!("[{}] on_fail command failed: {}", cfg.name, e);
                }
            }
        }
    }
}
