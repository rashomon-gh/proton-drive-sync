//! Dashboard CLI command

use crate::config::ConfigManager;
use crate::error::Result;
use clap::Parser;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Dashboard command options
#[derive(Parser, Debug)]
pub struct DashboardCommand {
    /// Dashboard host
    #[arg(short, long, default_value = "127.0.0.1")]
    pub host: String,

    /// Dashboard port
    #[arg(short, long, default_value_t = 4242)]
    pub port: u16,
}

impl DashboardCommand {
    /// Run the dashboard command
    pub async fn run(self) -> Result<()> {
        println!("Starting dashboard at http://{}:{}", self.host, self.port);
        println!("Press Ctrl+C to stop");
        println!();

        // Load config
        let config = Arc::new(Mutex::new(ConfigManager::new().await?));

        // Start dashboard server
        crate::dashboard::start_dashboard(config, self.host, self.port).await?;

        Ok(())
    }
}
