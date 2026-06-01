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
