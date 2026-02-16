//! Proton Drive Sync - Main entry point

use clap::{Parser, Subcommand};
use proton_drive_sync::cli;

#[derive(Parser)]
#[command(name = "proton-drive-sync")]
#[command(about = "Sync local files to Proton Drive", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable debug logging
    #[arg(long, global = true)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with Proton
    Auth {
        #[command(subcommand)]
        command: cli::AuthCommand,
    },
    /// Configure sync settings
    Config {
        #[command(subcommand)]
        command: cli::ConfigCommand,
    },
    /// Start the sync daemon
    Start(cli::StartCommand),
    /// Stop the sync daemon
    Stop(cli::StopCommand),
    /// Show sync status
    Status(cli::StatusCommand),
    /// Pause syncing
    Pause(cli::PauseCommand),
    /// Resume syncing
    Resume(cli::ResumeCommand),
    /// Run reconciliation scan
    Reconcile(cli::ReconcileCommand),
    /// Reset sync data
    Reset(cli::ResetCommand),
    /// View logs
    Logs(cli::LogsCommand),
    /// Start web dashboard
    Dashboard(cli::DashboardCommand),
    /// Interactive setup wizard
    Setup(cli::SetupCommand),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logger
    let log_dir = proton_drive_sync::paths::get_log_dir()?;
    if cli.debug {
        proton_drive_sync::logger::init(true)?;
    } else {
        proton_drive_sync::logger::init_with_file(log_dir, false)?;
    }

    // Run command
    let result = match cli.command {
        Commands::Auth { command } => command.run().await,
        Commands::Config { command } => command.run().await,
        Commands::Start(cmd) => cmd.run().await,
        Commands::Stop(cmd) => cmd.run().await,
        Commands::Status(cmd) => cmd.run().await,
        Commands::Pause(cmd) => cmd.run().await,
        Commands::Resume(cmd) => cmd.run().await,
        Commands::Reconcile(cmd) => cmd.run().await,
        Commands::Reset(cmd) => cmd.run().await,
        Commands::Logs(cmd) => cmd.run().await,
        Commands::Dashboard(cmd) => cmd.run().await,
        Commands::Setup(cmd) => cmd.run().await,
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
