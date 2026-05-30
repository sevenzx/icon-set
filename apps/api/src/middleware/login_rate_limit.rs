use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use tokio::sync::RwLock;

use crate::{AppState, error::AppError};

const LOGIN_LIMIT_WINDOW: Duration = Duration::from_secs(5 * 60);
const LOGIN_MAX_FAILURES: u32 = 5;

pub(crate) struct LoginRateLimitRecord {
    window_started_at: Instant,
    failures: u32,
}

pub type LoginRateLimitStore = Arc<RwLock<HashMap<String, LoginRateLimitRecord>>>;

/// 创建登录限流记录存储。
pub fn new_login_rate_limit_store() -> LoginRateLimitStore {
    Arc::new(RwLock::new(HashMap::new()))
}

/// 清理已经过期的登录限流记录。
pub async fn cleanup_expired_login_rate_limits(state: &AppState) -> usize {
    let now = Instant::now();
    let mut records = state.login_rate_limits.write().await;
    let before = records.len();

    records.retain(|_, record| {
        now.saturating_duration_since(record.window_started_at) < LOGIN_LIMIT_WINDOW
    });
    before.saturating_sub(records.len())
}

/// 登录接口限流：同一来源在窗口期内失败过多时拒绝继续尝试。
pub async fn limit_login(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let client_key = client_key(request.headers());

    if is_limited(&state, &client_key).await {
        return Err(AppError::RateLimited);
    }

    let response = next.run(request).await;
    match response.status() {
        StatusCode::UNAUTHORIZED => record_failed_login(&state, client_key).await,
        status if status.is_success() => clear_login_failures(&state, &client_key).await,
        _ => {}
    }

    Ok(response)
}

async fn is_limited(state: &AppState, client_key: &str) -> bool {
    let now = Instant::now();
    let mut records = state.login_rate_limits.write().await;
    records.retain(|_, record| {
        now.saturating_duration_since(record.window_started_at) < LOGIN_LIMIT_WINDOW
    });

    records
        .get(client_key)
        .is_some_and(|record| record.failures >= LOGIN_MAX_FAILURES)
}

async fn record_failed_login(state: &AppState, client_key: String) {
    let now = Instant::now();
    let mut records = state.login_rate_limits.write().await;
    let record = records.entry(client_key).or_insert(LoginRateLimitRecord {
        window_started_at: now,
        failures: 0,
    });

    if now.saturating_duration_since(record.window_started_at) >= LOGIN_LIMIT_WINDOW {
        record.window_started_at = now;
        record.failures = 0;
    }

    record.failures = record.failures.saturating_add(1);
}

async fn clear_login_failures(state: &AppState, client_key: &str) {
    state.login_rate_limits.write().await.remove(client_key);
}

fn client_key(headers: &HeaderMap) -> String {
    for header_name in ["cf-connecting-ip", "x-real-ip", "x-forwarded-for"] {
        if let Some(value) = headers
            .get(header_name)
            .and_then(|value| value.to_str().ok())
        {
            if let Some(ip) = value
                .split(',')
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                return ip.to_string();
            }
        }
    }

    "unknown".to_string()
}
