use chrono::{Duration, Utc};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};

use crate::{
    crypto::SecretBox,
    error::{AppError, AppResult},
    models::{CollabLinkResponse, RepoConfigResponse, ShareAccessInspectResponse, UserProfile},
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

#[derive(Clone)]
pub struct ShareAccessSession {
    pub owner_user_id: i64,
    pub set_id: String,
    pub expires_at: chrono::DateTime<Utc>,
}

#[derive(Clone)]
pub struct ShareAccessGrant {
    pub id: i64,
    pub owner_user_id: i64,
    pub set_id: String,
    pub set_name: String,
    pub password_hash: Option<String>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub revoked_at: Option<chrono::DateTime<Utc>>,
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
            r#"
            create table if not exists share_access (
                id bigserial primary key,
                owner_user_id bigint not null references app_user(id) on delete cascade,
                set_id text not null,
                set_name text not null,
                token_hash text not null unique,
                token_ciphertext text not null,
                password_hash text,
                password_ciphertext text,
                expires_at timestamptz,
                revoked_at timestamptz,
                created_at timestamptz not null default now(),
                updated_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists share_access_session (
                id bigserial primary key,
                token text not null unique,
                share_access_id bigint not null references share_access(id) on delete cascade,
                owner_user_id bigint not null references app_user(id) on delete cascade,
                set_id text not null,
                expires_at timestamptz not null,
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
            "create index if not exists audit_log_created_at_idx on audit_log (created_at)",
            "create index if not exists share_access_owner_set_idx on share_access (owner_user_id, set_id)",
            "create index if not exists share_access_session_token_idx on share_access_session (token)",
            "alter table share_access add column if not exists token_ciphertext text",
            "alter table share_access add column if not exists password_ciphertext text",
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
        let audit_log_result =
            sqlx::query("delete from audit_log where created_at < now() - interval '7 days'")
                .execute(&self.pool)
                .await
                .map_err(|err| AppError::Internal(format!("过期审计日志清理失败：{err}")))?;
        let share_session_result =
            sqlx::query("delete from share_access_session where expires_at <= now()")
                .execute(&self.pool)
                .await
                .map_err(|err| AppError::Internal(format!("过期协作会话清理失败：{err}")))?;

        Ok(session_result.rows_affected()
            + state_result.rows_affected()
            + audit_log_result.rows_affected()
            + share_session_result.rows_affected())
    }

    /// 创建新的协作分享链接。
    pub async fn create_share_access(
        &self,
        owner_user_id: i64,
        set_id: &str,
        set_name: &str,
        token: &str,
        token_hash: &str,
        password: Option<&str>,
        password_hash: Option<&str>,
        expires_at: Option<chrono::DateTime<Utc>>,
        secrets: &SecretBox,
    ) -> AppResult<CollabLinkResponse> {
        let token_ciphertext = secrets.encrypt(token)?;
        let password_ciphertext = password
            .map(|password| secrets.encrypt(password))
            .transpose()?;
        let row = sqlx::query(
            r#"
            insert into share_access (
                owner_user_id,
                set_id,
                set_name,
                token_hash,
                token_ciphertext,
                password_hash,
                password_ciphertext,
                expires_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8)
            returning id, set_id, token_hash, token_ciphertext, password_hash, password_ciphertext, expires_at, revoked_at, created_at
            "#,
        )
        .bind(owner_user_id)
        .bind(set_id)
        .bind(set_name)
        .bind(token_hash)
        .bind(token_ciphertext)
        .bind(password_hash)
        .bind(&password_ciphertext)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("协作链接创建失败：{err}")))?;

        let id: i64 = row
            .try_get("id")
            .map_err(|err| AppError::Internal(format!("协作链接 ID 读取失败：{err}")))?;
        let created_at = row
            .try_get::<chrono::DateTime<Utc>, _>("created_at")
            .map_err(|err| AppError::Internal(format!("协作链接创建时间读取失败：{err}")))?;
        let expires_at = row
            .try_get::<Option<chrono::DateTime<Utc>>, _>("expires_at")
            .map_err(|err| AppError::Internal(format!("协作链接过期时间读取失败：{err}")))?;
        let revoked_at = row
            .try_get::<Option<chrono::DateTime<Utc>>, _>("revoked_at")
            .map_err(|err| AppError::Internal(format!("协作链接状态读取失败：{err}")))?;

        Ok(CollabLinkResponse {
            id: id.to_string(),
            set_id: set_id.to_string(),
            share_url: String::new(),
            password_enabled: password_hash.is_some(),
            password: password_ciphertext
                .as_deref()
                .map(|value| secrets.decrypt(value))
                .transpose()?,
            expires_at,
            revoked_at,
            created_at,
            active: revoked_at.is_none()
                && expires_at.map(|value| value > Utc::now()).unwrap_or(true),
        })
    }

    /// 列出当前用户指定集合的协作分享链接。
    pub async fn list_share_accesses(
        &self,
        owner_user_id: i64,
        set_id: &str,
        secrets: &SecretBox,
    ) -> AppResult<Vec<CollabLinkResponse>> {
        let rows = sqlx::query(
            r#"
            select id, set_id, token_ciphertext, password_hash, password_ciphertext, expires_at, revoked_at, created_at
            from share_access
            where owner_user_id = $1 and set_id = $2
            order by created_at desc
            "#,
        )
        .bind(owner_user_id)
        .bind(set_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("协作链接列表查询失败：{err}")))?;

        rows.into_iter()
            .map(|row| {
                let id = row
                    .try_get::<i64, _>("id")
                    .map_err(|err| AppError::Internal(format!("协作链接 ID 读取失败：{err}")))?;
                let current_set_id = row
                    .try_get::<String, _>("set_id")
                    .map_err(|err| AppError::Internal(format!("协作链接集合读取失败：{err}")))?;
                let token_ciphertext = row
                    .try_get::<String, _>("token_ciphertext")
                    .map_err(|err| AppError::Internal(format!("协作链接 token 读取失败：{err}")))?;
                let password_hash_result = row.try_get::<Option<String>, _>("password_hash");
                let password_hash = password_hash_result.map_err(|err| {
                    AppError::Internal(format!("协作链接密码状态读取失败：{err}"))
                })?;
                let password_ciphertext_result =
                    row.try_get::<Option<String>, _>("password_ciphertext");
                let password_ciphertext = password_ciphertext_result
                    .map_err(|err| AppError::Internal(format!("协作链接密码读取失败：{err}")))?;
                let expires_at_result =
                    row.try_get::<Option<chrono::DateTime<Utc>>, _>("expires_at");
                let expires_at = expires_at_result.map_err(|err| {
                    AppError::Internal(format!("协作链接过期时间读取失败：{err}"))
                })?;
                let revoked_at = row
                    .try_get::<Option<chrono::DateTime<Utc>>, _>("revoked_at")
                    .map_err(|err| AppError::Internal(format!("协作链接状态读取失败：{err}")))?;
                let created_at = row
                    .try_get::<chrono::DateTime<Utc>, _>("created_at")
                    .map_err(|err| {
                        AppError::Internal(format!("协作链接创建时间读取失败：{err}"))
                    })?;

                Ok(CollabLinkResponse {
                    id: id.to_string(),
                    set_id: current_set_id,
                    share_url: secrets.decrypt(&token_ciphertext)?,
                    password_enabled: password_hash.is_some(),
                    password: decrypt_optional(secrets, password_ciphertext)?,
                    expires_at,
                    revoked_at,
                    created_at,
                    active: revoked_at.is_none()
                        && expires_at.map(|value| value > Utc::now()).unwrap_or(true),
                })
            })
            .collect()
    }

    /// 更新当前用户的单条协作分享链接配置，并清理旧协作者会话。
    pub async fn update_share_access(
        &self,
        owner_user_id: i64,
        share_access_id: i64,
        expires_at_update: Option<Option<chrono::DateTime<Utc>>>,
        password_hash_update: Option<Option<String>>,
        password_update: Option<Option<String>>,
        secrets: &SecretBox,
    ) -> AppResult<Option<CollabLinkResponse>> {
        let update_expires_at = expires_at_update.is_some();
        let next_expires_at = expires_at_update.flatten();
        let update_password = password_hash_update.is_some();
        let next_password_hash = password_hash_update.flatten();
        let next_password_ciphertext = password_update
            .flatten()
            .as_deref()
            .map(|password| secrets.encrypt(password))
            .transpose()?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| AppError::Internal(format!("协作链接更新事务创建失败：{err}")))?;

        let row = sqlx::query(
            r#"
            update share_access
            set
                expires_at = case when $3 then $4 else expires_at end,
                password_hash = case when $5 then $6 else password_hash end,
                password_ciphertext = case when $5 then $7 else password_ciphertext end,
                updated_at = now()
            where id = $1 and owner_user_id = $2 and revoked_at is null
            returning id, set_id, token_ciphertext, password_hash, password_ciphertext, expires_at, revoked_at, created_at
            "#,
        )
        .bind(share_access_id)
        .bind(owner_user_id)
        .bind(update_expires_at)
        .bind(next_expires_at)
        .bind(update_password)
        .bind(next_password_hash.as_deref())
        .bind(next_password_ciphertext.as_deref())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| AppError::Internal(format!("协作链接更新失败：{err}")))?;

        let Some(row) = row else {
            tx.rollback()
                .await
                .map_err(|err| AppError::Internal(format!("协作链接更新事务回滚失败：{err}")))?;
            return Ok(None);
        };

        // 链接策略变化后清理旧会话，确保新的到期时间或 password 立即生效。
        if update_expires_at || update_password {
            sqlx::query("delete from share_access_session where share_access_id = $1")
                .bind(share_access_id)
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(format!("协作者会话清理失败：{err}")))?;
        }

        tx.commit()
            .await
            .map_err(|err| AppError::Internal(format!("协作链接更新事务提交失败：{err}")))?;

        let id = row
            .try_get::<i64, _>("id")
            .map_err(|err| AppError::Internal(format!("协作链接 ID 读取失败：{err}")))?;
        let current_set_id = row
            .try_get::<String, _>("set_id")
            .map_err(|err| AppError::Internal(format!("协作链接集合读取失败：{err}")))?;
        let token_ciphertext = row
            .try_get::<String, _>("token_ciphertext")
            .map_err(|err| AppError::Internal(format!("协作链接 token 读取失败：{err}")))?;
        let password_hash = row
            .try_get::<Option<String>, _>("password_hash")
            .map_err(|err| AppError::Internal(format!("协作链接密码状态读取失败：{err}")))?;
        let password_ciphertext = row
            .try_get::<Option<String>, _>("password_ciphertext")
            .map_err(|err| AppError::Internal(format!("协作链接密码读取失败：{err}")))?;
        let expires_at = row
            .try_get::<Option<chrono::DateTime<Utc>>, _>("expires_at")
            .map_err(|err| AppError::Internal(format!("协作链接过期时间读取失败：{err}")))?;
        let revoked_at = row
            .try_get::<Option<chrono::DateTime<Utc>>, _>("revoked_at")
            .map_err(|err| AppError::Internal(format!("协作链接状态读取失败：{err}")))?;
        let created_at = row
            .try_get::<chrono::DateTime<Utc>, _>("created_at")
            .map_err(|err| AppError::Internal(format!("协作链接创建时间读取失败：{err}")))?;

        Ok(Some(CollabLinkResponse {
            id: id.to_string(),
            set_id: current_set_id,
            share_url: secrets.decrypt(&token_ciphertext)?,
            password_enabled: password_hash.is_some(),
            password: decrypt_optional(secrets, password_ciphertext)?,
            expires_at,
            revoked_at,
            created_at,
            active: revoked_at.is_none()
                && expires_at.map(|value| value > Utc::now()).unwrap_or(true),
        }))
    }

    /// 失效当前用户的单条协作分享链接，并清理其所有协作者会话。
    pub async fn revoke_share_access(
        &self,
        owner_user_id: i64,
        share_access_id: i64,
    ) -> AppResult<bool> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| AppError::Internal(format!("协作链接事务创建失败：{err}")))?;
        let row = sqlx::query(
            r#"
            update share_access
            set revoked_at = now(), updated_at = now()
            where id = $1 and owner_user_id = $2 and revoked_at is null
            returning id
            "#,
        )
        .bind(share_access_id)
        .bind(owner_user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| AppError::Internal(format!("协作链接失效失败：{err}")))?;

        if row.is_none() {
            tx.rollback()
                .await
                .map_err(|err| AppError::Internal(format!("协作链接事务回滚失败：{err}")))?;
            return Ok(false);
        }

        sqlx::query("delete from share_access_session where share_access_id = $1")
            .bind(share_access_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(format!("协作者会话清理失败：{err}")))?;

        tx.commit()
            .await
            .map_err(|err| AppError::Internal(format!("协作链接事务提交失败：{err}")))?;
        Ok(true)
    }

    /// 删除当前用户的单条协作分享链接，并清理其所有协作者会话。
    pub async fn delete_share_access(
        &self,
        owner_user_id: i64,
        share_access_id: i64,
    ) -> AppResult<bool> {
        let result = sqlx::query("delete from share_access where id = $1 and owner_user_id = $2")
            .bind(share_access_id)
            .bind(owner_user_id)
            .execute(&self.pool)
            .await
            .map_err(|err| AppError::Internal(format!("删除协作链接失败：{err}")))?;
        Ok(result.rows_affected() > 0)
    }

    /// 失效当前用户某个集合的全部协作链接，并清理已有协作者会话。
    pub async fn revoke_all_share_accesses(
        &self,
        owner_user_id: i64,
        set_id: &str,
    ) -> AppResult<u64> {
        let mut tx =
            self.pool.begin().await.map_err(|err| {
                AppError::Internal(format!("批量失效协作链接事务创建失败：{err}"))
            })?;
        let rows = sqlx::query(
            r#"
            update share_access
            set revoked_at = now(), updated_at = now()
            where owner_user_id = $1 and set_id = $2 and revoked_at is null
            returning id
            "#,
        )
        .bind(owner_user_id)
        .bind(set_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|err| AppError::Internal(format!("批量失效协作链接失败：{err}")))?;

        for row in &rows {
            let share_access_id = row
                .try_get::<i64, _>("id")
                .map_err(|err| AppError::Internal(format!("协作链接 ID 读取失败：{err}")))?;
            sqlx::query("delete from share_access_session where share_access_id = $1")
                .bind(share_access_id)
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(format!("协作者会话批量清理失败：{err}")))?;
        }

        tx.commit()
            .await
            .map_err(|err| AppError::Internal(format!("批量失效协作链接事务提交失败：{err}")))?;
        Ok(rows.len() as u64)
    }

    /// 按 token hash 查询协作分享链接。
    pub async fn find_share_access_by_token_hash(
        &self,
        token_hash: &str,
    ) -> AppResult<Option<ShareAccessGrant>> {
        let row = sqlx::query(
            r#"
            select id, owner_user_id, set_id, set_name, token_hash, token_ciphertext, password_hash, expires_at, revoked_at, created_at
            from share_access
            where token_hash = $1
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("协作链接查询失败：{err}")))?;

        row.map(map_share_access_grant).transpose()
    }

    /// 按 token hash 查询协作分享链接的进入信息。
    pub async fn inspect_share_access_by_token_hash(
        &self,
        token_hash: &str,
    ) -> AppResult<Option<ShareAccessInspectResponse>> {
        let Some(grant) = self.find_share_access_by_token_hash(token_hash).await? else {
            return Ok(None);
        };
        let active = share_access_is_active(&grant);

        Ok(Some(ShareAccessInspectResponse {
            set_id: grant.set_id,
            set_name: grant.set_name,
            password_enabled: grant.password_hash.is_some(),
            expires_at: grant.expires_at,
            active,
        }))
    }

    /// 创建新的协作者编辑会话。
    pub async fn create_share_access_session(
        &self,
        token: &str,
        share_access_id: i64,
        owner_user_id: i64,
        set_id: &str,
        ttl: Duration,
    ) -> AppResult<()> {
        let expires_at = Utc::now() + ttl;
        sqlx::query(
            r#"
            insert into share_access_session (token, share_access_id, owner_user_id, set_id, expires_at)
            values ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(token)
        .bind(share_access_id)
        .bind(owner_user_id)
        .bind(set_id)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("协作者会话创建失败：{err}")))?;
        Ok(())
    }

    /// 删除指定协作者会话。
    pub async fn delete_share_access_session(&self, token: &str) -> AppResult<()> {
        sqlx::query("delete from share_access_session where token = $1")
            .bind(token)
            .execute(&self.pool)
            .await
            .map_err(|err| AppError::Internal(format!("协作者会话删除失败：{err}")))?;
        Ok(())
    }

    /// 查询当前协作者会话。
    pub async fn get_share_access_session(
        &self,
        token: &str,
    ) -> AppResult<Option<ShareAccessSession>> {
        let row = sqlx::query(
            r#"
            select
                s.id,
                s.share_access_id,
                a.owner_user_id,
                a.set_id,
                least(s.expires_at, coalesce(a.expires_at, s.expires_at)) as expires_at
            from share_access_session s
            join share_access a on a.id = s.share_access_id
            where s.token = $1
                and s.expires_at > now()
                and a.revoked_at is null
                and (a.expires_at is null or a.expires_at > now())
            "#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("协作者会话查询失败：{err}")))?;

        row.map(|row| {
            Ok(ShareAccessSession {
                owner_user_id: row
                    .try_get("owner_user_id")
                    .map_err(|err| AppError::Internal(format!("协作者会话用户读取失败：{err}")))?,
                set_id: row
                    .try_get("set_id")
                    .map_err(|err| AppError::Internal(format!("协作者会话集合读取失败：{err}")))?,
                expires_at: row.try_get("expires_at").map_err(|err| {
                    AppError::Internal(format!("协作者会话过期时间读取失败：{err}"))
                })?,
            })
        })
        .transpose()
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

    pub async fn repo_config_optional(
        &self,
        user_id: i64,
        secrets: &SecretBox,
    ) -> AppResult<Option<RepoConfig>> {
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
        })
        .transpose()
    }

    pub async fn repo_config(&self, user_id: i64, secrets: &SecretBox) -> AppResult<RepoConfig> {
        self.repo_config_optional(user_id, secrets)
            .await?
            .ok_or_else(|| AppError::BadRequest("请先配置 GitHub 仓库".to_string()))
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

fn map_share_access_grant(row: sqlx::postgres::PgRow) -> AppResult<ShareAccessGrant> {
    Ok(ShareAccessGrant {
        id: row
            .try_get("id")
            .map_err(|err| AppError::Internal(format!("协作链接 ID 读取失败：{err}")))?,
        owner_user_id: row
            .try_get("owner_user_id")
            .map_err(|err| AppError::Internal(format!("协作链接用户读取失败：{err}")))?,
        set_id: row
            .try_get("set_id")
            .map_err(|err| AppError::Internal(format!("协作链接集合读取失败：{err}")))?,
        set_name: row
            .try_get("set_name")
            .map_err(|err| AppError::Internal(format!("协作链接集合名称读取失败：{err}")))?,
        password_hash: row
            .try_get("password_hash")
            .map_err(|err| AppError::Internal(format!("协作链接密码读取失败：{err}")))?,
        expires_at: row
            .try_get("expires_at")
            .map_err(|err| AppError::Internal(format!("协作链接过期时间读取失败：{err}")))?,
        revoked_at: row
            .try_get("revoked_at")
            .map_err(|err| AppError::Internal(format!("协作链接撤销状态读取失败：{err}")))?,
    })
}

fn share_access_is_active(grant: &ShareAccessGrant) -> bool {
    grant.revoked_at.is_none()
        && grant
            .expires_at
            .map(|value| value > Utc::now())
            .unwrap_or(true)
}
