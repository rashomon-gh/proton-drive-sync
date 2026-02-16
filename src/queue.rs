//! Job queue for sync operations

use crate::db::Db;
use crate::error::Result;
use crate::types::{SyncJob, SyncJobStatus};
use std::time::Duration;
use tracing::{error, info};

/// Job queue manager
#[derive(Clone)]
pub struct JobQueue {
    db: Db,
}

impl JobQueue {
    /// Create a new job queue
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Get pending jobs
    pub async fn get_pending_jobs(&self, limit: usize) -> Result<Vec<SyncJob>> {
        let jobs = self.db.get_pending_jobs(limit as i64).await?;

        // Filter out jobs that are currently being processed
        let filtered = jobs
            .into_iter()
            .filter(|j| j.status == SyncJobStatus::Pending)
            .collect();

        Ok(filtered)
    }

    /// Get job counts by status
    pub async fn get_status_counts(&self) -> Result<StatusCounts> {
        let pending = self.db.get_job_count(SyncJobStatus::Pending).await?;
        let processing = self.db.get_job_count(SyncJobStatus::Processing).await?;
        let synced = self.db.get_job_count(SyncJobStatus::Synced).await?;
        let blocked = self.db.get_job_count(SyncJobStatus::Blocked).await?;

        Ok(StatusCounts {
            pending: pending as usize,
            processing: processing as usize,
            synced: synced as usize,
            blocked: blocked as usize,
        })
    }

    /// Clean up old completed jobs
    pub async fn cleanup_old_jobs(&self, older_than: Duration) -> Result<u64> {
        self.db.delete_completed_jobs(chrono::Duration::from_std(older_than)?).await
    }

    /// Start background cleanup task
    pub fn start_cleanup_task(&self, interval_duration: Duration) -> tokio::task::JoinHandle<()> {
        let db = self.db.clone();
        let cleanup_duration = Duration::from_secs(7 * 24 * 60 * 60); // 7 days

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval_duration);
            ticker.tick().await; // Skip first tick

            loop {
                ticker.tick().await;

                match db
                    .delete_completed_jobs(chrono::Duration::from_std(cleanup_duration).unwrap())
                    .await
                {
                    Ok(count) if count > 0 => {
                        info!("Cleaned up {} old completed jobs", count);
                    }
                    Err(e) => {
                        error!("Error cleaning up old jobs: {}", e);
                    }
                    _ => {}
                }
            }
        })
    }

    /// Clear stale processing jobs
    pub async fn clear_stale_processing(&self, older_than_secs: i64) -> Result<u64> {
        self.db.clear_stale_processing(older_than_secs).await
    }
}

/// Status counts
#[derive(Debug, Clone)]
pub struct StatusCounts {
    pub pending: usize,
    pub processing: usize,
    pub synced: usize,
    pub blocked: usize,
}

impl StatusCounts {
    /// Total count
    pub fn total(&self) -> usize {
        self.pending + self.processing + self.synced + self.blocked
    }
}
