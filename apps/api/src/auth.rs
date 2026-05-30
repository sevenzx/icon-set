use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};

use axum::http::{HeaderMap, HeaderValue, header};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    AppState,
    error::{AppError, AppResult},
};

const SESSION_TTL: Duration = Duration::from_secs(60 * 60 * 24 * 7);
const ADMIN_TOKEN_HEADER: &str = "x-admin-token";

#[derive(Clone)]
pub struct SessionRecord {
    pub expires_at: SystemTime,
    pub admin_token: String,
}

pub struct CreatedSession {
    pub cookie_token: String,
    pub admin_token: String,
}

pub type SessionStore = Arc<RwLock<HashMap<String, SessionRecord>>>;

/// 创建内存会话存储。
pub fn new_session_store() -> SessionStore {
    Arc::new(RwLock::new(HashMap::new()))
}

/// 创建新的管理员会话并返回 Cookie token 和写 token。
pub async fn create_session(state: &AppState) -> CreatedSession {
    let cookie_token = Uuid::new_v4().to_string();
    let admin_token = Uuid::new_v4().to_string();
    let expires_at = SystemTime::now() + SESSION_TTL;
    state.sessions.write().await.insert(
        cookie_token.clone(),
        SessionRecord {
            expires_at,
            admin_token: admin_token.clone(),
        },
    );

    CreatedSession {
        cookie_token,
        admin_token,
    }
}

/// 删除当前请求中的会话。
pub async fn destroy_session(state: &AppState, headers: &HeaderMap) {
    if let Some(token) = read_session_cookie(headers, &state.config.session_cookie_name) {
        state.sessions.write().await.remove(&token);
    }
}

/// 检查当前请求是否包含有效管理员会话。
pub async fn is_authenticated(state: &AppState, headers: &HeaderMap) -> bool {
    let Some(token) = read_session_cookie(headers, &state.config.session_cookie_name) else {
        return false;
    };
    let now = SystemTime::now();
    let mut sessions = state.sessions.write().await;

    // 读取时顺手清理过期会话，避免长期运行时无界增长。
    sessions.retain(|_, record| record.expires_at > now);
    sessions.contains_key(&token)
}

/// 要求当前请求同时具备管理员会话和当前会话的写 token。
pub async fn require_admin_access(state: &AppState, headers: &HeaderMap) -> AppResult<()> {
    let Some(cookie_token) = read_session_cookie(headers, &state.config.session_cookie_name) else {
        return Err(AppError::Unauthorized);
    };
    let Some(admin_token) = headers
        .get(ADMIN_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
    else {
        return Err(AppError::Unauthorized);
    };

    let now = SystemTime::now();
    let mut sessions = state.sessions.write().await;

    // 读取时顺手清理过期会话，避免长期运行时无界增长。
    sessions.retain(|_, record| record.expires_at > now);

    let Some(record) = sessions.get(&cookie_token) else {
        return Err(AppError::Unauthorized);
    };

    if constant_time_eq(admin_token, &record.admin_token) {
        return Ok(());
    }

    Err(AppError::Unauthorized)
}

/// 用固定时间比较降低密码长度相同时的计时侧信道。
pub fn password_matches(input: &str, expected: &str) -> bool {
    constant_time_eq(input, expected)
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
        SESSION_TTL.as_secs()
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

#[cfg(test)]
mod tests {
    use super::password_matches;

    #[test]
    fn password_matches_requires_exact_value() {
        assert!(password_matches("secret", "secret"));
        assert!(!password_matches("secret", "Secret"));
        assert!(!password_matches("secret", "secret1"));
        assert!(!password_matches("", "secret"));
    }
}
