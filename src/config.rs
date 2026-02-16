//! Configuration management

use crate::error::{Error, Result};
use crate::types::Config;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use tokio::fs;

/// Configuration file name
const CONFIG_FILE: &str = "config.json";

/// Config manager with hot-reload support
#[derive(Debug, Clone)]
pub struct ConfigManager {
    config_path: PathBuf,
    config: Config,
    last_modified: DateTime<Utc>,
}

impl ConfigManager {
    /// Create a new config manager
    pub async fn new() -> Result<Self> {
        let config_dir = Self::get_config_dir()?;
        fs::create_dir_all(&config_dir).await?;

        let config_path = config_dir.join(CONFIG_FILE);

        let (config, last_modified) = if config_path.exists() {
            Self::load_config(&config_path).await?
        } else {
            (Config::default(), Utc::now())
        };

        Ok(Self {
            config_path,
            config,
            last_modified,
        })
    }

    /// Check for config updates
    pub async fn check_for_updates(&mut self) -> Result<bool> {
        if !self.config_path.exists() {
            return Ok(false);
        }

        let metadata = fs::metadata(&self.config_path).await?;
        let modified = metadata.modified()?;
        let modified = DateTime::<Utc>::from(modified);

        if modified > self.last_modified {
            let (config, _) = Self::load_config(&self.config_path).await?;
            self.config = config;
            self.last_modified = modified;
            return Ok(true);
        }

        Ok(false)
    }

    /// Get current config
    pub fn get(&self) -> &Config {
        &self.config
    }

    /// Save config
    pub async fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.config)?;
        fs::write(&self.config_path, json).await?;
        Ok(())
    }

    /// Add a sync directory
    pub async fn add_sync_dir(&mut self, source_path: String, remote_root: String) -> Result<()> {
        self.config.sync_dirs.push(crate::types::SyncDir {
            source_path,
            remote_root,
        });
        self.save().await?;
        Ok(())
    }

    /// Remove a sync directory
    pub async fn remove_sync_dir(&mut self, index: usize) -> Result<()> {
        if index >= self.config.sync_dirs.len() {
            return Err(Error::Config(format!(
                "Invalid sync directory index: {}",
                index
            )));
        }
        self.config.sync_dirs.remove(index);
        self.save().await?;
        Ok(())
    }

    /// Set sync concurrency
    pub async fn set_concurrency(&mut self, concurrency: usize) -> Result<()> {
        self.config.sync_concurrency = concurrency;
        self.save().await?;
        Ok(())
    }

    /// Set remote delete behavior
    pub async fn set_remote_delete_behavior(
        &mut self,
        behavior: crate::types::RemoteDeleteBehavior,
    ) -> Result<()> {
        self.config.remote_delete_behavior = behavior;
        self.save().await?;
        Ok(())
    }

    /// Add an exclude pattern
    pub async fn add_exclude_pattern(&mut self, path: String, globs: Vec<String>) -> Result<()> {
        self.config
            .exclude_patterns
            .push(crate::types::ExcludePattern { path, globs });
        self.save().await?;
        Ok(())
    }

    /// Remove an exclude pattern
    pub async fn remove_exclude_pattern(&mut self, index: usize) -> Result<()> {
        if index >= self.config.exclude_patterns.len() {
            return Err(Error::Config(format!(
                "Invalid exclude pattern index: {}",
                index
            )));
        }
        self.config.exclude_patterns.remove(index);
        self.save().await?;
        Ok(())
    }

    /// Get config directory path
    fn get_config_dir() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| Error::Config("Could not determine config directory".to_string()))?;

        Ok(config_dir.join("proton-drive-sync"))
    }

    /// Load config from file
    async fn load_config(path: &PathBuf) -> Result<(Config, DateTime<Utc>)> {
        let content = fs::read_to_string(path).await?;
        let config: Config = serde_json::from_str(&content)?;

        let metadata = fs::metadata(path).await?;
        let modified = metadata.modified()?;
        let modified = DateTime::<Utc>::from(modified);

        Ok((config, modified))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.sync_concurrency, 4);
        assert!(config.sync_dirs.is_empty());
        assert!(config.exclude_patterns.is_empty());
    }

    #[tokio::test]
    async fn test_config_manager_new() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();

        // Set config dir to temp directory
        let config_file = config_dir.join("config.json");

        // Write a test config
        let test_config = Config {
            sync_concurrency: 10,
            sync_dirs: vec![],
            exclude_patterns: vec![],
            remote_delete_behavior: crate::types::RemoteDeleteBehavior::Trash,
            dashboard_host: "127.0.0.1".to_string(),
            dashboard_port: 4242,
        };

        let json = serde_json::to_string_pretty(&test_config).unwrap();
        fs::write(&config_file, json).unwrap();

        // Note: This test would require mocking dirs::config_dir()
        // For now, we just test the default behavior
        let default_config = Config::default();
        assert_eq!(default_config.sync_concurrency, 4);
    }

    #[tokio::test]
    async fn test_add_sync_dir() {
        let mut config = Config::default();
        assert_eq!(config.sync_dirs.len(), 0);

        config.sync_dirs.push(crate::types::SyncDir {
            source_path: "/local/path".to_string(),
            remote_root: "/remote/path".to_string(),
        });

        assert_eq!(config.sync_dirs.len(), 1);
        assert_eq!(config.sync_dirs[0].source_path, "/local/path");
        assert_eq!(config.sync_dirs[0].remote_root, "/remote/path");
    }

    #[tokio::test]
    async fn test_remove_sync_dir() {
        let mut config = Config::default();

        config.sync_dirs.push(crate::types::SyncDir {
            source_path: "/local/path1".to_string(),
            remote_root: "/remote/path1".to_string(),
        });
        config.sync_dirs.push(crate::types::SyncDir {
            source_path: "/local/path2".to_string(),
            remote_root: "/remote/path2".to_string(),
        });

        assert_eq!(config.sync_dirs.len(), 2);

        config.sync_dirs.remove(0);
        assert_eq!(config.sync_dirs.len(), 1);
        assert_eq!(config.sync_dirs[0].source_path, "/local/path2");
    }

    #[tokio::test]
    async fn test_set_concurrency() {
        let mut config = Config::default();
        assert_eq!(config.sync_concurrency, 4);

        config.sync_concurrency = 10;
        assert_eq!(config.sync_concurrency, 10);
    }

    #[tokio::test]
    async fn test_add_exclude_pattern() {
        let mut config = Config::default();
        assert_eq!(config.exclude_patterns.len(), 0);

        config.exclude_patterns.push(crate::types::ExcludePattern {
            path: "/test/path".to_string(),
            globs: vec!["*.tmp".to_string(), "*.log".to_string()],
        });

        assert_eq!(config.exclude_patterns.len(), 1);
        assert_eq!(config.exclude_patterns[0].path, "/test/path");
        assert_eq!(config.exclude_patterns[0].globs.len(), 2);
    }

    #[tokio::test]
    async fn test_remote_delete_behavior() {
        let config1 = Config {
            sync_concurrency: 4,
            sync_dirs: vec![],
            exclude_patterns: vec![],
            remote_delete_behavior: crate::types::RemoteDeleteBehavior::Trash,
            dashboard_host: "127.0.0.1".to_string(),
            dashboard_port: 4242,
        };

        let config2 = Config {
            sync_concurrency: 4,
            sync_dirs: vec![],
            exclude_patterns: vec![],
            remote_delete_behavior: crate::types::RemoteDeleteBehavior::Permanent,
            dashboard_host: "127.0.0.1".to_string(),
            dashboard_port: 4242,
        };

        assert_eq!(
            config1.remote_delete_behavior,
            crate::types::RemoteDeleteBehavior::Trash
        );
        assert_eq!(
            config2.remote_delete_behavior,
            crate::types::RemoteDeleteBehavior::Permanent
        );
    }
}
