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

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Idle => "idle",
            JobStatus::Running => "running",
            JobStatus::Failed(_) => "failed",
            JobStatus::Done => "done",
        }
    }
}

/// Trigger an immediate one-off execution outside the normal schedule loop.
/// Used by the `heartbeat run <name>` IPC command.
pub async fn execute_job_now(
    config: Arc<JobConfig>,
    status: Arc<Mutex<JobStatus>>,
    logger: JobLogger,
) {
    let workspace = shellexpand::tilde(&config.workspace).to_string();
    run_once(&config, &workspace, &status, &logger).await;
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

pub(crate) async fn run_once(
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::schedule::Schedule;
    use crate::task::types::StepDef;
    use std::time::Duration;
    use tempfile::TempDir;

    fn make_config(steps: Vec<StepDef>, on_fail: Vec<String>) -> Arc<JobConfig> {
        Arc::new(JobConfig {
            name: "test-job".into(),
            schedule: Schedule::Every(Duration::from_secs(60)),
            workspace: "/tmp".into(),
            steps,
            on_fail,
        })
    }

    async fn make_logger(dir: &TempDir) -> JobLogger {
        JobLogger::new("test-job", dir.path()).await.unwrap()
    }

    fn shell(cmd: &str) -> StepDef {
        StepDef::Shell {
            name: None,
            command: cmd.to_string(),
            workspace: None,
            timeout: None,
            env: Default::default(),
        }
    }

    fn named_shell(name: &str, cmd: &str) -> StepDef {
        StepDef::Shell {
            name: Some(name.to_string()),
            command: cmd.to_string(),
            workspace: None,
            timeout: None,
            env: Default::default(),
        }
    }

    // ── Sequential execution ──────────────────────────────────────────────────

    #[tokio::test]
    async fn all_steps_succeed_status_is_idle() {
        let dir = TempDir::new().unwrap();
        let logger = make_logger(&dir).await;
        let status = Arc::new(Mutex::new(JobStatus::Idle));
        let cfg = make_config(
            vec![shell("echo step1"), shell("echo step2"), shell("echo step3")],
            vec![],
        );

        run_once(&cfg, "/tmp", &status, &logger).await;
        assert_eq!(*status.lock().await, JobStatus::Idle);
    }

    #[tokio::test]
    async fn failing_step_skips_remaining() {
        let dir = TempDir::new().unwrap();
        let logger = make_logger(&dir).await;
        let status = Arc::new(Mutex::new(JobStatus::Idle));

        // step 1 succeeds, step 2 fails, step 3 must not run (would create a file)
        let marker = dir.path().join("step3-ran.txt");
        let marker_path = marker.to_str().unwrap().to_string();
        let cfg = make_config(
            vec![
                shell("echo step1-ok"),
                shell("exit 1"),
                shell(&format!("touch {}", marker_path)),
            ],
            vec![],
        );

        run_once(&cfg, "/tmp", &status, &logger).await;

        // Status should be Failed
        assert!(matches!(*status.lock().await, JobStatus::Failed(_)));
        // Step 3 must NOT have run
        assert!(!marker.exists(), "step 3 ran despite step 2 failing");
    }

    #[tokio::test]
    async fn on_fail_runs_after_step_failure() {
        let dir = TempDir::new().unwrap();
        let logger = make_logger(&dir).await;
        let status = Arc::new(Mutex::new(JobStatus::Idle));

        let on_fail_marker = dir.path().join("on-fail.txt");
        let on_fail_path = on_fail_marker.to_str().unwrap().to_string();

        let cfg = make_config(
            vec![shell("exit 1")],
            vec![format!("touch {}", on_fail_path)],
        );

        run_once(&cfg, "/tmp", &status, &logger).await;

        assert!(matches!(*status.lock().await, JobStatus::Failed(_)));
        assert!(on_fail_marker.exists(), "on_fail command did not run");
    }

    #[tokio::test]
    async fn on_fail_skipped_on_success() {
        let dir = TempDir::new().unwrap();
        let logger = make_logger(&dir).await;
        let status = Arc::new(Mutex::new(JobStatus::Idle));

        let on_fail_marker = dir.path().join("on-fail.txt");
        let on_fail_path = on_fail_marker.to_str().unwrap().to_string();

        let cfg = make_config(
            vec![shell("echo ok")],
            vec![format!("touch {}", on_fail_path)],
        );

        run_once(&cfg, "/tmp", &status, &logger).await;

        assert_eq!(*status.lock().await, JobStatus::Idle);
        assert!(!on_fail_marker.exists(), "on_fail ran despite success");
    }

    // ── Log output ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn named_step_appears_in_log() {
        let dir = TempDir::new().unwrap();
        let logger = make_logger(&dir).await;
        let status = Arc::new(Mutex::new(JobStatus::Idle));
        let cfg = make_config(vec![named_shell("run-tests", "echo hi")], vec![]);

        run_once(&cfg, "/tmp", &status, &logger).await;

        let log = tokio::fs::read_to_string(logger.path()).await.unwrap();
        assert!(log.contains("[run-tests]"), "log: {}", log);
    }

    #[tokio::test]
    async fn unnamed_step_logged_by_index() {
        let dir = TempDir::new().unwrap();
        let logger = make_logger(&dir).await;
        let status = Arc::new(Mutex::new(JobStatus::Idle));
        let cfg = make_config(vec![shell("echo hi")], vec![]);

        run_once(&cfg, "/tmp", &status, &logger).await;

        let log = tokio::fs::read_to_string(logger.path()).await.unwrap();
        assert!(log.contains("[step[0]]"), "log: {}", log);
    }

    #[tokio::test]
    async fn run_completed_banner_in_log() {
        let dir = TempDir::new().unwrap();
        let logger = make_logger(&dir).await;
        let status = Arc::new(Mutex::new(JobStatus::Idle));
        let cfg = make_config(vec![shell("echo ok")], vec![]);

        run_once(&cfg, "/tmp", &status, &logger).await;

        let log = tokio::fs::read_to_string(logger.path()).await.unwrap();
        assert!(log.contains("run completed"), "log: {}", log);
    }

    #[tokio::test]
    async fn run_failed_banner_in_log() {
        let dir = TempDir::new().unwrap();
        let logger = make_logger(&dir).await;
        let status = Arc::new(Mutex::new(JobStatus::Idle));
        let cfg = make_config(vec![shell("exit 1")], vec![]);

        run_once(&cfg, "/tmp", &status, &logger).await;

        let log = tokio::fs::read_to_string(logger.path()).await.unwrap();
        assert!(log.contains("run FAILED"), "log: {}", log);
    }

    // ── JobStatus ─────────────────────────────────────────────────────────────

    #[test]
    fn job_status_as_str() {
        assert_eq!(JobStatus::Idle.as_str(), "idle");
        assert_eq!(JobStatus::Running.as_str(), "running");
        assert_eq!(JobStatus::Failed("oops".into()).as_str(), "failed");
        assert_eq!(JobStatus::Done.as_str(), "done");
    }
}
