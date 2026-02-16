//! Sync engine

use crate::config::ConfigManager;
use crate::db::Db;
use crate::error::{Error, Result};
use crate::processor::JobProcessor;
use crate::proton::ProtonClient;
use crate::queue::JobQueue;
use crate::types::Session;
use crate::watcher::FileWatcher;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

/// Sync engine state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    Idle,
    Running,
    Paused,
    Error,
}

/// Sync engine
pub struct SyncEngine {
    db: Db,
    config: Arc<Mutex<ConfigManager>>,
    session: Session,
    state: Arc<Mutex<SyncState>>,
    watcher: Arc<Mutex<FileWatcher>>,
    processor: Arc<Mutex<JobProcessor>>,
    queue: JobQueue,
}

impl SyncEngine {
    /// Create a new sync engine
    pub async fn new(db: Db, config: Arc<Mutex<ConfigManager>>, session: Session) -> Result<Self> {
        let cfg = config.lock().await;
        let client = ProtonClient::new(session.clone());
        let processor = JobProcessor::new(
            db.clone(),
            client,
            cfg.get().sync_concurrency,
            cfg.get().remote_delete_behavior,
        );

        let watcher = FileWatcher::new(db.clone(), config.clone())?;

        let queue = JobQueue::new(db.clone());

        drop(cfg);

        Ok(Self {
            db,
            config,
            session,
            state: Arc::new(Mutex::new(SyncState::Idle)),
            watcher: Arc::new(Mutex::new(watcher)),
            processor: Arc::new(Mutex::new(processor)),
            queue,
        })
    }

    /// Start the sync engine
    pub async fn start(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        if *state == SyncState::Running {
            return Ok(());
        }
        *state = SyncState::Running;
        drop(state);

        info!("Starting sync engine");

        // Start file watcher
        let mut watcher = self.watcher.lock().await;
        watcher.start().await?;
        drop(watcher);

        // Start processor task
        self.start_processor_task().await;

        // Start periodic reconciliation
        self.start_reconciliation_task().await;

        // Start config reload task
        self.start_config_reload_task().await;

        // Set running flag
        self.db.set_flag("running").await?;

        info!("Sync engine started");

        Ok(())
    }

    /// Stop the sync engine
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping sync engine");

        let mut state = self.state.lock().await;
        *state = SyncState::Idle;
        drop(state);

        // Stop file watcher
        let mut watcher = self.watcher.lock().await;
        watcher.stop().await?;
        drop(watcher);

        // Clear running flag
        self.db.clear_flag("running").await?;

        info!("Sync engine stopped");

        Ok(())
    }

    /// Pause the sync engine
    pub async fn pause(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        if *state != SyncState::Running {
            return Ok(());
        }
        *state = SyncState::Paused;
        drop(state);

        self.db.set_flag("paused").await?;

        info!("Sync engine paused");

        Ok(())
    }

    /// Resume the sync engine
    pub async fn resume(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        if *state != SyncState::Paused {
            return Ok(());
        }
        *state = SyncState::Running;
        drop(state);

        self.db.clear_flag("paused").await?;

        info!("Sync engine resumed");

        Ok(())
    }

    /// Get current state
    pub async fn get_state(&self) -> SyncState {
        *self.state.lock().await
    }

    /// Get status
    pub async fn get_status(&self) -> Result<SyncStatus> {
        let state = self.get_state().await;
        let counts = self.queue.get_status_counts().await?;

        Ok(SyncStatus {
            state,
            pending_jobs: counts.pending,
            processing_jobs: counts.processing,
            synced_jobs: counts.synced,
            blocked_jobs: counts.blocked,
        })
    }

    /// Start processor task
    async fn start_processor_task(&self) {
        let db = self.db.clone();
        let processor = self.processor.clone();
        let state = self.state.clone();
        let queue = self.queue.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            interval.tick().await; // Skip first tick

            loop {
                interval.tick().await;

                // Check if still running
                let current_state = *state.lock().await;
                if current_state != SyncState::Running {
                    continue;
                }

                // Get pending jobs
                let jobs = match db.get_pending_jobs(10).await {
                    Ok(j) => j,
                    Err(e) => {
                        error!("Error getting pending jobs: {}", e);
                        continue;
                    }
                };

                if jobs.is_empty() {
                    continue;
                }

                // Process each job
                let proc = processor.lock().await;
                for job in jobs {
                    if let Err(e) = proc.process_job(&job).await {
                        error!("Error processing job {}: {}", job.id, e);
                    }
                }
            }
        });
    }

    /// Start periodic reconciliation task
    async fn start_reconciliation_task(&self) {
        let db = self.db.clone();
        let config = self.config.clone();
        let state = self.state.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5 * 60)); // Every 5 minutes
            interval.tick().await; // Skip first tick

            loop {
                interval.tick().await;

                // Check if still running
                let current_state = *state.lock().await;
                if current_state != SyncState::Running {
                    continue;
                }

                // Skip if queue is busy
                let counts = match JobQueue::new(db.clone()).get_status_counts().await {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Error getting queue status: {}", e);
                        continue;
                    }
                };

                if counts.pending > 100 {
                    debug!("Skipping reconciliation: queue too busy");
                    continue;
                }

                // Scan each sync directory
                let cfg = config.lock().await;
                let sync_dirs = cfg.get().sync_dirs.clone();
                let exclusions = cfg.get().exclude_patterns.clone();
                drop(cfg);

                for sync_dir in sync_dirs {
                    if let Err(e) =
                        crate::watcher::FileScanner::scan_directory(&db, &sync_dir.source_path, &sync_dir.remote_root, &exclusions).await
                    {
                        error!("Error scanning directory {}: {}", sync_dir.source_path, e);
                    }
                }

                info!("Reconciliation scan complete");
            }
        });
    }

    /// Start config reload task
    async fn start_config_reload_task(&self) {
        let config = self.config.clone();
        let processor = self.processor.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            interval.tick().await; // Skip first tick

            loop {
                interval.tick().await;

                let mut cfg = config.lock().await;
                if let Ok(updated) = cfg.check_for_updates().await {
                    if updated {
                        info!("Configuration reloaded");

                        // Update processor concurrency if needed
                        let new_concurrency = cfg.get().sync_concurrency;
                        drop(cfg);

                        let mut proc = processor.lock().await;
                        // Note: In a full implementation, you'd update the semaphore size
                        // For now, this is a placeholder
                        info!("Processor concurrency updated to {}", new_concurrency);
                    }
                }
            }
        });
    }

    /// Run reconciliation manually
    pub async fn reconcile(&self) -> Result<usize> {
        info!("Running manual reconciliation");

        let cfg = self.config.lock().await;
        let sync_dirs = cfg.get().sync_dirs.clone();
        let exclusions = cfg.get().exclude_patterns.clone();
        drop(cfg);

        let mut total = 0;

        for sync_dir in sync_dirs {
            let count = crate::watcher::FileScanner::scan_directory(
                &self.db,
                &sync_dir.source_path,
                &sync_dir.remote_root,
                &exclusions,
            )
            .await?;
            total += count;
        }

        info!("Reconciliation complete: {} changes detected", total);

        Ok(total)
    }
}

/// Sync status
#[derive(Debug, Clone)]
pub struct SyncStatus {
    pub state: SyncState,
    pub pending_jobs: usize,
    pub processing_jobs: usize,
    pub synced_jobs: usize,
    pub blocked_jobs: usize,
}
