//! Reconcile CLI command

use crate::cli::auth::load_session;
use crate::config::ConfigManager;
use crate::db::Db;
use crate::error::Result;
use crate::paths::get_data_dir;
use crate::sync::SyncEngine;
use clap::Parser;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Reconcile command options
#[derive(Parser, Debug)]
pub struct ReconcileCommand {}

impl ReconcileCommand {
    /// Run the reconcile command
    pub async fn run(self) -> Result<()> {
        println!("Running reconciliation scan...");
        println!();

        // Load session
        let session = load_session()?;

        // Initialize database
        let data_dir = get_data_dir()?;
        let db_path = data_dir.join("proton-drive-sync.db");
        let db = Db::new(db_path).await?;

        // Load config
        let config = Arc::new(Mutex::new(ConfigManager::new().await?));

        // Create sync engine
        let engine = SyncEngine::new(db.clone(), config.clone(), session).await?;

        // Run reconciliation
        let count = engine.reconcile().await?;

        println!();
        println!("Reconciliation complete!");
        println!("Detected {} changes", count);

        Ok(())
    }
}
