use std::{env, net::SocketAddr, path::Path};

use thiserror::Error;

#[derive(Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub admin_password: String,
    pub github_token: Option<String>,
    pub github_owner: String,
    pub github_repo: String,
    pub github_branch: String,
    pub raw_base_url: String,
    pub cors_origin: String,
    pub cookie_secure: bool,
    pub session_cookie_name: String,
    pub max_upload_bytes: usize,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("缺少环境变量 {0}")]
    Missing(&'static str),
    #[error("环境变量 {name} 格式无效：{reason}")]
    Invalid { name: &'static str, reason: String },
}

impl Config {
    /// 从环境变量加载后端运行配置。
    pub fn from_env() -> Result<Self, ConfigError> {
        // 本地开发时优先使用 apps/api/.env，避免 shell 中的 GITHUB_TOKEN 覆盖项目配置。
        if Path::new("apps/api/.env").exists() {
            dotenvy::from_path_override("apps/api/.env").ok();
        } else {
            dotenvy::dotenv_override().ok();
        }

        let bind_addr = env_or_default("API_BIND_ADDR", "127.0.0.1:3000")
            .parse::<SocketAddr>()
            .map_err(|err| ConfigError::Invalid {
                name: "API_BIND_ADDR",
                reason: err.to_string(),
            })?;
        let admin_password = required_env("ADMIN_PASSWORD")?;
        let github_owner = required_env("GITHUB_OWNER")?;
        let github_repo = required_env("GITHUB_REPO")?;
        let github_branch = env_or_default("GITHUB_BRANCH", "main");
        let raw_base_url = env::var("RAW_BASE_URL").unwrap_or_else(|_| {
            format!(
                "https://raw.githubusercontent.com/{}/{}/{}",
                github_owner, github_repo, github_branch
            )
        });
        let cors_origin = env_or_default("CORS_ORIGIN", "http://localhost:5173");
        let cookie_secure = parse_bool_env("COOKIE_SECURE", false)?;
        let max_upload_bytes = parse_usize_env("MAX_UPLOAD_BYTES", 5 * 1024 * 1024)?;

        Ok(Self {
            bind_addr,
            admin_password,
            github_token: env::var("GITHUB_TOKEN")
                .ok()
                .filter(|value| !value.is_empty()),
            github_owner,
            github_repo,
            github_branch,
            raw_base_url,
            cors_origin,
            cookie_secure,
            session_cookie_name: "icon_set_session".to_string(),
            max_upload_bytes,
        })
    }

    /// 拼出某个仓库文件对应的 raw.githubusercontent.com 地址。
    pub fn raw_url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.raw_base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}

/// 读取必填环境变量，缺失时返回明确错误。
fn required_env(name: &'static str) -> Result<String, ConfigError> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or(ConfigError::Missing(name))
}

/// 读取环境变量，缺失时使用默认值。
fn env_or_default(name: &str, default_value: &str) -> String {
    env::var(name).unwrap_or_else(|_| default_value.to_string())
}

/// 解析布尔环境变量，支持 true/false 和 1/0。
fn parse_bool_env(name: &'static str, default_value: bool) -> Result<bool, ConfigError> {
    match env::var(name) {
        Ok(value) => match value.as_str() {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            _ => Err(ConfigError::Invalid {
                name,
                reason: "只支持 true/false 或 1/0".to_string(),
            }),
        },
        Err(_) => Ok(default_value),
    }
}

/// 解析 usize 环境变量，缺失时使用默认值。
fn parse_usize_env(name: &'static str, default_value: usize) -> Result<usize, ConfigError> {
    match env::var(name) {
        Ok(value) => value.parse::<usize>().map_err(|err| ConfigError::Invalid {
            name,
            reason: err.to_string(),
        }),
        Err(_) => Ok(default_value),
    }
}
