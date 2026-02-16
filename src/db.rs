//! Database module for SQLite operations

use crate::error::{Error, Result};
use crate::types::{FileState, NodeMapping, SyncEvent, SyncEventType, SyncJob, SyncJobStatus};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use std::path::PathBuf;

/// Database connection pool
#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    /// Create a new database connection
    pub async fn new(db_path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;

        // Run migrations manually
        Self::run_migrations(&pool).await?;

        Ok(Self { pool })
    }

    /// Run database migrations
    async fn run_migrations(pool: &SqlitePool) -> Result<()> {
        // Create tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS signals (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                signal TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS flags (
                name TEXT PRIMARY KEY,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS sync_jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL CHECK(event_type IN ('CREATE_FILE', 'CREATE_DIR', 'UPDATE', 'DELETE')),
                local_path TEXT NOT NULL,
                remote_path TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'PENDING' CHECK(status IN ('PENDING', 'PROCESSING', 'SYNCED', 'BLOCKED')),
                retry_at DATETIME,
                n_retries INTEGER DEFAULT 0,
                last_error TEXT,
                change_token TEXT,
                old_local_path TEXT,
                old_remote_path TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_sync_jobs_status ON sync_jobs(status, created_at);
            CREATE INDEX IF NOT EXISTS idx_sync_jobs_retry_at ON sync_jobs(retry_at);

            CREATE TABLE IF NOT EXISTS processing_queue (
                local_path TEXT PRIMARY KEY,
                started_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS file_state (
                local_path TEXT PRIMARY KEY,
                change_token TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_file_state_prefix ON file_state(local_path);

            CREATE TABLE IF NOT EXISTS node_mapping (
                local_path TEXT NOT NULL,
                remote_path TEXT NOT NULL,
                node_uid TEXT NOT NULL,
                parent_node_uid TEXT NOT NULL,
                is_directory BOOLEAN NOT NULL DEFAULT 0,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (local_path, remote_path)
            );

            CREATE INDEX IF NOT EXISTS idx_node_mapping_local ON node_mapping(local_path);
            CREATE INDEX IF NOT EXISTS idx_node_mapping_remote ON node_mapping(remote_path);
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get the underlying pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // === Signal operations (IPC) ===

    /// Send a signal
    pub async fn send_signal(&self, signal: &str) -> Result<()> {
        sqlx::query("INSERT INTO signals (signal) VALUES (?)")
            .bind(signal)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Receive all pending signals
    pub async fn receive_signals(&self) -> Result<Vec<String>> {
        let rows = sqlx::query_as::<_, (String,)>("SELECT signal FROM signals ORDER BY created_at")
            .fetch_all(&self.pool)
            .await?;

        // Clear received signals
        sqlx::query("DELETE FROM signals")
            .execute(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    // === Flag operations ===

    /// Set a flag
    pub async fn set_flag(&self, name: &str) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO flags (name) VALUES (?)")
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Check if a flag is set
    pub async fn get_flag(&self, name: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM flags WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await?;
        Ok(count > 0)
    }

    /// Clear a flag
    pub async fn clear_flag(&self, name: &str) -> Result<()> {
        sqlx::query("DELETE FROM flags WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // === Sync job operations ===

    /// Enqueue a sync job
    pub async fn enqueue_job(&self, job: &SyncEvent) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO sync_jobs (event_type, local_path, remote_path, status, change_token, old_local_path, old_remote_path)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(job.event_type.to_string())
        .bind(&job.local_path)
        .bind(&job.remote_path)
        .bind(SyncJobStatus::Pending.to_string())
        .bind(&job.change_token)
        .bind(&job.old_local_path)
        .bind(&job.old_remote_path)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get pending jobs
    pub async fn get_pending_jobs(&self, limit: i64) -> Result<Vec<SyncJob>> {
        let rows = sqlx::query(
            r#"
            SELECT id, event_type, local_path, remote_path,
                   status, retry_at, n_retries, last_error,
                   change_token, old_local_path, old_remote_path, created_at
            FROM sync_jobs
            WHERE status = 'PENDING'
               OR (status = 'PROCESSING' AND retry_at < datetime('now'))
            ORDER BY created_at ASC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let jobs = rows
            .into_iter()
            .map(|row| {
                let event_type_str: String = row
                    .try_get("event_type")
                    .map_err(|e| Error::Database(e.into()))?;
                let status_str: String = row
                    .try_get("status")
                    .map_err(|e| Error::Database(e.into()))?;

                Ok(SyncJob {
                    id: row.try_get("id").map_err(|e| Error::Database(e.into()))?,
                    event_type: parse_sync_event_type(&event_type_str),
                    local_path: row
                        .try_get("local_path")
                        .map_err(|e| Error::Database(e.into()))?,
                    remote_path: row
                        .try_get("remote_path")
                        .map_err(|e| Error::Database(e.into()))?,
                    status: parse_sync_job_status(&status_str),
                    retry_at: row.try_get("retry_at").ok(),
                    n_retries: row
                        .try_get("n_retries")
                        .map_err(|e| Error::Database(e.into()))?,
                    last_error: row.try_get("last_error").ok(),
                    change_token: row.try_get("change_token").ok(),
                    old_local_path: row.try_get("old_local_path").ok(),
                    old_remote_path: row.try_get("old_remote_path").ok(),
                    created_at: row
                        .try_get("created_at")
                        .map_err(|e| Error::Database(e.into()))?,
                })
            })
            .collect::<Result<Vec<SyncJob>>>()?;

        Ok(jobs)
    }

    /// Update job status
    pub async fn update_job_status(
        &self,
        id: i64,
        status: SyncJobStatus,
        error: Option<&str>,
    ) -> Result<()> {
        sqlx::query("UPDATE sync_jobs SET status = ?, last_error = ? WHERE id = ?")
            .bind(status.to_string())
            .bind(error)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Mark job as processing
    pub async fn mark_job_processing(&self, id: i64) -> Result<()> {
        sqlx::query("UPDATE sync_jobs SET status = ?, retry_at = NULL WHERE id = ?")
            .bind(SyncJobStatus::Processing.to_string())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Increment job retry count
    pub async fn increment_job_retry(&self, id: i64, retry_at: DateTime<Utc>) -> Result<()> {
        sqlx::query("UPDATE sync_jobs SET n_retries = n_retries + 1, retry_at = ? WHERE id = ?")
            .bind(retry_at)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete completed jobs
    pub async fn delete_completed_jobs(&self, older_than: chrono::Duration) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM sync_jobs WHERE status = 'SYNCED' AND created_at < datetime('now', '-' || ? || ' seconds')",
        )
        .bind(older_than.num_seconds())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Get job count by status
    pub async fn get_job_count(&self, status: SyncJobStatus) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sync_jobs WHERE status = ?")
            .bind(status.to_string())
            .fetch_one(&self.pool)
            .await?;

        Ok(count)
    }

    // === File state operations ===

    /// Get file state
    pub async fn get_file_state(&self, local_path: &str) -> Result<Option<FileState>> {
        let row = sqlx::query(
            "SELECT local_path, change_token, updated_at FROM file_state WHERE local_path = ?",
        )
        .bind(local_path)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            let local_path: String = r
                .try_get("local_path")
                .unwrap_or_else(|_| local_path.to_string());
            let change_token: String = r.try_get("change_token").unwrap_or_default();
            let updated_at: DateTime<Utc> = r.try_get("updated_at").unwrap_or_else(|_| Utc::now());

            FileState {
                local_path,
                change_token,
                updated_at,
            }
        }))
    }

    /// Update file state
    pub async fn update_file_state(&self, local_path: &str, change_token: &str) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO file_state (local_path, change_token, updated_at) VALUES (?, ?, datetime('now'))",
        )
        .bind(local_path)
        .bind(change_token)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete file state
    pub async fn delete_file_state(&self, local_path: &str) -> Result<()> {
        sqlx::query("DELETE FROM file_state WHERE local_path = ?")
            .bind(local_path)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get all file states for a path prefix
    pub async fn get_file_states_under(&self, path_prefix: &str) -> Result<Vec<FileState>> {
        let rows = sqlx::query(
            "SELECT local_path, change_token, updated_at FROM file_state WHERE local_path LIKE ? || '%'",
        )
        .bind(path_prefix)
        .fetch_all(&self.pool)
        .await?;

        let states = rows
            .into_iter()
            .map(|r| {
                let local_path: String = r.try_get("local_path").unwrap_or_default();
                let change_token: String = r.try_get("change_token").unwrap_or_default();
                let updated_at: DateTime<Utc> =
                    r.try_get("updated_at").unwrap_or_else(|_| Utc::now());

                FileState {
                    local_path,
                    change_token,
                    updated_at,
                }
            })
            .collect();

        Ok(states)
    }

    // === Node mapping operations ===

    /// Get node mapping
    pub async fn get_node_mapping(
        &self,
        local_path: &str,
        remote_path: &str,
    ) -> Result<Option<NodeMapping>> {
        let row = sqlx::query(
            r#"
            SELECT local_path, remote_path, node_uid, parent_node_uid, is_directory, updated_at
            FROM node_mapping
            WHERE local_path = ? AND remote_path = ?
            "#,
        )
        .bind(local_path)
        .bind(remote_path)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            let local_path: String = r
                .try_get("local_path")
                .unwrap_or_else(|_| local_path.to_string());
            let remote_path: String = r
                .try_get("remote_path")
                .unwrap_or_else(|_| remote_path.to_string());
            let node_uid: String = r.try_get("node_uid").unwrap_or_default();
            let parent_node_uid: String = r.try_get("parent_node_uid").unwrap_or_default();
            let is_directory: bool = r.try_get("is_directory").unwrap_or(false);
            let updated_at: DateTime<Utc> = r.try_get("updated_at").unwrap_or_else(|_| Utc::now());

            NodeMapping {
                local_path,
                remote_path,
                node_uid,
                parent_node_uid,
                is_directory,
                updated_at,
            }
        }))
    }

    /// Update node mapping
    pub async fn update_node_mapping(&self, mapping: &NodeMapping) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO node_mapping
            (local_path, remote_path, node_uid, parent_node_uid, is_directory, updated_at)
            VALUES (?, ?, ?, ?, ?, datetime('now'))
            "#,
        )
        .bind(&mapping.local_path)
        .bind(&mapping.remote_path)
        .bind(&mapping.node_uid)
        .bind(&mapping.parent_node_uid)
        .bind(mapping.is_directory)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete node mapping
    pub async fn delete_node_mapping(&self, local_path: &str, remote_path: &str) -> Result<()> {
        sqlx::query("DELETE FROM node_mapping WHERE local_path = ? AND remote_path = ?")
            .bind(local_path)
            .bind(remote_path)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get all node mappings for a path prefix
    pub async fn get_node_mappings_under(&self, path_prefix: &str) -> Result<Vec<NodeMapping>> {
        let rows = sqlx::query(
            r#"
            SELECT local_path, remote_path, node_uid, parent_node_uid, is_directory, updated_at
            FROM node_mapping
            WHERE local_path LIKE ? || '%'
            "#,
        )
        .bind(path_prefix)
        .fetch_all(&self.pool)
        .await?;

        let mappings = rows
            .into_iter()
            .map(|r| {
                let local_path: String = r.try_get("local_path").unwrap_or_default();
                let remote_path: String = r.try_get("remote_path").unwrap_or_default();
                let node_uid: String = r.try_get("node_uid").unwrap_or_default();
                let parent_node_uid: String = r.try_get("parent_node_uid").unwrap_or_default();
                let is_directory: bool = r.try_get("is_directory").unwrap_or(false);
                let updated_at: DateTime<Utc> =
                    r.try_get("updated_at").unwrap_or_else(|_| Utc::now());

                NodeMapping {
                    local_path,
                    remote_path,
                    node_uid,
                    parent_node_uid,
                    is_directory,
                    updated_at,
                }
            })
            .collect();

        Ok(mappings)
    }

    // === Processing queue operations ===

    /// Add to processing queue
    pub async fn add_to_processing_queue(&self, local_path: &str) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO processing_queue (local_path, started_at) VALUES (?, datetime('now'))")
            .bind(local_path)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Remove from processing queue
    pub async fn remove_from_processing_queue(&self, local_path: &str) -> Result<()> {
        sqlx::query("DELETE FROM processing_queue WHERE local_path = ?")
            .bind(local_path)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Clear stale processing queue entries
    pub async fn clear_stale_processing(&self, older_than: i64) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM processing_queue WHERE started_at < datetime('now', '-' || ? || ' seconds')",
        )
        .bind(older_than)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

/// Helper function to parse SyncEventType from string
fn parse_sync_event_type(s: &str) -> SyncEventType {
    match s {
        "CREATE_FILE" => SyncEventType::CreateFile,
        "CREATE_DIR" => SyncEventType::CreateDir,
        "UPDATE" => SyncEventType::Update,
        "DELETE" => SyncEventType::Delete,
        _ => SyncEventType::Update, // Default fallback
    }
}

/// Helper function to parse SyncJobStatus from string
fn parse_sync_job_status(s: &str) -> SyncJobStatus {
    match s {
        "PENDING" => SyncJobStatus::Pending,
        "PROCESSING" => SyncJobStatus::Processing,
        "SYNCED" => SyncJobStatus::Synced,
        "BLOCKED" => SyncJobStatus::Blocked,
        _ => SyncJobStatus::Pending, // Default fallback
    }
}
