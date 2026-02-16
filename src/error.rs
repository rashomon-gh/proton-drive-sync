//! Error types for Proton Drive Sync

use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Proton API error: {0}")]
    ProtonApi(String),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Keyring error: {0}")]
    Keyring(String),

    #[error("Sync error: {0}")]
    Sync(String),

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Timeout")]
    Timeout,

    #[error("Cancelled")]
    Cancelled,

    #[error("Watch error: {0}")]
    Watch(String),
}

impl From<keyring::Error> for Error {
    fn from(err: keyring::Error) -> Self {
        Error::Keyring(err.to_string())
    }
}

impl From<notify::Error> for Error {
    fn from(err: notify::Error) -> Self {
        Error::Watch(err.to_string())
    }
}

impl From<tokio::sync::AcquireError> for Error {
    fn from(_: tokio::sync::AcquireError) -> Self {
        Error::InvalidState("Failed to acquire semaphore permit".to_string())
    }
}

impl From<chrono::OutOfRangeError> for Error {
    fn from(err: chrono::OutOfRangeError) -> Self {
        Error::InvalidState(format!("Duration out of range: {}", err))
    }
}
