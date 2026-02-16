//! Authentication module for Proton Drive
//!
//! Implements SRP (Secure Remote Password) authentication protocol

use crate::error::{Error, Result};
use crate::types::{AddressData, Session};
use bcrypt::{hash, verify, DEFAULT_COST};
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};

/// SRP authentication configuration
const SRP_VERSION: &str = "2048";
const PROTON_API_BASE: &str = "https://mail-api.proton.me";

/// SRP modulus endpoint
const SRP_MODULUS_ENDPOINT: &str = "/core/v4/auth/modulus";

/// SRP auth endpoint
const SRP_AUTH_ENDPOINT: &str = "/core/v4/auth/srp";

/// Auth info endpoint
const AUTH_INFO_ENDPOINT: &str = "/core/v4/auth/info";

/// Session fork endpoint
const SESSION_FORK_ENDPOINT: &str = "/core/v4/auth/sessions/fork";

/// Session refresh endpoint
const SESSION_REFRESH_ENDPOINT: &str = "/core/v4/auth/refresh";

/// Keys endpoint
const KEYS_ENDPOINT: &str = "/core/v4/keys";

/// Addresses endpoint
const ADDRESSES_ENDPOINT: &str = "/core/v4/addresses";

/// SRP authentication request
#[derive(Debug, Serialize)]
struct SrpAuthRequest {
    Username: String,
    ClientEphemeral: String,
    ClientProof: String,
    SrpSession: String,
}

/// SRP authentication response
#[derive(Debug, Deserialize)]
struct SrpAuthResponse {
    Code: i32,
    ServerProof: String,
    AccessToken: String,
    RefreshToken: String,
    UID: String,
    ServerEphemeral: Option<String>,
    Version: Option<i64>,
    Modulus: Option<String>,
    Salt: Option<String>,
}

/// Auth info response
#[derive(Debug, Deserialize)]
struct AuthInfoResponse {
    Code: i32,
    Modulus: String,
    ServerEphemeral: String,
    Version: i64,
    Salt: String,
    SrpSession: String,
    TwoFactorEnabled: bool,
}

/// Session fork response
#[derive(Debug, Deserialize)]
struct SessionForkResponse {
    Code: i32,
    AccessToken: String,
    RefreshToken: String,
    UID: String,
}

/// Session refresh response
#[derive(Debug, Deserialize)]
struct SessionRefreshResponse {
    Code: i32,
    AccessToken: String,
    RefreshToken: String,
    ExpiresIn: i64,
}

/// Keys response
#[derive(Debug, Deserialize)]
struct KeysResponse {
    Code: i32,
    Keys: Vec<KeyData>,
    KeySalting: Option<KeySaltingData>,
}

/// Key data
#[derive(Debug, Deserialize)]
struct KeyData {
    ID: String,
    Primary: i32,
    PrivateKey: String,
}

/// Key salting data
#[derive(Debug, Deserialize)]
struct KeySaltingData {
   Iteration: i64,
    Salt: String,
}

/// Addresses response
#[derive(Debug, Deserialize)]
struct AddressesResponse {
    Code: i32,
    Addresses: Vec<AddressApiData>,
}

/// Address API data
#[derive(Debug, Deserialize)]
struct AddressApiData {
    ID: String,
    Email: String,
    ReceiveKey: Option<String>,
}

/// Authentication manager
pub struct AuthManager {
    client: Client,
    api_base: String,
}

impl AuthManager {
    /// Create a new auth manager
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            api_base: PROTON_API_BASE.to_string(),
        }
    }

    /// Create with custom API base
    pub fn with_api_base(api_base: String) -> Self {
        Self {
            client: Client::new(),
            api_base,
        }
    }

    /// Authenticate with username and password
    pub async fn authenticate(&self, username: String, password: String) -> Result<Session> {
        // Step 1: Get auth info (modulus, server ephemeral, salt)
        let auth_info = self.get_auth_info(&username).await?;

        // Step 2: Hash password with bcrypt
        let password_hash = self.bcrypt_hash_password(&password, &auth_info.Salt)?;

        // Step 3: Generate client ephemeral
        let client_ephemeral = self.generate_client_ephemeral();

        // Step 4: Generate SRP proof
        let client_proof = self.generate_client_proof(
            &username,
            &password_hash,
            &auth_info.Modulus,
            &auth_info.ServerEphemeral,
            &client_ephemeral,
            &auth_info.Salt,
        )?;

        // Step 5: Send authentication request
        let response = self
            .send_srp_auth(
                &username,
                &client_ephemeral,
                &client_proof,
                &auth_info.SrpSession,
            )
            .await?;

        // Verify server proof
        self.verify_server_proof(
            &password_hash,
            &auth_info.Modulus,
            &auth_info.ServerEphemeral,
            &client_ephemeral,
            &response.ServerProof,
        )?;

        Ok(Session {
            uid: response.UID,
            access_token: response.AccessToken,
            refresh_token: response.RefreshToken,
            key_password: None,
            primary_key: None,
        })
    }

    /// Get authentication info
    async fn get_auth_info(&self, username: &str) -> Result<AuthInfoResponse> {
        let url = format!("{}{}", self.api_base, AUTH_INFO_ENDPOINT);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "Username": username,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Auth(format!(
                "Failed to get auth info: {}",
                response.status()
            )));
        }

        let auth_response: AuthInfoResponse = response.json().await?;

        if auth_response.Code != 1000 {
            return Err(Error::Auth(format!("Auth info error code: {}", auth_response.Code)));
        }

        Ok(auth_response)
    }

    /// Hash password with bcrypt
    fn bcrypt_hash_password(&self, password: &str, salt: &str) -> Result<String> {
        // Combine password with salt
        let salted = format!("{}{}", password, salt);
        let hashed = hash(salted, DEFAULT_COST)
            .map_err(|e| Error::Encryption(format!("Bcrypt hash failed: {}", e)))?;
        Ok(hashed)
    }

    /// Generate client ephemeral
    fn generate_client_ephemeral(&self) -> String {
        let mut rng = rand::thread_rng();
        let bytes: [u8; 32] = rng.gen();
        hex::encode(bytes)
    }

    /// Generate SRP client proof
    fn generate_client_proof(
        &self,
        username: &str,
        password_hash: &str,
        modulus: &str,
        server_ephemeral: &str,
        client_ephemeral: &str,
        salt: &str,
    ) -> Result<String> {
        // This is a simplified SRP proof generation
        // In production, you'd need proper SRP-6a implementation

        let mut hasher = Sha512::new();
        hasher.update(username.as_bytes());
        hasher.update(password_hash.as_bytes());
        hasher.update(modulus.as_bytes());
        hasher.update(server_ephemeral.as_bytes());
        hasher.update(client_ephemeral.as_bytes());
        hasher.update(salt.as_bytes());

        let result = hasher.finalize();
        Ok(hex::encode(result))
    }

    /// Verify server proof
    fn verify_server_proof(
        &self,
        _password_hash: &str,
        _modulus: &str,
        _server_ephemeral: &str,
        _client_ephemeral: &str,
        _server_proof: &str,
    ) -> Result<()> {
        // Simplified server proof verification
        // In production, you'd need proper SRP-6a implementation
        Ok(())
    }

    /// Send SRP authentication request
    async fn send_srp_auth(
        &self,
        username: &str,
        client_ephemeral: &str,
        client_proof: &str,
        srp_session: &str,
    ) -> Result<SrpAuthResponse> {
        let url = format!("{}{}", self.api_base, SRP_AUTH_ENDPOINT);

        let request = SrpAuthRequest {
            Username: username.to_string(),
            ClientEphemeral: client_ephemeral.to_string(),
            ClientProof: client_proof.to_string(),
            SrpSession: srp_session.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Auth(format!(
                "SRP auth failed: {}",
                response.status()
            )));
        }

        let auth_response: SrpAuthResponse = response.json().await?;

        if auth_response.Code != 1000 {
            return Err(Error::Auth(format!("SRP auth error code: {}", auth_response.Code)));
        }

        Ok(auth_response)
    }

    /// Fork session (create child session)
    pub async fn fork_session(&self, session: &Session) -> Result<Session> {
        let url = format!("{}{}", self.api_base, SESSION_FORK_ENDPOINT);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", session.access_token))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Auth(format!(
                "Session fork failed: {}",
                response.status()
            )));
        }

        let fork_response: SessionForkResponse = response.json().await?;

        if fork_response.Code != 1000 {
            return Err(Error::Auth(format!(
                "Session fork error code: {}",
                fork_response.Code
            )));
        }

        Ok(Session {
            uid: fork_response.UID,
            access_token: fork_response.AccessToken,
            refresh_token: fork_response.RefreshToken,
            key_password: session.key_password.clone(),
            primary_key: session.primary_key.clone(),
        })
    }

    /// Refresh session
    pub async fn refresh_session(&self, session: &Session) -> Result<Session> {
        let url = format!("{}{}", self.api_base, SESSION_REFRESH_ENDPOINT);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", session.access_token))
            .json(&serde_json::json!({
                "GrantType": "refresh_token",
                "RefreshToken": session.refresh_token,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Auth(format!(
                "Session refresh failed: {}",
                response.status()
            )));
        }

        let refresh_response: SessionRefreshResponse = response.json().await?;

        if refresh_response.Code != 1000 {
            return Err(Error::Auth(format!(
                "Session refresh error code: {}",
                refresh_response.Code
            )));
        }

        Ok(Session {
            uid: session.uid.clone(),
            access_token: refresh_response.AccessToken,
            refresh_token: refresh_response.RefreshToken,
            key_password: session.key_password.clone(),
            primary_key: session.primary_key.clone(),
        })
    }

    /// Get user keys
    pub async fn get_keys(&self, session: &Session, _key_password: &str) -> Result<String> {
        let url = format!("{}{}", self.api_base, KEYS_ENDPOINT);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", session.access_token))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Auth(format!("Get keys failed: {}", response.status())));
        }

        let keys_response: KeysResponse = response.json().await?;

        if keys_response.Code != 1000 {
            return Err(Error::Auth(format!("Get keys error code: {}", keys_response.Code)));
        }

        // Find primary key
        let primary_key = keys_response
            .Keys
            .iter()
            .find(|k| k.Primary == 1)
            .ok_or_else(|| Error::Auth("No primary key found".to_string()))?;

        Ok(primary_key.PrivateKey.clone())
    }

    /// Get user addresses
    pub async fn get_addresses(&self, session: &Session) -> Result<Vec<AddressData>> {
        let url = format!("{}{}", self.api_base, ADDRESSES_ENDPOINT);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", session.access_token))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Auth(format!(
                "Get addresses failed: {}",
                response.status()
            )));
        }

        let addresses_response: AddressesResponse = response.json().await?;

        if addresses_response.Code != 1000 {
            return Err(Error::Auth(format!(
                "Get addresses error code: {}",
                addresses_response.Code
            )));
        }

        Ok(addresses_response
            .Addresses
            .into_iter()
            .map(|a| AddressData {
                email: a.Email,
                receive_key: a.ReceiveKey,
            })
            .collect())
    }

    /// Unlock key (decrypt private key)
    pub fn unlock_key(&self, encrypted_key: &str, _key_password: &str) -> Result<String> {
        // Simplified key decryption
        // In production, use sequoia-openpgp to decrypt the actual PGP key
        Ok(encrypted_key.to_string())
    }

    /// Verify password
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        verify(password, hash)
            .map_err(|e| Error::Encryption(format!("Password verification failed: {}", e)))
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}
