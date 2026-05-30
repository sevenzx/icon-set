mod auth;
mod cleanup;
mod config;
mod error;
mod github;
mod handlers;
mod limits;
mod middleware;
mod models;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderName, HeaderValue, Method, header},
    middleware as axum_middleware,
    routing::{get, patch, post},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{auth::SessionStore, config::Config, github::GitHubClient};

#[derive(Clone)]
pub struct AppState {
    pub(crate) config: Config,
    pub(crate) github: GitHubClient,
    pub(crate) sessions: SessionStore,
    pub(crate) login_rate_limits: middleware::LoginRateLimitStore,
    pub(crate) duplicate_submissions: middleware::DuplicateSubmissionStore,
}

/// 启动 Axum API 服务。
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "icon_set_api=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let github = GitHubClient::new(config.clone());
    let state = AppState {
        config: config.clone(),
        github,
        sessions: auth::new_session_store(),
        login_rate_limits: middleware::new_login_rate_limit_store(),
        duplicate_submissions: middleware::new_duplicate_submission_store(),
    };
    cleanup::spawn_cleanup_task(state.clone());
    let app = build_router(state)?;
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;

    tracing::info!("icon-set api listening on {}", config.bind_addr);
    axum::serve(listener, app).await?;

    Ok(())
}

/// 构建应用路由和跨域策略。
fn build_router(state: AppState) -> Result<Router, Box<dyn std::error::Error>> {
    let cors_origin = state.config.cors_origin.parse::<HeaderValue>()?;
    let cors = CorsLayer::new()
        .allow_origin(cors_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            HeaderName::from_static("x-admin-token"),
        ])
        .allow_credentials(true);

    let admin_router = Router::new()
        .route("/sets", post(handlers::create_set))
        .route(
            "/sets/{set_id}",
            patch(handlers::update_set).delete(handlers::delete_set),
        )
        .route("/sets/{set_id}/icons", post(handlers::upload_icon))
        .route(
            "/sets/{set_id}/icons/batch",
            post(handlers::upload_icons_batch),
        )
        .route(
            "/sets/{set_id}/icons/{icon_id}",
            patch(handlers::rename_icon).delete(handlers::delete_icon),
        )
        .route_layer(axum_middleware::from_fn(middleware::audit_admin))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::prevent_duplicate_submit,
        ))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::require_csrf,
        ))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::require_admin,
        ));

    let app = Router::new()
        .route("/api/health", get(handlers::health))
        .route("/api/sets", get(handlers::list_sets))
        .route("/api/sets/{set_id}", get(handlers::get_set))
        .route(
            "/api/auth/login",
            post(handlers::login).route_layer(axum_middleware::from_fn_with_state(
                state.clone(),
                middleware::limit_login,
            )),
        )
        .route("/api/auth/logout", post(handlers::logout))
        .route("/api/auth/session", get(handlers::session))
        .nest("/api/admin", admin_router)
        .layer(DefaultBodyLimit::max(
            state
                .config
                .max_upload_bytes
                .max(limits::BATCH_MULTIPART_BODY_LIMIT),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    Ok(app)
}
