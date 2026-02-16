//! Path utilities

use crate::error::Result;
use std::path::{Path, PathBuf};

/// Get data directory
pub fn get_data_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| crate::error::Error::Config("Could not determine data directory".to_string()))?;

    Ok(data_dir.join("proton-drive-sync"))
}

/// Get cache directory
pub fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| crate::error::Error::Config("Could not determine cache directory".to_string()))?;

    Ok(cache_dir.join("proton-drive-sync"))
}

/// Get log directory
pub fn get_log_dir() -> Result<PathBuf> {
    let log_dir = dirs::state_dir()
        .or_else(dirs::data_local_dir)
        .ok_or_else(|| crate::error::Error::Config("Could not determine log directory".to_string()))?;

    Ok(log_dir.join("proton-drive-sync").join("logs"))
}

/// Normalize a path for comparison
pub fn normalize_path(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .map_err(|e| crate::error::Error::InvalidPath(format!("{}: {}", path.display(), e)))?;

    Ok(canonical)
}

/// Join paths safely
pub fn safe_join(base: &Path, path: &str) -> Result<PathBuf> {
    let joined = base.join(path);
    let normalized = normalize_path(&joined)?;

    // Ensure the joined path doesn't escape the base
    if !normalized.starts_with(base) {
        return Err(crate::error::Error::InvalidPath(
            "Path escapes base directory".to_string(),
        ));
    }

    Ok(normalized)
}

/// Get relative path from base
pub fn get_relative_path(base: &Path, full_path: &Path) -> Result<String> {
    let relative = full_path
        .strip_prefix(base)
        .map_err(|_| crate::error::Error::InvalidPath("Path not within base".to_string()))?;

    Ok(relative.to_string_lossy().to_string())
}
