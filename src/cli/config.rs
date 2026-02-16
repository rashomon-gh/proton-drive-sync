//! Config CLI command

use crate::config::ConfigManager;
use crate::error::Result;
use clap::Subcommand;
use inquire::{Confirm, Text};

/// Config command
#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Show current configuration
    Show,
    /// Add a sync directory
    AddDir,
    /// Remove a sync directory
    RemoveDir,
    /// Set sync concurrency
    SetConcurrency {
        /// Number of concurrent uploads
        value: usize,
    },
    /// Set remote delete behavior
    SetDeleteBehavior {
        /// Behavior: trash or permanent
        behavior: String,
    },
}

impl ConfigCommand {
    /// Run the config command
    pub async fn run(self) -> Result<()> {
        let mut config = ConfigManager::new().await?;

        match &self {
            Self::Show => self.show(&config).await,
            Self::AddDir => self.add_dir(&mut config).await,
            Self::RemoveDir => self.remove_dir(&mut config).await,
            Self::SetConcurrency { value } => self.set_concurrency(&mut config, *value).await,
            Self::SetDeleteBehavior { behavior } => {
                self.set_delete_behavior(&mut config, behavior).await
            }
        }
    }

    /// Show configuration
    async fn show(&self, config: &ConfigManager) -> Result<()> {
        let cfg = config.get();

        println!("Proton Drive Sync Configuration");
        println!("================================");
        println!();

        println!("Sync Directories:");
        if cfg.sync_dirs.is_empty() {
            println!("  (none configured)");
        } else {
            for (i, dir) in cfg.sync_dirs.iter().enumerate() {
                println!("  {}. {} -> {}", i + 1, dir.source_path, dir.remote_root);
            }
        }

        println!();
        println!("Concurrency: {}", cfg.sync_concurrency);
        println!("Remote Delete Behavior: {:?}", cfg.remote_delete_behavior);

        println!();
        println!("Dashboard: {}:{}", cfg.dashboard_host, cfg.dashboard_port);

        if !cfg.exclude_patterns.is_empty() {
            println!();
            println!("Exclude Patterns:");
            for (i, pattern) in cfg.exclude_patterns.iter().enumerate() {
                println!("  {}. path: {}", i + 1, pattern.path);
                for glob in &pattern.globs {
                    println!("     - {}", glob);
                }
            }
        }

        Ok(())
    }

    /// Add a sync directory
    async fn add_dir(&self, config: &mut ConfigManager) -> Result<()> {
        let source = Text::new("Local path to sync:")
            .prompt()
            .map_err(|e| crate::error::Error::Config(format!("Prompt error: {}", e)))?;

        let remote = Text::new("Remote Proton Drive path:")
            .prompt()
            .map_err(|e| crate::error::Error::Config(format!("Prompt error: {}", e)))?;

        config.add_sync_dir(source, remote).await?;

        println!("✓ Added sync directory");

        Ok(())
    }

    /// Remove a sync directory
    async fn remove_dir(&self, config: &mut ConfigManager) -> Result<()> {
        let cfg = config.get();

        if cfg.sync_dirs.is_empty() {
            println!("No sync directories configured.");
            return Ok(());
        }

        let options: Vec<String> = cfg
            .sync_dirs
            .iter()
            .map(|d| format!("{} -> {}", d.source_path, d.remote_root))
            .collect();

        let selected = inquire::Select::new("Select sync directory to remove:", options)
            .prompt()
            .map_err(|e| crate::error::Error::Config(format!("Prompt error: {}", e)))?;

        let index = cfg
            .sync_dirs
            .iter()
            .position(|d| format!("{} -> {}", d.source_path, d.remote_root) == selected)
            .unwrap();

        config.remove_sync_dir(index).await?;

        println!("✓ Removed sync directory");

        Ok(())
    }

    /// Set concurrency
    async fn set_concurrency(&self, config: &mut ConfigManager, value: usize) -> Result<()> {
        config.set_concurrency(value).await?;
        println!("✓ Set concurrency to {}", value);
        Ok(())
    }

    /// Set delete behavior
    async fn set_delete_behavior(&self, config: &mut ConfigManager, behavior: &str) -> Result<()> {
        let behavior_value = match behavior.to_lowercase().as_str() {
            "trash" => crate::types::RemoteDeleteBehavior::Trash,
            "permanent" => crate::types::RemoteDeleteBehavior::Permanent,
            _ => {
                println!("Invalid behavior. Use 'trash' or 'permanent'.");
                return Ok(());
            }
        };

        config.set_remote_delete_behavior(behavior_value).await?;
        println!("✓ Set remote delete behavior to {:?}", behavior_value);
        Ok(())
    }
}
