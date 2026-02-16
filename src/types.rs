//! Core types and enums for Proton Drive Sync

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Sync event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncEventType {
    CreateFile,
    CreateDir,
    Update,
    Delete,
}

impl std::fmt::Display for SyncEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateFile => write!(f, "CREATE_FILE"),
            Self::CreateDir => write!(f, "CREATE_DIR"),
            Self::Update => write!(f, "UPDATE"),
            Self::Delete => write!(f, "DELETE"),
        }
    }
}

/// Sync job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncJobStatus {
    Pending,
    Processing,
    Synced,
    Blocked,
}

impl std::fmt::Display for SyncJobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "PENDING"),
            Self::Processing => write!(f, "PROCESSING"),
            Self::Synced => write!(f, "SYNCED"),
            Self::Blocked => write!(f, "BLOCKED"),
        }
    }
}

/// Remote delete behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemoteDeleteBehavior {
    Trash,
    Permanent,
}

/// Sync directory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDir {
    pub source_path: String,
    pub remote_root: String,
}

/// Exclude pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludePattern {
    pub path: String,
    pub globs: Vec<String>,
}

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub sync_dirs: Vec<SyncDir>,
    #[serde(default = "default_concurrency")]
    pub sync_concurrency: usize,
    #[serde(default = "default_delete_behavior")]
    pub remote_delete_behavior: RemoteDeleteBehavior,
    #[serde(default = "default_dashboard_host")]
    pub dashboard_host: String,
    #[serde(default = "default_dashboard_port")]
    pub dashboard_port: u16,
    #[serde(default)]
    pub exclude_patterns: Vec<ExcludePattern>,
}

fn default_concurrency() -> usize {
    4
}

fn default_delete_behavior() -> RemoteDeleteBehavior {
    RemoteDeleteBehavior::Trash
}

fn default_dashboard_host() -> String {
    "127.0.0.1".to_string()
}

fn default_dashboard_port() -> u16 {
    4242
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sync_dirs: Vec::new(),
            sync_concurrency: default_concurrency(),
            remote_delete_behavior: default_delete_behavior(),
            dashboard_host: default_dashboard_host(),
            dashboard_port: default_dashboard_port(),
            exclude_patterns: Vec::new(),
        }
    }
}

/// Sync job in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJob {
    pub id: i64,
    pub event_type: SyncEventType,
    pub local_path: String,
    pub remote_path: String,
    pub status: SyncJobStatus,
    pub retry_at: Option<DateTime<Utc>>,
    pub n_retries: i32,
    pub last_error: Option<String>,
    pub change_token: Option<String>,
    pub old_local_path: Option<String>,
    pub old_remote_path: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// File state for change detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    pub local_path: String,
    pub change_token: String,
    pub updated_at: DateTime<Utc>,
}

/// Node mapping for Proton Drive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMapping {
    pub local_path: String,
    pub remote_path: String,
    pub node_uid: String,
    pub parent_node_uid: String,
    pub is_directory: bool,
    pub updated_at: DateTime<Utc>,
}

/// Proton Drive session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub uid: String,
    pub access_token: String,
    pub refresh_token: String,
    pub key_password: Option<String>,
    pub primary_key: Option<String>,
}

/// Proton Drive node data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeData {
    pub uid: String,
    pub parent_uid: Option<String>,
    pub name: String,
    pub node_type: String,
    pub media_type: Option<String>,
    pub active_revision: Option<RevisionData>,
}

/// Proton Drive revision data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionData {
    pub uid: String,
    pub size: Option<i64>,
    pub manifest_signature: Option<String>,
}

/// Proton Drive address data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressData {
    pub email: String,
    pub receive_key: Option<String>,
}

/// Create operation result
#[derive(Debug, Clone)]
pub struct CreateResult {
    pub success: bool,
    pub node_uid: Option<String>,
    pub error: Option<String>,
}

/// Sync event for enqueuing
#[derive(Debug, Clone)]
pub struct SyncEvent {
    pub event_type: SyncEventType,
    pub local_path: String,
    pub remote_path: String,
    pub change_token: Option<String>,
    pub old_local_path: Option<String>,
    pub old_remote_path: Option<String>,
}
