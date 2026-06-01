use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::{Method, StatusCode, header};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    config::Config,
    db::RepoConfig,
    error::{AppError, AppResult},
};

#[derive(Clone)]
pub struct GitHubClient {
    repo: GitHubRepo,
    http: reqwest::Client,
}

#[derive(Clone)]
struct GitHubRepo {
    owner: String,
    repo: String,
    branch: String,
    token: Option<String>,
    raw_base_url: String,
}

pub struct GitHubFile {
    pub sha: String,
    pub content: Vec<u8>,
}

pub struct GitHubJson<T> {
    pub sha: String,
    pub value: T,
}

#[derive(Deserialize)]
struct ContentResponse {
    sha: String,
    content: String,
}

#[derive(Serialize)]
struct PutFileRequest<'a> {
    message: &'a str,
    content: String,
    branch: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha: Option<&'a str>,
}

#[derive(Serialize)]
struct DeleteFileRequest<'a> {
    message: &'a str,
    sha: &'a str,
    branch: &'a str,
}

impl GitHubClient {
    /// 创建公开案例仓库的 GitHub 内容 API 客户端。
    pub fn from_public_config(config: &Config) -> Self {
        Self {
            repo: GitHubRepo {
                owner: config.github_owner.clone(),
                repo: config.github_repo.clone(),
                branch: config.github_branch.clone(),
                token: config.github_token.clone(),
                raw_base_url: raw_base_url(
                    &config.github_owner,
                    &config.github_repo,
                    &config.github_branch,
                ),
            },
            http: reqwest::Client::new(),
        }
    }

    /// 创建当前登录用户配置仓库的 GitHub 内容 API 客户端。
    pub fn from_repo_config(config: RepoConfig) -> Self {
        let raw_base_url = raw_base_url(&config.owner, &config.repo, &config.branch);

        Self {
            repo: GitHubRepo {
                owner: config.owner,
                repo: config.repo,
                branch: config.branch,
                token: Some(config.token),
                raw_base_url,
            },
            http: reqwest::Client::new(),
        }
    }

    /// 读取仓库文件内容，404 会返回 None。
    pub async fn get_file(&self, path: &str) -> AppResult<Option<GitHubFile>> {
        let url = self.content_url(path);
        let response = self
            .request(Method::GET, url)
            .query(&[("ref", self.repo.branch.as_str())])
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            return Err(self.github_status_error(response).await);
        }

        let payload = response.json::<ContentResponse>().await?;
        let compact_content = payload.content.lines().collect::<String>();
        let content = STANDARD.decode(compact_content)?;

        Ok(Some(GitHubFile {
            sha: payload.sha,
            content,
        }))
    }

    /// 读取仓库 JSON 文件并反序列化。
    pub async fn get_json<T>(&self, path: &str) -> AppResult<Option<GitHubJson<T>>>
    where
        T: DeserializeOwned,
    {
        let Some(file) = self.get_file(path).await? else {
            return Ok(None);
        };
        let value = serde_json::from_slice::<T>(&file.content)?;

        Ok(Some(GitHubJson {
            sha: file.sha,
            value,
        }))
    }

    /// 新建或更新仓库文件。
    pub async fn put_file(
        &self,
        path: &str,
        content: &[u8],
        message: &str,
        sha: Option<&str>,
    ) -> AppResult<()> {
        self.ensure_write_token()?;

        let payload = PutFileRequest {
            message,
            content: STANDARD.encode(content),
            branch: &self.repo.branch,
            sha,
        };
        let response = self
            .request(Method::PUT, self.content_url(path))
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.github_status_error(response).await);
        }

        Ok(())
    }

    /// 删除仓库文件。
    pub async fn delete_file(&self, path: &str, sha: &str, message: &str) -> AppResult<()> {
        self.ensure_write_token()?;

        let payload = DeleteFileRequest {
            message,
            sha,
            branch: &self.repo.branch,
        };
        let response = self
            .request(Method::DELETE, self.content_url(path))
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.github_status_error(response).await);
        }

        Ok(())
    }

    /// 返回仓库文件对应的 raw 地址。
    pub fn raw_url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.repo.raw_base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    /// 构建 GitHub contents API 地址。
    fn content_url(&self, path: &str) -> String {
        let encoded_path = path
            .trim_start_matches('/')
            .split('/')
            .map(|part| urlencoding::encode(part).into_owned())
            .collect::<Vec<_>>()
            .join("/");

        format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            self.repo.owner, self.repo.repo, encoded_path
        )
    }

    /// 构建带 GitHub 标准请求头的 RequestBuilder。
    fn request(&self, method: Method, url: String) -> reqwest::RequestBuilder {
        let mut builder = self
            .http
            .request(method, url)
            .header(header::USER_AGENT, "icon-set-api")
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");

        if let Some(token) = &self.repo.token {
            builder = builder.bearer_auth(token);
        }

        builder
    }

    /// 写操作前确认已经配置 GitHub Token。
    fn ensure_write_token(&self) -> AppResult<()> {
        if self.repo.token.is_none() {
            return Err(AppError::BadRequest(
                "缺少 GitHub Token，无法写入 GitHub".to_string(),
            ));
        }

        Ok(())
    }

    /// 将 GitHub 非成功响应转成可读错误。
    async fn github_status_error(&self, response: reqwest::Response) -> AppError {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<empty>".to_string());
        let repo = format!("{}/{}", self.repo.owner, self.repo.repo);
        let compact_body = body.chars().take(500).collect::<String>();
        let message = match status {
            StatusCode::UNAUTHORIZED => {
                "GitHub Token 无效或已过期，请重新生成后在后台保存".to_string()
            }
            StatusCode::FORBIDDEN => format!(
                "GitHub 拒绝写入 {repo}，请确认 Token 对该仓库有 Contents: Read and write 权限"
            ),
            StatusCode::NOT_FOUND => format!(
                "GitHub 找不到 {repo} 或分支 {}，也可能是当前 Token 没有授权访问这个仓库",
                self.repo.branch
            ),
            StatusCode::CONFLICT => "GitHub 文件版本冲突，请刷新后重试".to_string(),
            _ => format!("{status}: {compact_body}"),
        };

        AppError::GitHub(message)
    }
}

fn raw_base_url(owner: &str, repo: &str, branch: &str) -> String {
    format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}")
}
