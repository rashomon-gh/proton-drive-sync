//! Setup CLI command

use crate::config::ConfigManager;
use crate::error::Result;
use crate::types::{RemoteDeleteBehavior, SyncDir};
use clap::Parser;
use inquire::{Confirm, Select, Text};

/// Setup command options
#[derive(Parser, Debug)]
pub struct SetupCommand {
    /// Skip interactive setup
    #[arg(long)]
    pub non_interactive: bool,
}

impl SetupCommand {
    /// Run the setup command
    pub async fn run(self) -> Result<()> {
        if !super::auth::is_authenticated() {
            println!("Please authenticate first:");
            println!("  proton-drive-sync auth login");
            println!();
            return Ok(());
        }

        println!("Proton Drive Sync Setup");
        println!("=======================");
        println!();

        let mut config = ConfigManager::new().await?;

        // Check if already configured
        if !config.get().sync_dirs.is_empty() {
            let overwrite = Confirm::new("Existing configuration found. Overwrite?")
                .with_default(false)
                .prompt()
                .map_err(|e| crate::error::Error::Config(format!("Prompt error: {}", e)))?;

            if !overwrite {
                println!("Setup cancelled.");
                return Ok(());
            }
        }

        // Add sync directories
        println!("Add sync directories (you can add more later):");
        println!();

        let mut added_dirs = 0;

        loop {
            let source = Text::new("Local path to sync:")
                .with_placeholder(&format!("{}/Documents", std::env::var("HOME").unwrap_or_default()))
                .prompt()
                .map_err(|e| crate::error::Error::Config(format!("Prompt error: {}", e)))?;

            let remote = Text::new("Remote Proton Drive path:")
                .with_placeholder("/My Files")
                .prompt()
                .map_err(|e| crate::error::Error::Config(format!("Prompt error: {}", e)))?;

            config.add_sync_dir(source, remote).await?;
            added_dirs += 1;

            println!("✓ Added sync directory");

            let add_more = Confirm::new("Add another sync directory?")
                .with_default(false)
                .prompt()
                .map_err(|e| crate::error::Error::Config(format!("Prompt error: {}", e)))?;

            if !add_more {
                break;
            }
        }

        // Set concurrency
        println!();
        let concurrency_opts = vec!["1 (sequential)", "2", "4 (default)", "8", "16"];
        let concurrency = Select::new("Number of concurrent uploads:", concurrency_opts)
            .prompt()
            .map_err(|e| crate::error::Error::Config(format!("Prompt error: {}", e)))?;

        let concurrency_val = match concurrency {
            "1 (sequential)" => 1,
            "2" => 2,
            "4 (default)" => 4,
            "8" => 8,
            "16" => 16,
            _ => 4,
        };

        config.set_concurrency(concurrency_val).await?;
        println!("✓ Set concurrency to {}", concurrency_val);

        // Set delete behavior
        println!();
        let delete_opts = vec![
            "Move to trash (default)",
            "Delete permanently",
        ];
        let delete_behavior = Select::new("Remote delete behavior:", delete_opts)
            .prompt()
            .map_err(|e| crate::error::Error::Config(format!("Prompt error: {}", e)))?;

        let behavior = match delete_behavior {
            "Move to trash (default)" => RemoteDeleteBehavior::Trash,
            "Delete permanently" => RemoteDeleteBehavior::Permanent,
            _ => RemoteDeleteBehavior::Trash,
        };

        config.set_remote_delete_behavior(behavior).await?;
        println!("✓ Set delete behavior to {:?}", behavior);

        println!();
        println!("Setup complete!");
        println!("Added {} sync directory(s)", added_dirs);
        println!();
        println!("Next steps:");
        println!("  proton-drive-sync start    - Start the sync daemon");
        println!("  proton-drive-sync status  - Check sync status");
        println!("  proton-drive-sync help    - See all commands");

        Ok(())
    }
}
