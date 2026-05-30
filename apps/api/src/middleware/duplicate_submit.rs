use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderMap, Method, Uri, header},
    middleware::Next,
    response::Response,
};
use tokio::sync::RwLock;

use crate::{AppState, error::AppError};

const DUPLICATE_SUBMIT_WINDOW: Duration = Duration::from_secs(3);
const ADMIN_TOKEN_HEADER: &str = "x-admin-token";

pub type DuplicateSubmissionStore = Arc<RwLock<HashMap<String, Instant>>>;

/// 创建重复提交记录存储。
pub fn new_duplicate_submission_store() -> DuplicateSubmissionStore {
    Arc::new(RwLock::new(HashMap::new()))
}

/// 防重复提交：短时间内相同会话、相同路径、相同请求体只允许通过一次。
pub async fn prevent_duplicate_submit(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    if !is_mutating_method(request.method()) {
        return Ok(next.run(request).await);
    }

    let (parts, body) = request.into_parts();
    let body_bytes = to_bytes(body, state.config.max_upload_bytes)
        .await
        .map_err(|err| AppError::BadRequest(format!("请求体读取失败：{err}")))?;
    let fingerprint = request_fingerprint(&parts.method, &parts.uri, &parts.headers, &body_bytes);

    if is_duplicate(&state, &fingerprint).await {
        return Err(AppError::DuplicateSubmit);
    }

    let request = Request::from_parts(parts, Body::from(body_bytes));
    Ok(next.run(request).await)
}

async fn is_duplicate(state: &AppState, fingerprint: &str) -> bool {
    let now = Instant::now();
    let mut submissions = state.duplicate_submissions.write().await;
    submissions.retain(|_, created_at| {
        now.saturating_duration_since(*created_at) < DUPLICATE_SUBMIT_WINDOW
    });

    if submissions.contains_key(fingerprint) {
        return true;
    }

    submissions.insert(fingerprint.to_string(), now);
    false
}

fn request_fingerprint(method: &Method, uri: &Uri, headers: &HeaderMap, body: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();

    method.as_str().hash(&mut hasher);
    uri.path().hash(&mut hasher);
    uri.query().hash(&mut hasher);
    header_value(headers, header::COOKIE.as_str()).hash(&mut hasher);
    header_value(headers, ADMIN_TOKEN_HEADER).hash(&mut hasher);
    body.hash(&mut hasher);

    format!("{:016x}", hasher.finish())
}

fn header_value(headers: &HeaderMap, name: &str) -> String {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

fn is_mutating_method(method: &Method) -> bool {
    matches!(
        method,
        &Method::POST | &Method::PATCH | &Method::DELETE | &Method::PUT
    )
}
