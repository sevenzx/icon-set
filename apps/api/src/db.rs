use chrono::{Duration, Utc};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};

use crate::{
    crypto::SecretBox,
    error::{AppError, AppResult},
    models::{RepoConfigResponse, UserProfile},
};

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

#[derive(Clone)]
pub struct AuthSession {
    pub user_id: i64,
    pub admin_token: String,
}

#[derive(Clone)]
pub struct RepoConfig {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub token: String,
}

pub struct GithubUserInput {
    pub github_id: i64,
    pub login: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

impl Database {
    pub async fn connect(database_url: &str) -> AppResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(database_url)
            .await
            .map_err(|err| AppError::Internal(format!("数据库连接失败：{err}")))?;
        let db = Self { pool };
        db.init().await?;
        Ok(db)
    }

    async fn init(&self) -> AppResult<()> {
        for statement in [
            r#"
            create table if not exists app_user (
                id bigserial primary key,
                display_name text,
                avatar_url text,
                email text,
                created_at timestamptz not null default now(),
                updated_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists oauth_account (
                id bigserial primary key,
                user_id bigint not null references app_user(id) on delete cascade,
                provider text not null,
                provider_user_id text not null,
                login text not null,
                created_at timestamptz not null default now(),
                updated_at timestamptz not null default now(),
                unique (provider, provider_user_id)
            )
            "#,
            r#"
            create table if not exists session (
                token text primary key,
                user_id bigint not null references app_user(id) on delete cascade,
                admin_token text not null,
                expires_at timestamptz not null,
                created_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists oauth_state (
                state text primary key,
                expires_at timestamptz not null,
                created_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists repo_config (
                user_id bigint primary key references app_user(id) on delete cascade,
                github_owner text not null,
                github_repo text not null,
                github_branch text not null,
                github_token_ciphertext text not null,
                created_at timestamptz not null default now(),
                updated_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists audit_log (
                id bigserial primary key,
                user_id bigint references app_user(id) on delete set null,
                action text not null,
                target text,
                status_code integer,
                client_ip text,
                elapsed_ms bigint,
                created_at timestamptz not null default now()
            )
            "#,
        ] {
            sqlx::query(statement)
                .execute(&self.pool)
                .await
                .map_err(|err| AppError::Internal(format!("数据库初始化失败：{err}")))?;
        }

        for statement in [
            "alter table audit_log add column if not exists status_code integer",
            "alter table audit_log add column if not exists client_ip text",
            "alter table audit_log add column if not exists elapsed_ms bigint",
        ] {
            sqlx::query(statement)
                .execute(&self.pool)
                .await
                .map_err(|err| AppError::Internal(format!("数据库迁移失败：{err}")))?;
        }

        Ok(())
    }

    pub async fn create_oauth_state(&self, state: &str, ttl: Duration) -> AppResult<()> {
        let expires_at = Utc::now() + ttl;
        sqlx::query(
            r#"
            insert into oauth_state (state, expires_at)
            values ($1, $2)
            on conflict (state) do update set expires_at = excluded.expires_at
            "#,
        )
        .bind(state)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("OAuth state 保存失败：{err}")))?;
        Ok(())
    }

    pub async fn consume_oauth_state(&self, state: &str) -> AppResult<bool> {
        let row = sqlx::query(
            "delete from oauth_state where state = $1 and expires_at > now() returning state",
        )
        .bind(state)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("OAuth state 校验失败：{err}")))?;
        Ok(row.is_some())
    }

    pub async fn upsert_github_user(
        &self,
        input: GithubUserInput,
        secrets: &SecretBox,
    ) -> AppResult<i64> {
        let provider_user_id = input.github_id.to_string();
        let display_name =
            secrets.encrypt(&input.name.clone().unwrap_or_else(|| input.login.clone()))?;
        let email = input
            .email
            .as_deref()
            .map(|email| secrets.encrypt(email))
            .transpose()?;
        let login = secrets.encrypt(&input.login)?;
        let user_id = if let Some(row) = sqlx::query(
            "select user_id from oauth_account where provider = 'github' and provider_user_id = $1",
        )
        .bind(&provider_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("OAuth 账号查询失败：{err}")))?
        {
            row.try_get::<i64, _>("user_id")
                .map_err(|err| AppError::Internal(format!("OAuth 账号读取失败：{err}")))?
        } else {
            let row = sqlx::query(
                r#"
                insert into app_user (display_name, avatar_url, email)
                values ($1, $2, $3)
                returning id
                "#,
            )
            .bind(&display_name)
            .bind(&input.avatar_url)
            .bind(&email)
            .fetch_one(&self.pool)
            .await
            .map_err(|err| AppError::Internal(format!("用户创建失败：{err}")))?;
            row.try_get::<i64, _>("id")
                .map_err(|err| AppError::Internal(format!("用户 ID 读取失败：{err}")))?
        };

        sqlx::query(
            r#"
            update app_user
            set display_name = $2, avatar_url = $3, email = $4, updated_at = now()
            where id = $1
            "#,
        )
        .bind(user_id)
        .bind(&display_name)
        .bind(&input.avatar_url)
        .bind(&email)
        .execute(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("用户更新失败：{err}")))?;

        sqlx::query(
            r#"
            insert into oauth_account (user_id, provider, provider_user_id, login)
            values ($1, 'github', $2, $3)
            on conflict (provider, provider_user_id)
            do update set login = excluded.login, updated_at = now()
            "#,
        )
        .bind(user_id)
        .bind(provider_user_id)
        .bind(&login)
        .execute(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("OAuth 账号更新失败：{err}")))?;

        Ok(user_id)
    }

    pub async fn create_session(
        &self,
        token: &str,
        user_id: i64,
        admin_token: &str,
        ttl: Duration,
    ) -> AppResult<()> {
        let expires_at = Utc::now() + ttl;
        sqlx::query(
            r#"
            insert into session (token, user_id, admin_token, expires_at)
            values ($1, $2, $3, $4)
            "#,
        )
        .bind(token)
        .bind(user_id)
        .bind(admin_token)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("会话创建失败：{err}")))?;
        Ok(())
    }

    pub async fn delete_session(&self, token: &str) -> AppResult<()> {
        sqlx::query("delete from session where token = $1")
            .bind(token)
            .execute(&self.pool)
            .await
            .map_err(|err| AppError::Internal(format!("会话删除失败：{err}")))?;
        Ok(())
    }

    pub async fn get_session(&self, token: &str) -> AppResult<Option<AuthSession>> {
        let row = sqlx::query(
            r#"
            select user_id, admin_token
            from session
            where token = $1 and expires_at > now()
            "#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("会话查询失败：{err}")))?;

        row.map(|row| {
            Ok(AuthSession {
                user_id: row
                    .try_get("user_id")
                    .map_err(|err| AppError::Internal(format!("会话用户读取失败：{err}")))?,
                admin_token: row
                    .try_get("admin_token")
                    .map_err(|err| AppError::Internal(format!("会话 token 读取失败：{err}")))?,
            })
        })
        .transpose()
    }

    pub async fn cleanup_expired(&self) -> AppResult<u64> {
        let session_result = sqlx::query("delete from session where expires_at <= now()")
            .execute(&self.pool)
            .await
            .map_err(|err| AppError::Internal(format!("过期会话清理失败：{err}")))?;
        let state_result = sqlx::query("delete from oauth_state where expires_at <= now()")
            .execute(&self.pool)
            .await
            .map_err(|err| AppError::Internal(format!("过期 OAuth state 清理失败：{err}")))?;

        Ok(session_result.rows_affected() + state_result.rows_affected())
    }

    pub async fn user_profile(&self, user_id: i64, secrets: &SecretBox) -> AppResult<UserProfile> {
        let row = sqlx::query(
            r#"
            select u.id, u.display_name, u.email, u.avatar_url, oa.login
            from app_user u
            join oauth_account oa on oa.user_id = u.id and oa.provider = 'github'
            where u.id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("用户资料查询失败：{err}")))?;
        let login = row
            .try_get::<String, _>("login")
            .map_err(|err| AppError::Internal(format!("用户 login 读取失败：{err}")))?;
        let name = row
            .try_get::<Option<String>, _>("display_name")
            .map_err(|err| AppError::Internal(format!("用户名称读取失败：{err}")))?;
        let email = row
            .try_get::<Option<String>, _>("email")
            .map_err(|err| AppError::Internal(format!("用户邮箱读取失败：{err}")))?;

        Ok(UserProfile {
            id: row
                .try_get::<i64, _>("id")
                .map_err(|err| AppError::Internal(format!("用户 ID 读取失败：{err}")))?
                .to_string(),
            login: secrets.decrypt(&login)?,
            name: decrypt_optional(secrets, name)?,
            email: decrypt_optional(secrets, email)?,
            avatar_url: row
                .try_get::<Option<String>, _>("avatar_url")
                .map_err(|err| AppError::Internal(format!("用户头像读取失败：{err}")))?,
        })
    }

    pub async fn repo_config_response(
        &self,
        user_id: i64,
        secrets: &SecretBox,
    ) -> AppResult<Option<RepoConfigResponse>> {
        let row = sqlx::query(
            r#"
            select github_owner, github_repo, github_branch, github_token_ciphertext
            from repo_config
            where user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("仓库配置查询失败：{err}")))?;

        row.map(|row| {
            let owner = row
                .try_get::<String, _>("github_owner")
                .map_err(|err| AppError::Internal(format!("仓库 owner 读取失败：{err}")))?;
            let repo = row
                .try_get::<String, _>("github_repo")
                .map_err(|err| AppError::Internal(format!("仓库 repo 读取失败：{err}")))?;
            let branch = row
                .try_get::<String, _>("github_branch")
                .map_err(|err| AppError::Internal(format!("仓库 branch 读取失败：{err}")))?;

            Ok(RepoConfigResponse {
                configured: true,
                owner: secrets.decrypt(&owner)?,
                repo: secrets.decrypt(&repo)?,
                branch: secrets.decrypt(&branch)?,
                token_configured: row
                    .try_get::<String, _>("github_token_ciphertext")
                    .map(|value| !value.is_empty())
                    .unwrap_or(false),
            })
        })
        .transpose()
    }

    pub async fn repo_config(&self, user_id: i64, secrets: &SecretBox) -> AppResult<RepoConfig> {
        let row = sqlx::query(
            r#"
            select github_owner, github_repo, github_branch, github_token_ciphertext
            from repo_config
            where user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("仓库配置查询失败：{err}")))?
        .ok_or_else(|| AppError::BadRequest("请先配置 GitHub 仓库".to_string()))?;

        let encrypted_token: String = row
            .try_get("github_token_ciphertext")
            .map_err(|err| AppError::Internal(format!("仓库 token 读取失败：{err}")))?;
        let owner: String = row
            .try_get("github_owner")
            .map_err(|err| AppError::Internal(format!("仓库 owner 读取失败：{err}")))?;
        let repo: String = row
            .try_get("github_repo")
            .map_err(|err| AppError::Internal(format!("仓库 repo 读取失败：{err}")))?;
        let branch: String = row
            .try_get("github_branch")
            .map_err(|err| AppError::Internal(format!("仓库 branch 读取失败：{err}")))?;
        Ok(RepoConfig {
            owner: secrets.decrypt(&owner)?,
            repo: secrets.decrypt(&repo)?,
            branch: secrets.decrypt(&branch)?,
            token: secrets.decrypt(&encrypted_token)?,
        })
    }

    pub async fn upsert_repo_config(
        &self,
        user_id: i64,
        owner: &str,
        repo: &str,
        branch: &str,
        token: &str,
        secrets: &SecretBox,
    ) -> AppResult<()> {
        let encrypted_owner = secrets.encrypt(owner)?;
        let encrypted_repo = secrets.encrypt(repo)?;
        let encrypted_branch = secrets.encrypt(branch)?;
        let encrypted_token = secrets.encrypt(token)?;

        sqlx::query(
            r#"
            insert into repo_config (
                user_id,
                github_owner,
                github_repo,
                github_branch,
                github_token_ciphertext
            )
            values ($1, $2, $3, $4, $5)
            on conflict (user_id)
            do update set
                github_owner = excluded.github_owner,
                github_repo = excluded.github_repo,
                github_branch = excluded.github_branch,
                github_token_ciphertext = excluded.github_token_ciphertext,
                updated_at = now()
            "#,
        )
        .bind(user_id)
        .bind(encrypted_owner)
        .bind(encrypted_repo)
        .bind(encrypted_branch)
        .bind(encrypted_token)
        .execute(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("仓库配置保存失败：{err}")))?;
        Ok(())
    }

    pub async fn migrate_sensitive_plaintext(&self, secrets: &SecretBox) -> AppResult<()> {
        self.migrate_app_user(secrets).await?;
        self.migrate_oauth_account(secrets).await?;
        self.migrate_repo_config(secrets).await?;
        Ok(())
    }

    async fn migrate_app_user(&self, secrets: &SecretBox) -> AppResult<()> {
        let rows = sqlx::query("select id, display_name, email from app_user")
            .fetch_all(&self.pool)
            .await
            .map_err(|err| AppError::Internal(format!("用户敏感字段查询失败：{err}")))?;

        for row in rows {
            let id = row
                .try_get::<i64, _>("id")
                .map_err(|err| AppError::Internal(format!("用户 ID 读取失败：{err}")))?;
            let display_name = row
                .try_get::<Option<String>, _>("display_name")
                .map_err(|err| AppError::Internal(format!("用户名称读取失败：{err}")))?;
            let email = row
                .try_get::<Option<String>, _>("email")
                .map_err(|err| AppError::Internal(format!("用户邮箱读取失败：{err}")))?;
            let (display_name, display_name_changed) =
                encrypt_optional_if_plaintext(secrets, display_name)?;
            let (email, email_changed) = encrypt_optional_if_plaintext(secrets, email)?;

            if display_name_changed || email_changed {
                sqlx::query("update app_user set display_name = $2, email = $3 where id = $1")
                    .bind(id)
                    .bind(display_name)
                    .bind(email)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| AppError::Internal(format!("用户敏感字段迁移失败：{err}")))?;
            }
        }

        Ok(())
    }

    async fn migrate_oauth_account(&self, secrets: &SecretBox) -> AppResult<()> {
        let rows = sqlx::query("select id, login from oauth_account")
            .fetch_all(&self.pool)
            .await
            .map_err(|err| AppError::Internal(format!("OAuth 账号敏感字段查询失败：{err}")))?;

        for row in rows {
            let id = row
                .try_get::<i64, _>("id")
                .map_err(|err| AppError::Internal(format!("OAuth 账号 ID 读取失败：{err}")))?;
            let login = row
                .try_get::<String, _>("login")
                .map_err(|err| AppError::Internal(format!("OAuth login 读取失败：{err}")))?;
            let encrypted_login = secrets.encrypt_if_plaintext(&login)?;

            if encrypted_login != login {
                sqlx::query("update oauth_account set login = $2 where id = $1")
                    .bind(id)
                    .bind(encrypted_login)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| AppError::Internal(format!("OAuth login 迁移失败：{err}")))?;
            }
        }

        Ok(())
    }

    async fn migrate_repo_config(&self, secrets: &SecretBox) -> AppResult<()> {
        let rows = sqlx::query(
            r#"
            select user_id, github_owner, github_repo, github_branch
            from repo_config
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("仓库配置敏感字段查询失败：{err}")))?;

        for row in rows {
            let user_id = row
                .try_get::<i64, _>("user_id")
                .map_err(|err| AppError::Internal(format!("仓库配置用户读取失败：{err}")))?;
            let owner = row
                .try_get::<String, _>("github_owner")
                .map_err(|err| AppError::Internal(format!("仓库 owner 读取失败：{err}")))?;
            let repo = row
                .try_get::<String, _>("github_repo")
                .map_err(|err| AppError::Internal(format!("仓库 repo 读取失败：{err}")))?;
            let branch = row
                .try_get::<String, _>("github_branch")
                .map_err(|err| AppError::Internal(format!("仓库 branch 读取失败：{err}")))?;
            let encrypted_owner = secrets.encrypt_if_plaintext(&owner)?;
            let encrypted_repo = secrets.encrypt_if_plaintext(&repo)?;
            let encrypted_branch = secrets.encrypt_if_plaintext(&branch)?;

            if encrypted_owner != owner || encrypted_repo != repo || encrypted_branch != branch {
                sqlx::query(
                    r#"
                    update repo_config
                    set github_owner = $2, github_repo = $3, github_branch = $4
                    where user_id = $1
                    "#,
                )
                .bind(user_id)
                .bind(encrypted_owner)
                .bind(encrypted_repo)
                .bind(encrypted_branch)
                .execute(&self.pool)
                .await
                .map_err(|err| AppError::Internal(format!("仓库配置敏感字段迁移失败：{err}")))?;
            }
        }

        Ok(())
    }

    pub async fn insert_audit_log(
        &self,
        user_id: Option<i64>,
        action: &str,
        target: Option<&str>,
        status_code: u16,
        client_ip: &str,
        elapsed_ms: i64,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            insert into audit_log (
                user_id,
                action,
                target,
                status_code,
                client_ip,
                elapsed_ms
            )
            values ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(user_id)
        .bind(action)
        .bind(target)
        .bind(i32::from(status_code))
        .bind(client_ip)
        .bind(elapsed_ms)
        .execute(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("审计日志写入失败：{err}")))?;
        Ok(())
    }
}

fn decrypt_optional(secrets: &SecretBox, value: Option<String>) -> AppResult<Option<String>> {
    value
        .as_deref()
        .map(|value| secrets.decrypt(value))
        .transpose()
}

fn encrypt_optional_if_plaintext(
    secrets: &SecretBox,
    value: Option<String>,
) -> AppResult<(Option<String>, bool)> {
    let Some(value) = value else {
        return Ok((None, false));
    };

    let encrypted = secrets.encrypt_if_plaintext(&value)?;
    let changed = encrypted != value;
    Ok((Some(encrypted), changed))
}
