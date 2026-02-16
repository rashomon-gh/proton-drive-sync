//! Job processor for sync operations

use crate::db::Db;
use crate::error::{Error, Result};
use crate::proton::{ProtonClient, PathUtils};
use crate::types::{SyncEventType, SyncJob, SyncJobStatus};
use chrono::{Duration, Utc};
use std::path::Path;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

/// Job processor
pub struct JobProcessor {
    db: Db,
    client: ProtonClient,
    #[allow(dead_code)]
    concurrency: usize,
    semaphore: Semaphore,
    remote_delete_behavior: crate::types::RemoteDeleteBehavior,
}

impl JobProcessor {
    /// Create a new job processor
    pub fn new(
        db: Db,
        client: ProtonClient,
        concurrency: usize,
        remote_delete_behavior: crate::types::RemoteDeleteBehavior,
    ) -> Self {
        Self {
            db,
            client,
            concurrency,
            semaphore: Semaphore::new(concurrency),
            remote_delete_behavior,
        }
    }

    /// Process a single job
    pub async fn process_job(&self, job: &SyncJob) -> Result<()> {
        let _permit = self.semaphore.acquire().await?;

        // Mark job as processing
        self.db.mark_job_processing(job.id).await?;

        // Add to processing queue
        self.db.add_to_processing_queue(&job.local_path).await?;

        // Process the job
        let result = match job.event_type {
            SyncEventType::CreateFile => self.process_create_file(job).await,
            SyncEventType::CreateDir => self.process_create_dir(job).await,
            SyncEventType::Update => self.process_update(job).await,
            SyncEventType::Delete => self.process_delete(job).await,
        };

        // Remove from processing queue
        let _ = self.db.remove_from_processing_queue(&job.local_path).await;

        match result {
            Ok(_) => {
                // Mark as synced
                self.db
                    .update_job_status(job.id, SyncJobStatus::Synced, None)
                    .await?;

                // Update file state
                if job.event_type != SyncEventType::Delete {
                    if let Some(token) = &job.change_token {
                        let _ = self.db.update_file_state(&job.local_path, token).await;
                    }
                } else {
                    let _ = self.db.delete_file_state(&job.local_path).await;
                }

                info!("Synced: {} -> {}", job.local_path, job.remote_path);
                Ok(())
            }
            Err(e) => {
                error!("Failed to sync {}: {}", job.local_path, e);

                // Check if we should retry
                if job.n_retries < 5 {
                    // Calculate retry time with exponential backoff
                    let retry_delay = std::time::Duration::from_secs(60 * 2_u64.pow(job.n_retries as u32));
                    let retry_at = Utc::now() + Duration::from_std(retry_delay).unwrap();

                    self.db.increment_job_retry(job.id, retry_at).await?;

                    warn!("Job {} will retry at {}", job.id, retry_at);
                } else {
                    // Mark as blocked
                    self.db
                        .update_job_status(job.id, SyncJobStatus::Blocked, Some(&e.to_string()))
                        .await?;
                }

                Err(e)
            }
        }
    }

    /// Process create file job
    async fn process_create_file(&self, job: &SyncJob) -> Result<()> {
        let path = Path::new(&job.local_path);

        if !path.exists() {
            return Err(Error::FileNotFound(path.to_path_buf()));
        }

        // Read file content
        let content = tokio::fs::read(path).await?;

        // Get parent directory from remote path
        let parent_path = PathUtils::parent(&job.remote_path)
            .ok_or_else(|| Error::InvalidPath("No parent directory".to_string()))?;

        // Get or create parent node ID
        let parent_id = self.get_or_create_parent_node(&parent_path).await?;

        // Get file name
        let file_name = PathUtils::filename(&job.remote_path);

        // Detect mime type
        let mime_type = mime_guess::from_path(path)
            .first()
            .map(|m| m.to_string())
            .or_else(|| {
                if path.extension().map_or(false, |e| e == "txt") {
                    Some("text/plain".to_string())
                } else {
                    Some("application/octet-stream".to_string())
                }
            });

        // Create file
        let result = self
            .client
            .create_file(&parent_id, &file_name, content, mime_type.as_deref())
            .await?;

        if !result.success {
            return Err(Error::Sync(
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        // Store node mapping
        if let Some(node_uid) = result.node_uid {
            let mapping = crate::types::NodeMapping {
                local_path: job.local_path.clone(),
                remote_path: job.remote_path.clone(),
                node_uid,
                parent_node_uid: parent_id,
                is_directory: false,
                updated_at: Utc::now(),
            };

            let _ = self.db.update_node_mapping(&mapping).await;
        }

        Ok(())
    }

    /// Process create directory job
    async fn process_create_dir(&self, job: &SyncJob) -> Result<()> {
        // Get parent directory from remote path
        let parent_path = PathUtils::parent(&job.remote_path)
            .ok_or_else(|| Error::InvalidPath("No parent directory".to_string()))?;

        // Get or create parent node ID
        let parent_id = self.get_or_create_parent_node(&parent_path).await?;

        // Get folder name
        let folder_name = PathUtils::filename(&job.remote_path);

        // Create folder
        let result = self
            .client
            .create_folder(&parent_id, &folder_name)
            .await?;

        if !result.success {
            return Err(Error::Sync(
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        // Store node mapping
        if let Some(node_uid) = result.node_uid {
            let mapping = crate::types::NodeMapping {
                local_path: job.local_path.clone(),
                remote_path: job.remote_path.clone(),
                node_uid,
                parent_node_uid: parent_id,
                is_directory: true,
                updated_at: Utc::now(),
            };

            let _ = self.db.update_node_mapping(&mapping).await;
        }

        Ok(())
    }

    /// Process update job
    async fn process_update(&self, job: &SyncJob) -> Result<()> {
        let path = Path::new(&job.local_path);

        if !path.exists() {
            return Err(Error::FileNotFound(path.to_path_buf()));
        }

        // Check if file exists in node mapping
        let existing = self
            .db
            .get_node_mapping(&job.local_path, &job.remote_path)
            .await?;

        if existing.is_none() {
            // File doesn't exist remotely, treat as create
            return self.process_create_file(job).await;
        }

        // Read file content
        let content = tokio::fs::read(path).await?;

        // Delete old and create new (Proton Drive doesn't have a direct update)
        let existing = existing.unwrap();
        self.client.delete_node(&existing.node_uid).await?;

        // Get parent node ID
        let parent_id = existing.parent_node_uid;

        // Get file name
        let file_name = PathUtils::filename(&job.remote_path);

        // Detect mime type
        let mime_type = mime_guess::from_path(path)
            .first()
            .map(|m| m.to_string())
            .or_else(|| Some("application/octet-stream".to_string()));

        // Create new file
        let result = self
            .client
            .create_file(&parent_id, &file_name, content, mime_type.as_deref())
            .await?;

        if !result.success {
            return Err(Error::Sync(
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        Ok(())
    }

    /// Process delete job
    async fn process_delete(&self, job: &SyncJob) -> Result<()> {
        // Check if file exists in node mapping
        let existing = self
            .db
            .get_node_mapping(&job.local_path, &job.remote_path)
            .await?;

        if let Some(existing) = existing {
            // Delete based on behavior
            match self.remote_delete_behavior {
                crate::types::RemoteDeleteBehavior::Trash => {
                    self.client.delete_node(&existing.node_uid).await?;
                }
                crate::types::RemoteDeleteBehavior::Permanent => {
                    self.client.delete_node_permanent(&existing.node_uid).await?;
                }
            }

            // Remove node mapping
            let _ = self
                .db
                .delete_node_mapping(&job.local_path, &job.remote_path)
                .await;
        }

        Ok(())
    }

    /// Get or create parent node
    async fn get_or_create_parent_node(&self, _remote_path: &str) -> Result<String> {
        // Check if parent exists in mappings
        // For simplicity, we'll just use the root ID
        // In a full implementation, you'd walk up the path

        Ok(self.client.get_root_id())
    }

    /// Refresh client session
    pub async fn refresh_session(&mut self) -> Result<()> {
        self.client.refresh_session().await?;
        Ok(())
    }

    /// Get remaining capacity
    pub fn available_capacity(&self) -> usize {
        self.semaphore.available_permits()
    }
}
