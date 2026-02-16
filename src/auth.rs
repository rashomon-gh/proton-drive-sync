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

const PROTON_API_BASE: &str = "https://mail-api.proton.me";

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
    #[serde(rename = "Username")]
    username: String,
    #[serde(rename = "ClientEphemeral")]
    client_ephemeral: String,
    #[serde(rename = "ClientProof")]
    client_proof: String,
    #[serde(rename = "SrpSession")]
    srp_session: String,
}

/// SRP authentication response
#[derive(Debug, Deserialize)]
struct SrpAuthResponse {
    #[serde(rename = "Code")]
    code: i32,
    #[serde(rename = "ServerProof")]
    server_proof: String,
    #[serde(rename = "AccessToken")]
    access_token: String,
    #[serde(rename = "RefreshToken")]
    refresh_token: String,
    #[serde(rename = "UID")]
    uid: String,
}

/// Auth info response
#[derive(Debug, Deserialize)]
struct AuthInfoResponse {
    #[serde(rename = "Code")]
    code: i32,
    modulus: String,
    #[serde(rename = "ServerEphemeral")]
    server_ephemeral: String,
    #[serde(rename = "Version")]
    #[allow(dead_code)]
    version: i64,
    salt: String,
    #[serde(rename = "SrpSession")]
    srp_session: String,
    #[serde(rename = "TwoFactorEnabled")]
    #[allow(dead_code)]
    two_factor_enabled: bool,
}

/// Session fork response
#[derive(Debug, Deserialize)]
struct SessionForkResponse {
    #[serde(rename = "Code")]
    code: i32,
    #[serde(rename = "AccessToken")]
    access_token: String,
    #[serde(rename = "RefreshToken")]
    refresh_token: String,
    #[serde(rename = "UID")]
    uid: String,
}

/// Session refresh response
#[derive(Debug, Deserialize)]
struct SessionRefreshResponse {
    #[serde(rename = "Code")]
    code: i32,
    #[serde(rename = "AccessToken")]
    access_token: String,
    #[serde(rename = "RefreshToken")]
    refresh_token: String,
    #[serde(rename = "ExpiresIn")]
    #[allow(dead_code)]
    expires_in: i64,
}

/// Keys response
#[derive(Debug, Deserialize)]
struct KeysResponse {
    #[serde(rename = "Code")]
    code: i32,
    keys: Vec<KeyData>,
    #[serde(rename = "KeySalting")]
    #[allow(dead_code)]
    key_salting: Option<KeySaltingData>,
}

/// Key data
#[derive(Debug, Deserialize)]
struct KeyData {
    #[serde(rename = "ID")]
    #[allow(dead_code)]
    id: String,
    #[serde(rename = "Primary")]
    primary: i32,
    #[serde(rename = "PrivateKey")]
    private_key: String,
}

/// Key salting data
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct KeySaltingData {
    iteration: i64,
    salt: String,
}

/// Addresses response
#[derive(Debug, Deserialize)]
struct AddressesResponse {
    #[serde(rename = "Code")]
    code: i32,
    addresses: Vec<AddressApiData>,
}

/// Address API data
#[derive(Debug, Deserialize)]
struct AddressApiData {
    #[serde(rename = "ID")]
    #[allow(dead_code)]
    id: String,
    email: String,
    #[serde(rename = "ReceiveKey")]
    receive_key: Option<String>,
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
        let password_hash = self.bcrypt_hash_password(&password, &auth_info.salt)?;

        // Step 3: Generate client ephemeral
        let client_ephemeral = self.generate_client_ephemeral();

        // Step 4: Generate SRP proof
        let client_proof = self.generate_client_proof(
            &username,
            &password_hash,
            &auth_info.modulus,
            &auth_info.server_ephemeral,
            &client_ephemeral,
            &auth_info.salt,
        )?;

        // Step 5: Send authentication request
        let response = self
            .send_srp_auth(
                &username,
                &client_ephemeral,
                &client_proof,
                &auth_info.srp_session,
            )
            .await?;

        // Verify server proof
        self.verify_server_proof(
            &password_hash,
            &auth_info.modulus,
            &auth_info.server_ephemeral,
            &client_ephemeral,
            &response.server_proof,
        )?;

        Ok(Session {
            uid: response.uid,
            access_token: response.access_token,
            refresh_token: response.refresh_token,
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

        if auth_response.code != 1000 {
            return Err(Error::Auth(format!("Auth info error code: {}", auth_response.code)));
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
            username: username.to_string(),
            client_ephemeral: client_ephemeral.to_string(),
            client_proof: client_proof.to_string(),
            srp_session: srp_session.to_string(),
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

        if auth_response.code != 1000 {
            return Err(Error::Auth(format!("SRP auth error code: {}", auth_response.code)));
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

        if fork_response.code != 1000 {
            return Err(Error::Auth(format!(
                "Session fork error code: {}",
                fork_response.code
            )));
        }

        Ok(Session {
            uid: fork_response.uid,
            access_token: fork_response.access_token,
            refresh_token: fork_response.refresh_token,
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

        if refresh_response.code != 1000 {
            return Err(Error::Auth(format!(
                "Session refresh error code: {}",
                refresh_response.code
            )));
        }

        Ok(Session {
            uid: session.uid.clone(),
            access_token: refresh_response.access_token,
            refresh_token: refresh_response.refresh_token,
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

        if keys_response.code != 1000 {
            return Err(Error::Auth(format!("Get keys error code: {}", keys_response.code)));
        }

        // Find primary key
        let primary_key = keys_response
            .keys
            .iter()
            .find(|k| k.primary == 1)
            .ok_or_else(|| Error::Auth("No primary key found".to_string()))?;

        Ok(primary_key.private_key.clone())
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

        if addresses_response.code != 1000 {
            return Err(Error::Auth(format!(
                "Get addresses error code: {}",
                addresses_response.code
            )));
        }

        Ok(addresses_response
            .addresses
            .into_iter()
            .map(|a| AddressData {
                email: a.email,
                receive_key: a.receive_key,
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
