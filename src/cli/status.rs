//! Status CLI command

use crate::db::Db;
use crate::error::Result;
use crate::paths::get_data_dir;
use crate::types::SyncJobStatus;
use clap::Parser;

/// Status command options
#[derive(Parser, Debug)]
pub struct StatusCommand {
    /// Show detailed output
    #[arg(short, long)]
    pub verbose: bool,
}

impl StatusCommand {
    /// Run the status command
    pub async fn run(self) -> Result<()> {
        // Initialize database
        let data_dir = get_data_dir()?;
        let db_path = data_dir.join("proton-drive-sync.db");
        let db = Db::new(db_path).await?;

        // Check if running
        let running = db.get_flag("running").await?;
        let paused = db.get_flag("paused").await?;

        println!("Proton Drive Sync Status");
        println!("========================");
        println!();

        if !running {
            println!("Status: Stopped");
            println!();
            println!("Start the sync engine with: proton-drive-sync start");
            return Ok(());
        }

        if paused {
            println!("Status: Paused");
            println!();
            println!("Resume with: proton-drive-sync resume");
        } else {
            println!("Status: Running");
        }

        println!();

        // Get job counts
        let pending = db.get_job_count(SyncJobStatus::Pending).await? as usize;
        let processing = db.get_job_count(SyncJobStatus::Processing).await? as usize;
        let synced = db.get_job_count(SyncJobStatus::Synced).await? as usize;
        let blocked = db.get_job_count(SyncJobStatus::Blocked).await? as usize;

        println!("Queue Status:");
        println!("  Pending: {}", pending);
        println!("  Processing: {}", processing);
        println!("  Synced: {}", synced);
        println!("  Blocked: {}", blocked);

        if self.verbose && blocked > 0 {
            println!();
            println!("Blocked jobs:");
            // In a full implementation, you'd list the blocked jobs with their errors
        }

        Ok(())
    }
}
