use reqwest::header;
use serde::{Deserialize, Serialize};
use urlencoding::encode;

use crate::{
    config::Config,
    db::GithubUserInput,
    error::{AppError, AppResult},
};

#[derive(Clone)]
pub struct GithubOAuthClient {
    config: Config,
    http: reqwest::Client,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GithubUserResponse {
    id: i64,
    login: String,
    name: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct GithubEmailResponse {
    email: String,
    primary: bool,
    verified: bool,
}

#[derive(Serialize)]
struct TokenRequest<'a> {
    client_id: &'a str,
    client_secret: &'a str,
    code: &'a str,
    redirect_uri: String,
}

impl GithubOAuthClient {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
        }
    }

    pub fn authorize_url(&self, state: &str) -> String {
        format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}&state={}",
            encode(&self.config.github_oauth_client_id),
            encode(&self.redirect_uri()),
            encode("read:user user:email"),
            encode(state)
        )
    }

    pub async fn exchange_code(&self, code: &str) -> AppResult<String> {
        let response = self
            .http
            .post("https://github.com/login/oauth/access_token")
            .header(header::ACCEPT, "application/json")
            .json(&TokenRequest {
                client_id: &self.config.github_oauth_client_id,
                client_secret: &self.config.github_oauth_client_secret,
                code,
                redirect_uri: self.redirect_uri(),
            })
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AppError::GitHub(format!(
                "GitHub OAuth 换取 token 失败：{}",
                response.status()
            )));
        }

        let payload = response.json::<TokenResponse>().await?;
        Ok(payload.access_token)
    }

    pub async fn fetch_user(&self, access_token: &str) -> AppResult<GithubUserInput> {
        let response = self
            .github_get("https://api.github.com/user", access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AppError::GitHub(format!(
                "GitHub 用户信息读取失败：{}",
                response.status()
            )));
        }

        let user = response.json::<GithubUserResponse>().await?;
        let email = match user.email {
            Some(email) if !email.is_empty() => Some(email),
            _ => self.fetch_primary_email(access_token).await?,
        };

        Ok(GithubUserInput {
            github_id: user.id,
            login: user.login,
            name: user.name,
            email,
            avatar_url: user.avatar_url,
        })
    }

    async fn fetch_primary_email(&self, access_token: &str) -> AppResult<Option<String>> {
        let response = self
            .github_get("https://api.github.com/user/emails", access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let emails = response.json::<Vec<GithubEmailResponse>>().await?;
        Ok(emails
            .into_iter()
            .find(|email| email.primary && email.verified)
            .map(|email| email.email))
    }

    fn github_get(&self, url: &str, access_token: &str) -> reqwest::RequestBuilder {
        self.http
            .get(url)
            .bearer_auth(access_token)
            .header(header::USER_AGENT, "icon-set-api")
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
    }

    fn redirect_uri(&self) -> String {
        format!(
            "{}/api/auth/github/callback",
            self.config.app_base_url.trim_end_matches('/')
        )
    }
}
