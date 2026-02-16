//! Pause CLI command

use crate::db::Db;
use crate::error::Result;
use crate::paths::get_data_dir;
use clap::Parser;

/// Pause command options
#[derive(Parser, Debug)]
pub struct PauseCommand {}

impl PauseCommand {
    /// Run the pause command
    pub async fn run(self) -> Result<()> {
        // Initialize database
        let data_dir = get_data_dir()?;
        let db_path = data_dir.join("proton-drive-sync.db");
        let db = Db::new(db_path).await?;

        // Send pause signal
        db.send_signal("pause").await?;

        // Set paused flag
        db.set_flag("paused").await?;

        println!("Sync paused");

        Ok(())
    }
}
