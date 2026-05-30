use std::time::Instant;

use axum::{extract::Request, http::HeaderMap, middleware::Next, response::Response};

/// 记录后台写接口的访问结果，不记录请求体、密码或 token。
pub async fn audit_admin(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let client_ip = client_ip(request.headers());
    let started_at = Instant::now();

    let response = next.run(request).await;
    let elapsed_ms = started_at.elapsed().as_millis();

    tracing::info!(
        target: "icon_set_api::audit",
        method = %method,
        path = %uri.path(),
        status = %response.status(),
        client_ip = %client_ip,
        elapsed_ms = elapsed_ms,
        "admin request"
    );

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
