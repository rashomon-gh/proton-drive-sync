//! Proton Drive API client

use crate::auth::AuthManager;
use crate::error::{Error, Result};
use crate::types::{CreateResult, NodeData, Session};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Proton Drive API base URL
const DRIVE_API_BASE: &str = "https://drive-api.proton.me";

/// Drive nodes endpoint
const NODES_ENDPOINT: &str = "/drive/v2/nodes";

/// Drive share endpoint
const SHARE_ENDPOINT: &str = "/drive/v2/share";

/// Drive files endpoint
const FILES_ENDPOINT: &str = "/drive/v2/files";

/// Create node request
#[derive(Debug, Serialize)]
struct CreateNodeRequest {
    ParentLinkID: String,
    NodeName: String,
    NodeType: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ContentKeyPacket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    Signature: Option<String>,
}

/// Create node response
#[derive(Debug, Deserialize)]
struct CreateNodeResponse {
    Code: i32,
    Node: Option<NodeApiResponse>,
}

/// Node API response
#[derive(Debug, Deserialize)]
struct NodeApiResponse {
    UID: String,
    ParentLinkID: String,
    Name: String,
    NodeType: String,
    State: i32,
    Hash: Option<String>,
    Size: Option<i64>,
    MIMEType: Option<String>,
    ActiveRevision: Option<RevisionApiResponse>,
}

/// Revision API response
#[derive(Debug, Deserialize)]
struct RevisionApiResponse {
    ID: String,
    Size: Option<i64>,
    ManifestSignature: Option<String>,
}

/// Delete node response
#[derive(Debug, Deserialize)]
struct DeleteNodeResponse {
    Code: i32,
}

/// Rename node request
#[derive(Debug, Serialize)]
struct RenameNodeRequest {
    Name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    Signature: Option<String>,
}

/// Rename node response
#[derive(Debug, Deserialize)]
struct RenameNodeResponse {
    Code: i32,
    Node: Option<NodeApiResponse>,
}

/// List nodes response
#[derive(Debug, Deserialize)]
struct ListNodesResponse {
    Code: i32,
    Nodes: Vec<NodeApiResponse>,
}

/// Proton Drive client
pub struct ProtonClient {
    client: Client,
    api_base: String,
    session: Session,
    auth_manager: AuthManager,
}

impl ProtonClient {
    /// Create a new Proton Drive client
    pub fn new(session: Session) -> Self {
        Self {
            client: Client::new(),
            api_base: DRIVE_API_BASE.to_string(),
            session,
            auth_manager: AuthManager::new(),
        }
    }

    /// Create with custom API base
    pub fn with_api_base(api_base: String, session: Session) -> Self {
        Self {
            client: Client::new(),
            api_base,
            session,
            auth_manager: AuthManager::new(),
        }
    }

    /// Get access token
    fn get_token(&self) -> &str {
        &self.session.access_token
    }

    /// Create a file node
    pub async fn create_file(
        &self,
        parent_id: &str,
        name: &str,
        content: Vec<u8>,
        mime_type: Option<&str>,
    ) -> Result<CreateResult> {
        let url = format!("{}{}", self.api_base, FILES_ENDPOINT);

        let mut form = reqwest::multipart::Form::new();

        form = form.text("ParentLinkID", parent_id.to_string());
        form = form.text("NodeName", name.to_string());
        form = form.text("NodeType", "file");

        if let Some(mt) = mime_type {
            form = form.text("MIMEType", mt.to_string());
        }

        let part = reqwest::multipart::Part::bytes(content)
            .file_name(name.to_string());
        form = form.part("File", part);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.get_token()))
            .multipart(form)
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    let error_text = resp.text().await.unwrap_or_default();
                    return Ok(CreateResult {
                        success: false,
                        node_uid: None,
                        error: Some(format!("HTTP {}: {}", status, error_text)),
                    });
                }

                let create_response: CreateNodeResponse = resp.json().await?;

                if create_response.Code == 1000 {
                    if let Some(node) = create_response.Node {
                        return Ok(CreateResult {
                            success: true,
                            node_uid: Some(node.UID),
                            error: None,
                        });
                    }
                }

                Ok(CreateResult {
                    success: false,
                    node_uid: None,
                    error: Some(format!("API error code: {}", create_response.Code)),
                })
            }
            Err(e) => Ok(CreateResult {
                success: false,
                node_uid: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Create a folder node
    pub async fn create_folder(&self, parent_id: &str, name: &str) -> Result<CreateResult> {
        let url = format!("{}{}", self.api_base, NODES_ENDPOINT);

        let request = CreateNodeRequest {
            ParentLinkID: parent_id.to_string(),
            NodeName: name.to_string(),
            NodeType: "folder".to_string(),
            ContentKeyPacket: None,
            Signature: None,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.get_token()))
            .json(&request)
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    let error_text = resp.text().await.unwrap_or_default();
                    return Ok(CreateResult {
                        success: false,
                        node_uid: None,
                        error: Some(format!("HTTP {}: {}", status, error_text)),
                    });
                }

                let create_response: CreateNodeResponse = resp.json().await?;

                if create_response.Code == 1000 {
                    if let Some(node) = create_response.Node {
                        return Ok(CreateResult {
                            success: true,
                            node_uid: Some(node.UID),
                            error: None,
                        });
                    }
                }

                Ok(CreateResult {
                    success: false,
                    node_uid: None,
                    error: Some(format!("API error code: {}", create_response.Code)),
                })
            }
            Err(e) => Ok(CreateResult {
                success: false,
                node_uid: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete a node (move to trash)
    pub async fn delete_node(&self, node_id: &str) -> Result<()> {
        self.delete_node_internal(node_id, false).await
    }

    /// Permanently delete a node
    pub async fn delete_node_permanent(&self, node_id: &str) -> Result<()> {
        self.delete_node_internal(node_id, true).await
    }

    /// Internal delete implementation
    async fn delete_node_internal(&self, node_id: &str, permanent: bool) -> Result<()> {
        let url = format!("{}{}/{}", self.api_base, NODES_ENDPOINT, node_id);

        let mut query = Vec::new();
        if permanent {
            query.push(("permanent", "true"));
        }

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.get_token()))
            .query(&query)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::ProtonApi(format!(
                "Delete failed: {}",
                response.status()
            )));
        }

        let delete_response: DeleteNodeResponse = response.json().await?;

        if delete_response.Code != 1000 {
            return Err(Error::ProtonApi(format!(
                "Delete error code: {}",
                delete_response.Code
            )));
        }

        Ok(())
    }

    /// Rename a node
    pub async fn rename_node(&self, node_id: &str, new_name: &str) -> Result<String> {
        let url = format!("{}{}/{}", self.api_base, NODES_ENDPOINT, node_id);

        let request = RenameNodeRequest {
            Name: new_name.to_string(),
            Signature: None,
        };

        let response = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.get_token()))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::ProtonApi(format!(
                "Rename failed: {}",
                response.status()
            )));
        }

        let rename_response: RenameNodeResponse = response.json().await?;

        if rename_response.Code != 1000 {
            return Err(Error::ProtonApi(format!(
                "Rename error code: {}",
                rename_response.Code
            )));
        }

        Ok(rename_response.Node.unwrap().UID)
    }

    /// List nodes in a folder
    pub async fn list_nodes(&self, parent_id: &str) -> Result<Vec<NodeData>> {
        let url = format!("{}{}", self.api_base, NODES_ENDPOINT);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.get_token()))
            .query(&[("ParentLinkID", parent_id)])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::ProtonApi(format!(
                "List nodes failed: {}",
                response.status()
            )));
        }

        let list_response: ListNodesResponse = response.json().await?;

        if list_response.Code != 1000 {
            return Err(Error::ProtonApi(format!(
                "List nodes error code: {}",
                list_response.Code
            )));
        }

        Ok(list_response
            .Nodes
            .into_iter()
            .map(|n| NodeData {
                uid: n.UID,
                parent_uid: Some(n.ParentLinkID),
                name: n.Name,
                node_type: n.NodeType,
                media_type: n.MIMEType,
                active_revision: n.ActiveRevision.map(|r| crate::types::RevisionData {
                    uid: r.ID,
                    size: r.Size,
                    manifest_signature: r.ManifestSignature,
                }),
            })
            .collect())
    }

    /// Get node by path
    pub async fn get_node_by_path(&self, share_id: &str, path: &str) -> Result<Option<NodeData>> {
        // This requires walking the path from root
        // For simplicity, we'll implement a basic version
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if parts.is_empty() {
            return Ok(None);
        }

        // Start from root and traverse
        let mut current_id = share_id.to_string();

        for part in parts {
            let children = self.list_nodes(&current_id).await?;

            let found = children.iter().find(|n| n.name == part);

            match found {
                Some(node) => {
                    current_id = node.uid.clone();
                }
                None => return Ok(None),
            }
        }

        // Get final node
        let children = self.list_nodes(&current_id).await?;
        Ok(children.into_iter().next())
    }

    /// Refresh session if needed
    pub async fn refresh_session(&mut self) -> Result<()> {
        self.session = self.auth_manager.refresh_session(&self.session).await?;
        Ok(())
    }

    /// Get session
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Get root node ID
    pub fn get_root_id(&self) -> String {
        // Default root ID for Proton Drive
        // In practice, you'd get this from the share info
        "root".to_string()
    }
}

/// Path utilities for Proton Drive
pub struct PathUtils;

impl PathUtils {
    /// Join Proton Drive paths
    pub fn join(base: &str, name: &str) -> String {
        let base = base.trim_end_matches('/');
        let name = name.trim_start_matches('/');

        if base.is_empty() {
            return format!("/{}", name);
        }

        format!("{}/{}", base, name)
    }

    /// Get parent path
    pub fn parent(path: &str) -> Option<String> {
        if path == "/" {
            return None;
        }

        Path::new(path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
    }

    /// Get file name from path
    pub fn filename(path: &str) -> String {
        Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string())
    }

    /// Normalize path
    pub fn normalize(path: &str) -> String {
        let path = path.trim_start_matches('/');

        if path.is_empty() {
            return "/".to_string();
        }

        format!("/{}", path.replace("//", "/"))
    }
}
