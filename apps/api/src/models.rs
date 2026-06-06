use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconSetSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub icon_count: usize,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconManifest {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub icons: Vec<IconEntry>,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconEntry {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub path: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub md5: String,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_config: Option<RepoConfigResponse>,
}

#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub id: String,
    pub login: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoConfigResponse {
    pub configured: bool,
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub token_configured: bool,
}

#[derive(Debug, Deserialize)]
pub struct RepoConfigRequest {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    #[serde(default)]
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSetRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSetRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RenameIconRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCollabLinkRequest {
    pub set_id: String,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCollabLinkRequest {
    #[serde(default)]
    pub expires_at: Option<Option<DateTime<Utc>>>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub clear_password: bool,
}

#[derive(Debug, Deserialize)]
pub struct CollabLinkListQuery {
    pub set_id: String,
}

#[derive(Debug, Deserialize)]
pub struct RevokeCollabLinksRequest {
    pub set_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ShareAccessInspectQuery {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct ShareAccessAuthorizeRequest {
    pub token: String,
    #[serde(default)]
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CollabLinkResponse {
    pub id: String,
    pub set_id: String,
    pub share_url: String,
    pub password_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub active: bool,
}

#[derive(Debug, Serialize)]
pub struct ShareAccessInspectResponse {
    pub set_id: String,
    pub set_name: String,
    pub password_enabled: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub active: bool,
}

#[derive(Debug, Serialize)]
pub struct ShareAccessSessionResponse {
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}
