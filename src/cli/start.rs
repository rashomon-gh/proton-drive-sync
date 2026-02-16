//! Start CLI command

use crate::cli::auth::load_session;
use crate::config::ConfigManager;
use crate::db::Db;
use crate::error::Result;
use crate::paths::get_data_dir;
use crate::sync::SyncEngine;
use clap::Parser;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::Mutex;
use tracing::info;

/// Start command options
#[derive(Parser, Debug)]
pub struct StartCommand {
    /// Run in foreground (don't daemonize)
    #[arg(short, long)]
    pub foreground: bool,

    /// Enable debug logging
    #[arg(long)]
    pub debug: bool,
}

impl StartCommand {
    /// Run the start command
    pub async fn run(self) -> Result<()> {
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

        // Start the engine
        engine.start().await?;

        info!("Sync engine started");

        if self.foreground {
            // Run in foreground - wait for shutdown signal
            info!("Running in foreground. Press Ctrl+C to stop.");

            #[cfg(unix)]
            {
                use signal::unix::{signal, SignalKind};
                let mut sigterm = signal(SignalKind::terminate())?;
                let mut sigint = signal(SignalKind::interrupt())?;

                tokio::select! {
                    _ = sigterm.recv() => {
                        info!("Received SIGTERM, shutting down...");
                    }
                    _ = sigint.recv() => {
                        info!("Received SIGINT, shutting down...");
                    }
                }
            }

            #[cfg(windows)]
            {
                use tokio::signal::windows::{ctrl_break, ctrl_c};
                tokio::select! {
                    _ = ctrl_c() => {
                        info!("Received Ctrl+C, shutting down...");
                    }
                    _ = ctrl_break() => {
                        info!("Received Ctrl+Break, shutting down...");
                    }
                }
            }

            engine.stop().await?;
            info!("Shutdown complete");
        } else {
            // Run as daemon
            #[cfg(target_os = "macos")]
            {
                println!("Use launchd to run as a service on macOS");
                println!("See: proton-drive-sync service install --help");
            }

            #[cfg(target_os = "linux")]
            {
                println!("Use systemd to run as a service on Linux");
                println!("See: proton-drive-sync service install --help");
            }

            #[cfg(windows)]
            {
                println!("Use Windows Service to run as a service");
                println!("See: proton-drive-sync service install --help");
            }
        }

        Ok(())
    }
}
