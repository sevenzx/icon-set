use axum::http::{HeaderMap, HeaderValue, header};
use chrono::Duration;
use uuid::Uuid;

use crate::{
    AppState,
    db::AuthSession,
    error::{AppError, AppResult},
};

pub const SESSION_TTL: Duration = Duration::days(7);
const ADMIN_TOKEN_HEADER: &str = "x-admin-token";

pub struct CreatedSession {
    pub cookie_token: String,
}

/// 创建新的登录会话并返回 Cookie token 和写 token。
pub async fn create_session(state: &AppState, user_id: i64) -> AppResult<CreatedSession> {
    let cookie_token = Uuid::new_v4().to_string();
    let admin_token = Uuid::new_v4().to_string();
    state
        .db
        .create_session(&cookie_token, user_id, &admin_token, SESSION_TTL)
        .await?;

    Ok(CreatedSession { cookie_token })
}

/// 删除当前请求中的会话。
pub async fn destroy_session(state: &AppState, headers: &HeaderMap) -> AppResult<()> {
    if let Some(token) = read_session_cookie(headers, &state.config.session_cookie_name) {
        state.db.delete_session(&token).await?;
    }
    Ok(())
}

/// 清理已经过期的登录会话和 OAuth state。
pub async fn cleanup_expired_sessions(state: &AppState) -> AppResult<u64> {
    state.db.cleanup_expired().await
}

/// 读取当前请求对应的登录会话。
pub async fn current_session(
    state: &AppState,
    headers: &HeaderMap,
) -> AppResult<Option<AuthSession>> {
    let Some(token) = read_session_cookie(headers, &state.config.session_cookie_name) else {
        return Ok(None);
    };
    state.db.get_session(&token).await
}

/// 要求当前请求同时具备登录会话和当前会话的写 token。
pub async fn require_admin_access(state: &AppState, headers: &HeaderMap) -> AppResult<()> {
    let Some(admin_token) = headers
        .get(ADMIN_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
    else {
        return Err(AppError::Unauthorized);
    };

    let Some(session) = current_session(state, headers).await? else {
        return Err(AppError::Unauthorized);
    };

    if constant_time_eq(admin_token, &session.admin_token) {
        return Ok(());
    }

    Err(AppError::Unauthorized)
}

fn constant_time_eq(input: &str, expected: &str) -> bool {
    let input = input.as_bytes();
    let expected = expected.as_bytes();
    let mut diff = input.len() ^ expected.len();
    let max_len = input.len().max(expected.len());

    for index in 0..max_len {
        let left = input.get(index).copied().unwrap_or(0);
        let right = expected.get(index).copied().unwrap_or(0);
        diff |= usize::from(left ^ right);
    }

    diff == 0
}

/// 生成登录成功后的 Set-Cookie 值。
pub fn session_cookie_value(state: &AppState, token: &str) -> String {
    let mut value = format!(
        "{}={}; Path=/; Max-Age={}; HttpOnly; SameSite=Lax",
        state.config.session_cookie_name,
        token,
        SESSION_TTL.num_seconds()
    );
    if state.config.cookie_secure {
        value.push_str("; Secure");
    }
    value
}

/// 生成退出登录时清理 Cookie 的 Set-Cookie 值。
pub fn expired_session_cookie_value(state: &AppState) -> String {
    let mut value = format!(
        "{}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax",
        state.config.session_cookie_name
    );
    if state.config.cookie_secure {
        value.push_str("; Secure");
    }
    value
}

/// 把 Set-Cookie 字符串安全写入响应头。
pub fn set_cookie_header(headers: &mut HeaderMap, cookie: String) -> AppResult<()> {
    let value = HeaderValue::from_str(&cookie)
        .map_err(|err| AppError::Internal(format!("Cookie 生成失败：{err}")))?;
    headers.append(header::SET_COOKIE, value);
    Ok(())
}

/// 从 Cookie 请求头里读取指定会话 ID。
fn read_session_cookie(headers: &HeaderMap, cookie_name: &str) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie_header.split(';').find_map(|chunk| {
        let mut parts = chunk.trim().splitn(2, '=');
        let name = parts.next()?;
        let value = parts.next()?;
        (name == cookie_name).then(|| value.to_string())
    })
}
