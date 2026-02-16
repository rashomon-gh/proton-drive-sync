//! Resume CLI command

use crate::db::Db;
use crate::error::Result;
use crate::paths::get_data_dir;
use clap::Parser;

/// Resume command options
#[derive(Parser, Debug)]
pub struct ResumeCommand {}

impl ResumeCommand {
    /// Run the resume command
    pub async fn run(self) -> Result<()> {
        // Initialize database
        let data_dir = get_data_dir()?;
        let db_path = data_dir.join("proton-drive-sync.db");
        let db = Db::new(db_path).await?;

        // Send resume signal
        db.send_signal("resume").await?;

        // Clear paused flag
        db.clear_flag("paused").await?;

        println!("Sync resumed");

        Ok(())
    }
}
