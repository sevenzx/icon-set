use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{AppState, auth, error::AppResult};

/// Admin 路由统一鉴权中间件。
pub async fn require_admin(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> AppResult<Response> {
    let headers = request.headers().clone();
    auth::require_admin_access(&state, &headers).await?;

    Ok(next.run(request).await)
}
