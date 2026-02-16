//! Logs CLI command

use crate::error::Result;
use crate::paths::get_log_dir;
use clap::Parser;
use std::path::Path;

/// Logs command options
#[derive(Parser, Debug)]
pub struct LogsCommand {
    /// Number of lines to show
    #[arg(short, long, default_value_t = 50)]
    pub lines: usize,

    /// Follow log output
    #[arg(short, long)]
    pub follow: bool,
}

impl LogsCommand {
    /// Run the logs command
    pub async fn run(self) -> Result<()> {
        let log_dir = get_log_dir()?;

        if !log_dir.exists() {
            println!("No logs found. Has the sync engine been started?");
            return Ok(());
        }

        // Find the latest log file
        let mut entries = tokio::fs::read_dir(&log_dir).await?;
        let mut log_files_with_meta = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map(|e| e == "log").unwrap_or(false) {
                let modified = entry.metadata().await.ok().and_then(|m| m.modified().ok());
                log_files_with_meta.push((entry, modified));
            }
        }

        log_files_with_meta.sort_by_key(|(_, m)| *m);

        if let Some((latest, _)) = log_files_with_meta.last() {
            let log_path = latest.path();

            if self.follow {
                self.follow_log(&log_path).await?;
            } else {
                self.show_log_tail(&log_path, self.lines).await?;
            }
        } else {
            println!("No log files found.");
        }

        Ok(())
    }

    /// Show tail of log file
    async fn show_log_tail(&self, path: &Path, lines: usize) -> Result<()> {
        let content = tokio::fs::read_to_string(path).await?;

        let log_lines: Vec<&str> = content.lines().collect();
        let start = if log_lines.len() > lines {
            log_lines.len() - lines
        } else {
            0
        };

        for line in log_lines.iter().skip(start) {
            println!("{}", line);
        }

        Ok(())
    }

    /// Follow log file
    async fn follow_log(&self, path: &Path) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, BufReader};

        let file = tokio::fs::File::open(path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            println!("{}", line);
        }

        Ok(())
    }
}
