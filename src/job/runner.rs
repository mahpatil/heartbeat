use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use super::config::JobConfig;
use super::schedule::{secs_until_hhmm, Schedule};
use crate::log::writer::JobLogger;
use crate::task::executor::execute_step;

#[derive(Debug, Clone, PartialEq)]
pub enum JobStatus {
    Idle,
    Running,
    Failed(String),
    Done,
}

/// Long-running task for a single job. Respects the schedule, executes steps,
/// and handles on_fail. Exits only when the tokio task is aborted (by the
/// controller on shutdown or hot-reload) or when a OnceAt job completes.
pub async fn run_job_loop(
    config: Arc<JobConfig>,
    status: Arc<Mutex<JobStatus>>,
    logger: JobLogger,
) {
    let workspace = shellexpand::tilde(&config.workspace).to_string();

    match &config.schedule {
        Schedule::Every(interval) => {
            // Fire immediately on first tick, then on interval with Delay behaviour.
            let mut ticker = tokio::time::interval(*interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                run_once(&config, &workspace, &status, &logger).await;
            }
        }

        Schedule::DailyAt { hour, minute } => loop {
            let secs = secs_until_hhmm(*hour, *minute);
            if secs > 0 {
                info!(
                    "[{}] sleeping {}s until {:02}:{:02} local",
                    config.name, secs, hour, minute
                );
                tokio::time::sleep(Duration::from_secs(secs)).await;
            }
            run_once(&config, &workspace, &status, &logger).await;
            // Sleep past the current minute so we don't re-fire in the same slot.
            tokio::time::sleep(Duration::from_secs(61)).await;
        },

        Schedule::OnceAt(target) => {
            let now = Utc::now();
            if *target > now {
                let delta = (*target - now)
                    .to_std()
                    .unwrap_or(Duration::ZERO);
                info!(
                    "[{}] waiting until {} to run",
                    config.name,
                    target.format("%Y-%m-%dT%H:%M:%SZ")
                );
                tokio::time::sleep(delta).await;
            } else {
                warn!(
                    "[{}] schedule is in the past, running immediately",
                    config.name
                );
            }
            run_once(&config, &workspace, &status, &logger).await;
            *status.lock().await = JobStatus::Done;
            // Task exits naturally — controller detects handle completion.
        }
    }
}

// ── Single execution ──────────────────────────────────────────────────────────

async fn run_once(
    config: &JobConfig,
    workspace: &str,
    status: &Arc<Mutex<JobStatus>>,
    logger: &JobLogger,
) {
    *status.lock().await = JobStatus::Running;
    let start = std::time::Instant::now();
    info!(
        "[{}] run started ({} step(s))",
        config.name,
        config.steps.len()
    );
    logger.write_line("===== run started =====").await;

    let mut failed_error: Option<String> = None;

    for (i, step) in config.steps.iter().enumerate() {
        let label = step.display_name(i);
        match execute_step(step, workspace, &config.name, &label, logger).await {
            Ok(()) => {
                info!("[{}][{}] ok", config.name, label);
            }
            Err(err) => {
                let msg = err.to_string();
                error!("[{}][{}] failed: {}", config.name, label, msg);
                logger
                    .write_line(&format!("[{}] FAILED: {}", label, msg))
                    .await;
                failed_error = Some(msg);
                break; // Skip remaining steps
            }
        }
    }

    let elapsed = start.elapsed().as_secs();

    if let Some(err_msg) = failed_error {
        logger
            .write_line(&format!("===== run FAILED ({}s) =====", elapsed))
            .await;
        *status.lock().await = JobStatus::Failed(err_msg);

        // Run on_fail shell commands (failures here are logged, not recursive)
        for cmd in &config.on_fail {
            info!("[{}] on_fail: {}", config.name, cmd);
            logger
                .write_line(&format!("[on_fail] $ {}", cmd))
                .await;
            let result = tokio::process::Command::new("bash")
                .arg("-c")
                .arg(cmd)
                .status()
                .await;
            match result {
                Ok(s) if s.success() => {}
                Ok(s) => {
                    error!("[{}] on_fail exited {}", config.name, s);
                    logger
                        .write_line(&format!("[on_fail] exited {}", s))
                        .await;
                }
                Err(e) => {
                    error!("[{}] on_fail error: {}", config.name, e);
                }
            }
        }
    } else {
        info!(
            "[{}] run completed in {}s",
            config.name, elapsed
        );
        logger
            .write_line(&format!("===== run completed ({}s) =====", elapsed))
            .await;
        *status.lock().await = JobStatus::Idle;
    }
}
