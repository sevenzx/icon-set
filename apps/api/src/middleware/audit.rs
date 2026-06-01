use std::time::Instant;

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};

use crate::{AppState, auth};

/// 记录后台写接口的访问结果，不记录请求体、密码或 token。
pub async fn audit_admin(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let client_ip = client_ip(request.headers());
    let user_id = auth::current_session(&state, request.headers())
        .await
        .ok()
        .flatten()
        .map(|session| session.user_id);
    let started_at = Instant::now();

    let response = next.run(request).await;
    let elapsed_ms = started_at.elapsed().as_millis();
    let action = format!("{} {}", method, uri.path());
    let status_code = response.status().as_u16();

    tracing::info!(
        target: "icon_set_api::audit",
        method = %method,
        path = %uri.path(),
        status = %response.status(),
        client_ip = %client_ip,
        elapsed_ms = elapsed_ms,
        "admin request"
    );

    if let Err(err) = state
        .db
        .insert_audit_log(
            user_id,
            &action,
            Some(uri.path()),
            status_code,
            &client_ip,
            elapsed_ms.min(i64::MAX as u128) as i64,
        )
        .await
    {
        tracing::warn!(target: "icon_set_api::audit", error = %err, "audit log write failed");
    }

    response
}

fn client_ip(headers: &HeaderMap) -> String {
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
