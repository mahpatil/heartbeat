use anyhow::Result;
use notify::{Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::job::{
    config::JobConfig,
    runner::{run_job_loop, JobStatus},
};
use crate::log::writer::JobLogger;

// ── Internal job state ────────────────────────────────────────────────────────

// Fields used by M2 IPC (list/status commands)
#[allow(dead_code)]
struct JobState {
    config: Arc<JobConfig>,
    handle: JoinHandle<()>,
    status: Arc<Mutex<JobStatus>>,
    logger: JobLogger,
}

// ── Controller ────────────────────────────────────────────────────────────────

pub struct Controller {
    hb_dir: PathBuf,
    jobs: Arc<Mutex<HashMap<String, JobState>>>,
}

impl Controller {
    pub fn new(hb_dir: PathBuf) -> Self {
        Self {
            hb_dir,
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Main run loop. Blocks until `shutdown` fires.
    pub async fn run(
        self,
        mut shutdown: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<()> {
        let jobs_dir = self.hb_dir.join("jobs");
        let logs_dir = self.hb_dir.join("logs");

        // Ensure directories exist
        for dir in [&jobs_dir, &logs_dir] {
            if !dir.exists() {
                tokio::fs::create_dir_all(dir).await?;
                info!("Created directory: {}", dir.display());
            }
        }

        // Initial load of all existing job files
        self.load_all(&jobs_dir, &logs_dir).await;

        // ── Filesystem watcher ────────────────────────────────────────────────
        // notify uses a sync callback; bridge it to an async channel.
        let (sync_tx, sync_rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = sync_tx.send(res);
            },
            NotifyConfig::default(),
        )?;
        watcher.watch(&jobs_dir, RecursiveMode::NonRecursive)?;

        let (async_tx, mut async_rx) =
            tokio::sync::mpsc::channel::<notify::Result<Event>>(64);
        tokio::task::spawn_blocking(move || {
            for event in sync_rx {
                if async_tx.blocking_send(event).is_err() {
                    break;
                }
            }
        });

        // ── Event loop ────────────────────────────────────────────────────────
        loop {
            tokio::select! {
                biased;

                _ = &mut shutdown => {
                    info!("Shutdown received — stopping all jobs");
                    self.stop_all().await;
                    // Dropping watcher closes the sync channel, unblocking
                    // the spawn_blocking relay task.
                    drop(watcher);
                    break;
                }

                Some(event) = async_rx.recv() => {
                    match event {
                        Ok(evt) => {
                            self.handle_fs_event(evt, &jobs_dir, &logs_dir).await;
                        }
                        Err(e) => {
                            error!("Watcher error: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // ── FS event dispatch ─────────────────────────────────────────────────────

    async fn handle_fs_event(&self, event: Event, jobs_dir: &Path, logs_dir: &Path) {
        use notify::event::{CreateKind, ModifyKind, RemoveKind};

        let is_htb = |p: &std::path::Path| {
            p.extension().and_then(|e| e.to_str()) == Some("htb")
        };

        match event.kind {
            // File created or data changed
            EventKind::Create(CreateKind::File)
            | EventKind::Modify(ModifyKind::Data(_)) => {
                for path in &event.paths {
                    if is_htb(path) && path.exists() {
                        self.reload_job(path, logs_dir).await;
                    }
                }
            }

            // File renamed (both old and new paths arrive together sometimes)
            EventKind::Modify(ModifyKind::Name(_)) => {
                for path in &event.paths {
                    if is_htb(path) {
                        if path.exists() {
                            self.reload_job(path, logs_dir).await;
                        } else if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            self.stop_job(stem).await;
                        }
                    }
                }
            }

            // File removed
            EventKind::Remove(RemoveKind::File) => {
                for path in &event.paths {
                    if is_htb(path) {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            self.stop_job(stem).await;
                        }
                    }
                }
            }

            _ => {}
        }

        let _ = jobs_dir; // suppress unused warning
    }

    // ── Job lifecycle ─────────────────────────────────────────────────────────

    async fn load_all(&self, jobs_dir: &Path, logs_dir: &Path) {
        let entries = match std::fs::read_dir(jobs_dir) {
            Ok(e) => e,
            Err(e) => {
                error!("Cannot read jobs dir {}: {}", jobs_dir.display(), e);
                return;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("htb") {
                self.start_job(&path, logs_dir).await;
            }
        }
    }

    async fn reload_job(&self, path: &Path, logs_dir: &Path) {
        // Stop the old instance (if any) before starting the new one.
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            // Check by file stem first (fast path), then by config name if loaded.
            let has_by_stem = self.jobs.lock().await.contains_key(stem);
            if has_by_stem {
                self.stop_job(stem).await;
            }
        }
        self.start_job(path, logs_dir).await;
    }

    async fn start_job(&self, path: &Path, logs_dir: &Path) {
        let cfg = match JobConfig::load(path) {
            Ok(c) => Arc::new(c),
            Err(e) => {
                warn!("Failed to load {}: {}", path.display(), e);
                return;
            }
        };

        let mut jobs = self.jobs.lock().await;

        if jobs.contains_key(&cfg.name) {
            warn!(
                "Job name conflict: {:?} — skipping {}",
                cfg.name,
                path.display()
            );
            return;
        }

        let logger = match JobLogger::new(&cfg.name, logs_dir).await {
            Ok(l) => l,
            Err(e) => {
                error!(
                    "Failed to create logger for job {}: {}",
                    cfg.name, e
                );
                return;
            }
        };

        let status = Arc::new(Mutex::new(JobStatus::Idle));
        let cfg_c = Arc::clone(&cfg);
        let status_c = Arc::clone(&status);
        let logger_c = logger.clone();

        info!(
            "Scheduling job: {} ({})",
            cfg.name,
            cfg.schedule.display()
        );

        let handle = tokio::spawn(run_job_loop(cfg_c, status_c, logger_c));
        jobs.insert(
            cfg.name.clone(),
            JobState {
                config: cfg,
                handle,
                status,
                logger,
            },
        );
    }

    async fn stop_job(&self, name: &str) {
        let mut jobs = self.jobs.lock().await;
        if let Some(state) = jobs.remove(name) {
            info!("Stopping job: {}", name);
            state.handle.abort();
        }
    }

    async fn stop_all(&self) {
        let mut jobs = self.jobs.lock().await;
        for (name, state) in jobs.drain() {
            info!("Stopping job: {}", name);
            state.handle.abort();
        }
    }
}
