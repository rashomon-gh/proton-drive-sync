//! Stop CLI command

use crate::db::Db;
use crate::error::Result;
use crate::paths::get_data_dir;
use clap::Parser;

/// Stop command options
#[derive(Parser, Debug)]
pub struct StopCommand {}

impl StopCommand {
    /// Run the stop command
    pub async fn run(self) -> Result<()> {
        // Initialize database
        let data_dir = get_data_dir()?;
        let db_path = data_dir.join("proton-drive-sync.db");
        let db = Db::new(db_path).await?;

        // Send stop signal
        db.send_signal("stop").await?;

        println!("Stop signal sent");

        // Clear running flag
        db.clear_flag("running").await?;

        println!("Sync engine stopped");

        Ok(())
    }
}
