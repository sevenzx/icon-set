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
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_token: Option<String>,
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
