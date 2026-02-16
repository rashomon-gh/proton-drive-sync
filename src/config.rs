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
    pub async fn add_exclude_pattern(
        &mut self,
        path: String,
        globs: Vec<String>,
    ) -> Result<()> {
        self.config.exclude_patterns.push(crate::types::ExcludePattern { path, globs });
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
