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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_event_type_display() {
        assert_eq!(SyncEventType::CreateFile.to_string(), "CREATE_FILE");
        assert_eq!(SyncEventType::CreateDir.to_string(), "CREATE_DIR");
        assert_eq!(SyncEventType::Update.to_string(), "UPDATE");
        assert_eq!(SyncEventType::Delete.to_string(), "DELETE");
    }

    #[test]
    fn test_sync_job_status_display() {
        assert_eq!(SyncJobStatus::Pending.to_string(), "PENDING");
        assert_eq!(SyncJobStatus::Processing.to_string(), "PROCESSING");
        assert_eq!(SyncJobStatus::Synced.to_string(), "SYNCED");
        assert_eq!(SyncJobStatus::Blocked.to_string(), "BLOCKED");
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.sync_concurrency, 4);
        assert_eq!(config.dashboard_host, "127.0.0.1");
        assert_eq!(config.dashboard_port, 4242);
        assert!(config.sync_dirs.is_empty());
        assert!(config.exclude_patterns.is_empty());
        assert_eq!(config.remote_delete_behavior, RemoteDeleteBehavior::Trash);
    }

    #[test]
    fn test_remote_delete_behavior_serialize() {
        let behavior = RemoteDeleteBehavior::Trash;
        let serialized = serde_json::to_string(&behavior).unwrap();
        assert_eq!(serialized, "\"trash\"");

        let behavior = RemoteDeleteBehavior::Permanent;
        let serialized = serde_json::to_string(&behavior).unwrap();
        assert_eq!(serialized, "\"permanent\"");
    }

    #[test]
    fn test_remote_delete_behavior_deserialize() {
        let behavior: RemoteDeleteBehavior = serde_json::from_str("\"trash\"").unwrap();
        assert_eq!(behavior, RemoteDeleteBehavior::Trash);

        let behavior: RemoteDeleteBehavior = serde_json::from_str("\"permanent\"").unwrap();
        assert_eq!(behavior, RemoteDeleteBehavior::Permanent);
    }

    #[test]
    fn test_sync_dir() {
        let sync_dir = SyncDir {
            source_path: "/local/path".to_string(),
            remote_root: "/remote/path".to_string(),
        };

        let serialized = serde_json::to_string(&sync_dir).unwrap();
        let deserialized: SyncDir = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.source_path, "/local/path");
        assert_eq!(deserialized.remote_root, "/remote/path");
    }

    #[test]
    fn test_session_serialize() {
        let session = Session {
            uid: "test_uid".to_string(),
            access_token: "test_token".to_string(),
            refresh_token: "test_refresh".to_string(),
            key_password: Some("password".to_string()),
            primary_key: Some("key".to_string()),
        };

        let serialized = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.uid, "test_uid");
        assert_eq!(deserialized.access_token, "test_token");
        assert_eq!(deserialized.key_password, Some("password".to_string()));
    }

    #[test]
    fn test_sync_event_type_equality() {
        assert_eq!(SyncEventType::CreateFile, SyncEventType::CreateFile);
        assert_ne!(SyncEventType::CreateFile, SyncEventType::CreateDir);
    }

    #[test]
    fn test_sync_job_status_equality() {
        assert_eq!(SyncJobStatus::Pending, SyncJobStatus::Pending);
        assert_ne!(SyncJobStatus::Pending, SyncJobStatus::Processing);
    }
}
