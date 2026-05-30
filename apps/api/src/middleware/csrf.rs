use axum::{
    extract::{Request, State},
    http::{Method, header},
    middleware::Next,
    response::Response,
};

use crate::{AppState, error::AppError};

/// CSRF 防护：后台写请求必须来自配置的同源 Origin。
pub async fn require_csrf(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    if is_mutating_method(request.method()) {
        let Some(origin) = request
            .headers()
            .get(header::ORIGIN)
            .and_then(|value| value.to_str().ok())
        else {
            return Err(AppError::Forbidden);
        };

        if normalize_origin(origin) != normalize_origin(&state.config.cors_origin) {
            return Err(AppError::Forbidden);
        }
    }

    Ok(next.run(request).await)
}

fn is_mutating_method(method: &Method) -> bool {
    matches!(
        method,
        &Method::POST | &Method::PATCH | &Method::DELETE | &Method::PUT
    )
}

fn normalize_origin(origin: &str) -> &str {
    origin.trim().trim_end_matches('/')
}
