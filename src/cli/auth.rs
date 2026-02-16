//! Authentication CLI command

use crate::auth::AuthManager;
use crate::config::ConfigManager;
use crate::db::Db;
use crate::error::Result;
use crate::paths::{get_data_dir, get_log_dir};
use crate::types::Session;
use clap::Subcommand;
use inquire::{Password, Text};
use keyring::Entry;
use tracing::info;

/// Authentication command
#[derive(Subcommand, Debug)]
pub enum AuthCommand {
    /// Authenticate with Proton
    Login,
    /// Logout and clear credentials
    Logout,
}

impl AuthCommand {
    /// Run the auth command
    pub async fn run(self) -> Result<()> {
        match self {
            Self::Login => self.login().await,
            Self::Logout => self.logout().await,
        }
    }

    /// Login to Proton
    async fn login(&self) -> Result<()> {
        println!("Proton Drive Authentication");
        println!("============================");
        println!();

        // Get username
        let username = Text::new("Email or username:")
            .prompt()
            .map_err(|e| crate::error::Error::Auth(format!("Prompt error: {}", e)))?;

        // Get password
        let password = Password::new("Password:")
            .prompt()
            .map_err(|e| crate::error::Error::Auth(format!("Prompt error: {}", e)))?;

        println!();
        println!("Authenticating...");

        // Authenticate
        let auth_manager = AuthManager::new();
        let session = auth_manager.authenticate(username, password).await?;

        println!("✓ Authentication successful");

        // Check for 2FA
        // In a full implementation, you'd prompt for 2FA code here

        // Store credentials in keyring
        let entry = Entry::new("proton-drive-sync", "credentials")?;
        let credential_json = serde_json::to_string(&session)?;
        entry.set_password(&credential_json)?;

        println!("✓ Credentials saved securely");

        // Initialize database
        let data_dir = get_data_dir()?;
        let db_path = data_dir.join("proton-drive-sync.db");
        let _db = Db::new(db_path).await?;

        println!();
        println!("Setup complete! Run 'proton-drive-sync setup' to configure sync directories.");

        Ok(())
    }

    /// Logout from Proton
    async fn logout(&self) -> Result<()> {
        println!("Clearing Proton credentials...");

        // Remove credentials from keyring
        let entry = Entry::new("proton-drive-sync", "credentials")?;
        let _ = entry.delete_credential();

        println!("✓ Credentials cleared");

        Ok(())
    }
}

/// Load session from keyring
pub fn load_session() -> Result<Session> {
    let entry = Entry::new("proton-drive-sync", "credentials")?;
    let credential_json = entry.get_password()?;
    let session: Session = serde_json::from_str(&credential_json)?;
    Ok(session)
}

/// Check if user is authenticated
pub fn is_authenticated() -> bool {
    let entry = Entry::new("proton-drive-sync", "credentials");
    entry.ok().and_then(|e| e.get_password().ok()).is_some()
}
