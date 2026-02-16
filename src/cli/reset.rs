//! Reset CLI command

use crate::db::Db;
use crate::error::Result;
use crate::paths::get_data_dir;
use clap::Parser;
use inquire::Confirm;

/// Reset command options
#[derive(Parser, Debug)]
pub struct ResetCommand {
    /// Purge all data including configuration
    #[arg(long)]
    pub purge: bool,
}

impl ResetCommand {
    /// Run the reset command
    pub async fn run(self) -> Result<()> {
        println!("This will stop the sync engine and clear all sync history.");

        if self.purge {
            println!("Purge mode: This will also remove all configuration and credentials.");
        }

        println!();

        let confirm = Confirm::new("Are you sure?")
            .with_default(false)
            .prompt()
            .map_err(|e| crate::error::Error::Config(format!("Prompt error: {}", e)))?;

        if !confirm {
            println!("Reset cancelled.");
            return Ok(());
        }

        // Stop sync engine
        let data_dir = get_data_dir()?;
        let db_path = data_dir.join("proton-drive-sync.db");

        if db_path.exists() {
            let db = Db::new(db_path.clone()).await?;

            // Send stop signal
            db.send_signal("stop").await.ok();

            // Clear flags
            db.clear_flag("running").await.ok();
            db.clear_flag("paused").await.ok();

            println!("✓ Sync engine stopped");
        }

        if self.purge {
            // Remove database
            tokio::fs::remove_file(&db_path).await.ok();
            println!("✓ Database cleared");

            // Remove configuration
            let config_dir = dirs::config_dir()
                .map(|d| d.join("proton-drive-sync"))
                .unwrap_or_default();

            let config_file = config_dir.join("config.json");
            tokio::fs::remove_file(&config_file).await.ok();
            println!("✓ Configuration cleared");

            // Remove credentials
            let entry = keyring::Entry::new("proton-drive-sync", "credentials")?;
            let _ = entry.delete_credential();
            println!("✓ Credentials cleared");
        } else {
            // Just clear the database (keep config and credentials)
            if db_path.exists() {
                tokio::fs::remove_file(&db_path).await?;
                println!("✓ Sync history cleared");
            }
        }

        println!();
        println!("Reset complete!");

        if self.purge {
            println!("Run 'proton-drive-sync auth login' to set up again.");
        } else {
            println!("Your configuration and credentials are preserved.");
            println!("Run 'proton-drive-sync start' to begin syncing again.");
        }

        Ok(())
    }
}
