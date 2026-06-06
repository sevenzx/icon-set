use std::{
    collections::HashMap,
    io::{Cursor, Read},
};

use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState, auth,
    db::RepoConfig,
    demo,
    error::{AppError, AppResult},
    github::GitHubClient,
    limits,
    models::{
        CollabLinkListQuery, CollabLinkResponse, CreateCollabLinkRequest, CreateSetRequest,
        IconEntry, IconManifest, IconSetSummary, RenameIconRequest, RepoConfigRequest,
        RepoConfigResponse, RevokeCollabLinksRequest, SessionResponse, ShareAccessAuthorizeRequest,
        ShareAccessInspectQuery, ShareAccessInspectResponse, ShareAccessSessionResponse,
        UpdateCollabLinkRequest, UpdateSetRequest,
    },
};
use zip::ZipArchive;

const SETS_INDEX_PATH: &str = "sets.json";
const ICON_NAME_MAX_LEN: usize = 120;
const OAUTH_STATE_TTL: Duration = Duration::minutes(10);

enum PublicDataSource {
    Demo,
    UserWithoutConfig,
    UserRepo(GitHubClient),
}

#[derive(Debug, Deserialize)]
pub struct GithubCallbackQuery {
    code: String,
    state: String,
}

#[derive(Debug, Deserialize)]
pub struct ShareSetQuery {
    icon_set_url: String,
}

/// 返回健康检查状态。
pub async fn health() -> Json<serde_json::Value> {
    Json(json!({ "ok": true }))
}

/// 列出公开演示集合，登录后改为读取当前用户仓库。
pub async fn list_sets(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<IconSetSummary>>> {
    match public_data_source(&state, &headers).await? {
        PublicDataSource::Demo => Ok(Json(demo::list_sets())),
        PublicDataSource::UserWithoutConfig => Ok(Json(Vec::new())),
        PublicDataSource::UserRepo(github) => {
            let (sets, _) = load_sets(&github).await?;
            Ok(Json(sets))
        }
    }
}

/// 读取公开演示集合，登录后改为读取当前用户仓库。
pub async fn get_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(set_id): Path<String>,
) -> AppResult<Json<IconManifest>> {
    validate_set_id(&set_id)?;

    match public_data_source(&state, &headers).await? {
        PublicDataSource::Demo => demo::get_set(&set_id).map(Json).ok_or(AppError::NotFound),
        PublicDataSource::UserWithoutConfig => Err(AppError::NotFound),
        PublicDataSource::UserRepo(github) => {
            let (manifest, _) = load_manifest(&github, &set_id).await?;
            Ok(Json(manifest))
        }
    }
}

/// 按公网 manifest 地址读取分享集合。
pub async fn share_set(Query(query): Query<ShareSetQuery>) -> AppResult<Json<IconManifest>> {
    let manifest_url = validate_share_manifest_url(&query.icon_set_url)?;
    let response = reqwest::Client::new()
        .get(manifest_url.clone())
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(AppError::BadRequest(format!(
            "分享链接不可访问：HTTP {}",
            response.status()
        )));
    }

    let mut manifest = response
        .json::<IconManifest>()
        .await
        .map_err(|err| AppError::BadRequest(format!("分享链接返回的 manifest 无法解析：{err}")))?;

    normalize_shared_manifest(&manifest_url, &mut manifest)?;

    Ok(Json(manifest))
}

/// 列出当前用户指定集合的协作分享链接。
pub async fn list_collab_links(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CollabLinkListQuery>,
) -> AppResult<Json<Vec<CollabLinkResponse>>> {
    validate_set_id(&query.set_id)?;
    let session = require_session(&state, &headers).await?;
    let mut links = state
        .db
        .list_share_accesses(session.user_id, &query.set_id, &state.secrets)
        .await?;

    for link in &mut links {
        link.share_url = absolute_collab_share_url(&state, &link.share_url);
    }

    Ok(Json(links))
}

/// 为当前用户指定集合创建新的协作分享链接。
pub async fn create_collab_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateCollabLinkRequest>,
) -> AppResult<Json<CollabLinkResponse>> {
    validate_set_id(&payload.set_id)?;
    let session = require_session(&state, &headers).await?;
    let github = admin_github(&state, &headers).await?;
    let (manifest, _) = load_manifest(&github, &payload.set_id).await?;
    let token = Uuid::new_v4().to_string();
    let token_hash = digest_token(&token);
    let password = payload.password.trim();
    let password_hash = if password.is_empty() {
        None
    } else {
        Some(digest_password(password))
    };
    let password_plaintext = if password.is_empty() {
        None
    } else {
        Some(password)
    };
    let expires_at = if let Some(expires_at) = payload.expires_at {
        if expires_at <= Utc::now() {
            return Err(AppError::BadRequest("到期时间必须晚于当前时间".to_string()));
        }
        Some(expires_at)
    } else {
        None
    };
    let mut link = state
        .db
        .create_share_access(
            session.user_id,
            &payload.set_id,
            &manifest.name,
            &token,
            &token_hash,
            password_plaintext,
            password_hash.as_deref(),
            expires_at,
            &state.secrets,
        )
        .await?;
    link.share_url = absolute_collab_share_url(&state, &token);

    Ok(Json(link))
}

/// 更新当前用户的单条协作分享链接配置。
pub async fn update_collab_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(link_id): Path<String>,
    Json(payload): Json<UpdateCollabLinkRequest>,
) -> AppResult<Json<CollabLinkResponse>> {
    let session = require_session(&state, &headers).await?;
    let link_id = link_id
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("协作链接 ID 无效".to_string()))?;

    if let Some(Some(expires_at)) = payload.expires_at.as_ref() {
        if expires_at.to_owned() <= Utc::now() {
            return Err(AppError::BadRequest("到期时间必须晚于当前时间".to_string()));
        }
    }

    let (password_hash_update, password_update) = if payload.clear_password {
        (Some(None), Some(None))
    } else if let Some(password) = payload.password {
        let password = password.trim();
        if password.is_empty() {
            return Err(AppError::BadRequest("password 不能为空".to_string()));
        }
        (
            Some(Some(digest_password(password))),
            Some(Some(password.to_string())),
        )
    } else {
        (None, None)
    };

    let mut link = state
        .db
        .update_share_access(
            session.user_id,
            link_id,
            payload.expires_at,
            password_hash_update,
            password_update,
            &state.secrets,
        )
        .await?
        .ok_or(AppError::NotFound)?;
    link.share_url = absolute_collab_share_url(&state, &link.share_url);

    Ok(Json(link))
}

/// 失效当前用户某个集合的全部协作分享链接。
pub async fn revoke_all_collab_links(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RevokeCollabLinksRequest>,
) -> AppResult<Json<serde_json::Value>> {
    validate_set_id(&payload.set_id)?;
    let session = require_session(&state, &headers).await?;
    let revoked = state
        .db
        .revoke_all_share_accesses(session.user_id, &payload.set_id)
        .await?;

    Ok(Json(json!({ "revoked": revoked })))
}

/// 失效当前用户的单条协作分享链接。
pub async fn revoke_collab_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(link_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let session = require_session(&state, &headers).await?;
    let link_id = link_id
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("协作链接 ID 无效".to_string()))?;
    let revoked = state
        .db
        .revoke_share_access(session.user_id, link_id)
        .await?;

    if !revoked {
        return Err(AppError::NotFound);
    }

    Ok(Json(json!({ "revoked": true })))
}

/// 删除当前用户的单条协作分享链接。
pub async fn delete_collab_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(link_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let session = require_session(&state, &headers).await?;
    let link_id = link_id
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("协作链接 ID 无效".to_string()))?;
    let deleted = state
        .db
        .delete_share_access(session.user_id, link_id)
        .await?;

    if !deleted {
        return Err(AppError::NotFound);
    }

    Ok(Json(json!({ "deleted": true })))
}

/// 检查协作分享链接是否可进入。
pub async fn inspect_share_access(
    State(state): State<AppState>,
    Query(query): Query<ShareAccessInspectQuery>,
) -> AppResult<Json<ShareAccessInspectResponse>> {
    let token_hash = digest_token(query.token.trim());
    let share = state
        .db
        .inspect_share_access_by_token_hash(&token_hash)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(share))
}

/// 使用协作分享链接进入共享编辑会话。
pub async fn authorize_share_access(
    State(state): State<AppState>,
    Json(payload): Json<ShareAccessAuthorizeRequest>,
) -> AppResult<Response> {
    let token = payload.token.trim();
    if token.is_empty() {
        return Err(AppError::BadRequest("token 不能为空".to_string()));
    }

    let token_hash = digest_token(token);
    let share = state
        .db
        .find_share_access_by_token_hash(&token_hash)
        .await?
        .ok_or(AppError::NotFound)?;

    ensure_share_access_available(&share)?;

    if let Some(expected_password_hash) = &share.password_hash {
        let password = payload.password.trim();
        if password.is_empty() || digest_password(password) != *expected_password_hash {
            return Err(AppError::Unauthorized);
        }
    }

    let session_token =
        auth::create_share_access_session(&state, share.id, share.owner_user_id, &share.set_id)
            .await?;
    let cookie = auth::share_access_cookie_value(&state, &session_token);
    let mut response = Json(ShareAccessSessionResponse {
        active: true,
        set_id: Some(share.set_id),
        set_name: Some(share.set_name),
        expires_at: Some(Utc::now() + auth::SHARE_ACCESS_SESSION_TTL),
    })
    .into_response();
    auth::set_cookie_header(response.headers_mut(), cookie)?;

    Ok(response)
}

/// 查询当前协作者共享编辑会话状态。
pub async fn current_share_access(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<ShareAccessSessionResponse>> {
    let Some(session) = auth::current_share_access_session(&state, &headers).await? else {
        return Ok(Json(ShareAccessSessionResponse {
            active: false,
            set_id: None,
            set_name: None,
            expires_at: None,
        }));
    };
    let github = share_access_github(&state, &session).await?;
    let (manifest, _) = load_manifest(&github, &session.set_id).await?;

    Ok(Json(ShareAccessSessionResponse {
        active: true,
        set_id: Some(session.set_id),
        set_name: Some(manifest.name),
        expires_at: Some(session.expires_at),
    }))
}

/// 退出当前协作者共享编辑会话。
pub async fn logout_share_access(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Response> {
    auth::destroy_share_access_session(&state, &headers).await?;
    let cookie = auth::expired_share_access_cookie_value(&state);
    let mut response = Json(ShareAccessSessionResponse {
        active: false,
        set_id: None,
        set_name: None,
        expires_at: None,
    })
    .into_response();
    auth::set_cookie_header(response.headers_mut(), cookie)?;
    Ok(response)
}

/// 读取当前协作者被授权的图标集合。
pub async fn get_share_edit_set(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<IconManifest>> {
    let session = auth::require_share_access_session(&state, &headers).await?;
    let github = share_access_github(&state, &session).await?;
    let (manifest, _) = load_manifest(&github, &session.set_id).await?;
    Ok(Json(manifest))
}

/// 协作者上传图片到被授权的图标集合。
pub async fn upload_share_edit_icon(
    State(state): State<AppState>,
    headers: HeaderMap,
    multipart: Multipart,
) -> AppResult<Json<IconManifest>> {
    let session = auth::require_share_access_session(&state, &headers).await?;
    upload_icon_to_set(state, session.owner_user_id, &session.set_id, multipart).await
}

/// 协作者批量上传图片或 zip 压缩包到被授权的图标集合。
pub async fn upload_share_edit_icons_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    multipart: Multipart,
) -> AppResult<Json<IconManifest>> {
    let session = auth::require_share_access_session(&state, &headers).await?;
    upload_icons_batch_to_set(state, session.owner_user_id, &session.set_id, multipart).await
}

/// 协作者修改被授权集合中的图标名称。
pub async fn rename_share_edit_icon(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(icon_id): Path<String>,
    Json(payload): Json<RenameIconRequest>,
) -> AppResult<Json<IconManifest>> {
    let session = auth::require_share_access_session(&state, &headers).await?;
    rename_icon_in_set(
        state,
        session.owner_user_id,
        &session.set_id,
        &icon_id,
        payload,
    )
    .await
}

/// 跳转到 GitHub OAuth 授权页。
pub async fn github_oauth_start(State(state): State<AppState>) -> AppResult<Redirect> {
    let oauth_state = Uuid::new_v4().to_string();
    state
        .db
        .create_oauth_state(&oauth_state, OAUTH_STATE_TTL)
        .await?;
    Ok(Redirect::temporary(
        &state.oauth.authorize_url(&oauth_state),
    ))
}

/// 处理 GitHub OAuth 回调并建立登录会话。
pub async fn github_oauth_callback(
    State(state): State<AppState>,
    Query(query): Query<GithubCallbackQuery>,
) -> AppResult<Response> {
    if !state.db.consume_oauth_state(&query.state).await? {
        return Err(AppError::Unauthorized);
    }

    let access_token = state.oauth.exchange_code(&query.code).await?;
    let github_user = state.oauth.fetch_user(&access_token).await?;
    let user_id = state
        .db
        .upsert_github_user(github_user, &state.secrets)
        .await?;
    let session = auth::create_session(&state, user_id).await?;
    let cookie = auth::session_cookie_value(&state, &session.cookie_token);
    let mut response = Redirect::temporary("/console").into_response();
    auth::set_cookie_header(response.headers_mut(), cookie)?;

    Ok(response)
}

/// 清理当前管理员会话。
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    auth::destroy_session(&state, &headers).await?;
    let cookie = auth::expired_session_cookie_value(&state);
    let mut response = Json(SessionResponse {
        authenticated: false,
        admin_token: None,
        user: None,
        repo_config: None,
    })
    .into_response();
    auth::set_cookie_header(response.headers_mut(), cookie)?;

    Ok(response)
}

/// 查询当前请求是否已经登录。
pub async fn session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<SessionResponse>> {
    let Some(session) = auth::current_session(&state, &headers).await? else {
        return Ok(Json(SessionResponse {
            authenticated: false,
            admin_token: None,
            user: None,
            repo_config: None,
        }));
    };
    let user = state
        .db
        .user_profile(session.user_id, &state.secrets)
        .await?;
    let repo_config = state
        .db
        .repo_config_response(session.user_id, &state.secrets)
        .await?;

    Ok(Json(SessionResponse {
        authenticated: true,
        admin_token: Some(session.admin_token),
        user: Some(user),
        repo_config,
    }))
}

/// 读取当前用户的 GitHub 仓库配置。
pub async fn get_repo_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<RepoConfigResponse>> {
    let session = require_session(&state, &headers).await?;
    let config = state
        .db
        .repo_config_response(session.user_id, &state.secrets)
        .await?
        .unwrap_or(RepoConfigResponse {
            configured: false,
            owner: String::new(),
            repo: String::new(),
            branch: "main".to_string(),
            token_configured: false,
        });

    Ok(Json(config))
}

/// 保存当前用户的 GitHub 仓库配置，token 会加密后写入数据库。
pub async fn save_repo_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RepoConfigRequest>,
) -> AppResult<Json<RepoConfigResponse>> {
    let session = require_session(&state, &headers).await?;
    let owner = validate_required_text(&payload.owner, "GitHub Owner", 120)?;
    let repo = validate_required_text(&payload.repo, "GitHub Repo", 120)?;
    let branch = validate_required_text(&payload.branch, "GitHub Branch", 120)?;
    let token = if payload.token.trim().is_empty() {
        state
            .db
            .repo_config(session.user_id, &state.secrets)
            .await
            .map(|config| config.token)
            .map_err(|_| AppError::BadRequest("首次配置必须填写 GitHub Token".to_string()))?
    } else {
        payload.token.trim().to_string()
    };
    let github = GitHubClient::from_repo_config(RepoConfig {
        owner: owner.clone(),
        repo: repo.clone(),
        branch: branch.clone(),
        token: token.clone(),
    });

    // 保存前轻量校验仓库、分支和 token 是否可访问；sets.json 不存在也允许继续。
    let _ = github.get_file(SETS_INDEX_PATH).await?;

    state
        .db
        .upsert_repo_config(
            session.user_id,
            &owner,
            &repo,
            &branch,
            &token,
            &state.secrets,
        )
        .await?;

    Ok(Json(RepoConfigResponse {
        configured: true,
        owner,
        repo,
        branch,
        token_configured: true,
    }))
}

/// 列出当前用户仓库中的图标集合。
pub async fn list_admin_sets(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<IconSetSummary>>> {
    let github = admin_github(&state, &headers).await?;
    let (sets, _) = load_sets(&github).await?;
    Ok(Json(sets))
}

/// 读取当前用户仓库中的某个图标集合。
pub async fn get_admin_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(set_id): Path<String>,
) -> AppResult<Json<IconManifest>> {
    validate_set_id(&set_id)?;
    let github = admin_github(&state, &headers).await?;
    let (manifest, _) = load_manifest(&github, &set_id).await?;
    Ok(Json(manifest))
}

/// 创建一个新的图标集合。
pub async fn create_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateSetRequest>,
) -> AppResult<Json<IconSetSummary>> {
    let github = admin_github(&state, &headers).await?;
    let set_id = slugify(&payload.id);
    validate_set_id(&set_id)?;
    let name = validate_required_text(&payload.name, "集合名称", 120)?;
    let description = validate_optional_text(&payload.description, 800)?;
    let (mut sets, sets_sha) = load_sets(&github).await?;

    if sets.iter().any(|set| set.id == set_id) {
        return Err(AppError::Conflict(set_id));
    }

    let now = now_iso();
    let summary = IconSetSummary {
        id: set_id.clone(),
        name: name.clone(),
        description: description.clone(),
        icon_count: 0,
        updated_at: now.clone(),
    };
    let manifest = IconManifest {
        id: set_id.clone(),
        name,
        description,
        icons: Vec::new(),
        updated_at: now,
    };
    let manifest_path = manifest_path(&set_id);

    if github.get_file(&manifest_path).await?.is_some() {
        return Err(AppError::Conflict(manifest_path));
    }

    save_manifest(
        &github,
        &manifest,
        None,
        &format!("Create icon set {set_id}"),
    )
    .await?;
    sets.push(summary.clone());
    sort_sets(&mut sets);
    save_sets(&github, &sets, sets_sha.as_deref(), "Update sets index").await?;

    Ok(Json(summary))
}

/// 更新图标集合的名称和描述。
pub async fn update_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(set_id): Path<String>,
    Json(payload): Json<UpdateSetRequest>,
) -> AppResult<Json<IconSetSummary>> {
    let github = admin_github(&state, &headers).await?;
    validate_set_id(&set_id)?;

    let (mut sets, sets_sha) = load_sets(&github).await?;
    let Some(summary) = sets.iter_mut().find(|set| set.id == set_id) else {
        return Err(AppError::NotFound);
    };
    let (mut manifest, manifest_sha) = load_manifest(&github, &set_id).await?;

    if let Some(name) = payload.name {
        let name = validate_required_text(&name, "集合名称", 120)?;
        summary.name = name.clone();
        manifest.name = name;
    }
    if let Some(description) = payload.description {
        let description = validate_optional_text(&description, 800)?;
        summary.description = description.clone();
        manifest.description = description;
    }

    let now = now_iso();
    summary.updated_at = now.clone();
    summary.icon_count = manifest.icons.len();
    manifest.updated_at = now;
    let updated_summary = summary.clone();

    save_manifest(
        &github,
        &manifest,
        Some(&manifest_sha),
        &format!("Update icon set {set_id}"),
    )
    .await?;
    sort_sets(&mut sets);
    save_sets(&github, &sets, sets_sha.as_deref(), "Update sets index").await?;

    Ok(Json(updated_summary))
}

/// 删除一个图标集合及其 manifest 中登记的图片。
pub async fn delete_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(set_id): Path<String>,
) -> AppResult<Json<Vec<IconSetSummary>>> {
    let github = admin_github(&state, &headers).await?;
    validate_set_id(&set_id)?;

    let (mut sets, sets_sha) = load_sets(&github).await?;
    let original_len = sets.len();
    sets.retain(|set| set.id != set_id);
    if sets.len() == original_len {
        return Err(AppError::NotFound);
    }

    if let Ok((manifest, manifest_sha)) = load_manifest(&github, &set_id).await {
        for icon in manifest.icons.iter().filter(|icon| !icon.path.is_empty()) {
            delete_github_file_if_exists(
                &github,
                &icon.path,
                &format!("Delete icon {}", icon.name),
            )
            .await?;
        }
        github
            .delete_file(
                &manifest_path(&set_id),
                &manifest_sha,
                &format!("Delete icon set {set_id}"),
            )
            .await?;
    }

    save_sets(&github, &sets, sets_sha.as_deref(), "Update sets index").await?;
    Ok(Json(sets))
}

/// 上传图片并写入对应集合的 manifest。
pub async fn upload_icon(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(set_id): Path<String>,
    multipart: Multipart,
) -> AppResult<Json<IconManifest>> {
    let session = require_session(&state, &headers).await?;
    upload_icon_to_set(state, session.user_id, &set_id, multipart).await
}

/// 批量上传图片或 zip 压缩包并写入对应集合的 manifest。
pub async fn upload_icons_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(set_id): Path<String>,
    multipart: Multipart,
) -> AppResult<Json<IconManifest>> {
    let session = require_session(&state, &headers).await?;
    upload_icons_batch_to_set(state, session.user_id, &set_id, multipart).await
}

/// 修改指定图标的名称。
pub async fn rename_icon(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((set_id, icon_id)): Path<(String, String)>,
    Json(payload): Json<RenameIconRequest>,
) -> AppResult<Json<IconManifest>> {
    let session = require_session(&state, &headers).await?;
    rename_icon_in_set(state, session.user_id, &set_id, &icon_id, payload).await
}

/// 删除指定图标及其 GitHub 图片文件。
pub async fn delete_icon(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((set_id, icon_id)): Path<(String, String)>,
) -> AppResult<Json<IconManifest>> {
    let github = admin_github(&state, &headers).await?;
    validate_set_id(&set_id)?;

    let (mut manifest, manifest_sha) = load_manifest(&github, &set_id).await?;
    let Some(position) = manifest.icons.iter().position(|icon| icon.id == icon_id) else {
        return Err(AppError::NotFound);
    };
    let icon = manifest.icons.remove(position);

    if !icon.path.is_empty() {
        delete_github_file_if_exists(&github, &icon.path, &format!("Delete icon {}", icon.name))
            .await?;
    }

    manifest.updated_at = now_iso();
    save_manifest(
        &github,
        &manifest,
        Some(&manifest_sha),
        &format!("Update icon set {set_id}"),
    )
    .await?;
    sync_set_summary(&github, &manifest).await?;

    Ok(Json(manifest))
}

/// 上传图片并写入对应集合的 manifest。
async fn upload_icon_to_set(
    state: AppState,
    owner_user_id: i64,
    set_id: &str,
    mut multipart: Multipart,
) -> AppResult<Json<IconManifest>> {
    let github = github_for_user(&state, owner_user_id).await?;
    validate_set_id(set_id)?;

    let mut icon_name: Option<String> = None;
    let mut upload: Option<UploadFile> = None;

    while let Some(field) = multipart.next_field().await? {
        let field_name = field.name().unwrap_or_default().to_string();
        match field_name.as_str() {
            "name" => icon_name = Some(field.text().await?),
            "file" => {
                let file_name = field.file_name().unwrap_or("icon.png").to_string();
                let content_type = field.content_type().map(|value| value.to_string());
                let bytes = field.bytes().await?;
                if bytes.len() > state.config.max_upload_bytes {
                    return Err(AppError::BadRequest(format!(
                        "图片不能超过 {} 字节",
                        state.config.max_upload_bytes
                    )));
                }
                upload = Some(UploadFile {
                    file_name,
                    content_type,
                    bytes: bytes.to_vec(),
                });
            }
            _ => {}
        }
    }

    let upload = upload.ok_or_else(|| AppError::BadRequest("缺少 file 字段".to_string()))?;
    if upload.bytes.is_empty() {
        return Err(AppError::BadRequest("图片内容不能为空".to_string()));
    }

    let extension = detect_extension(&upload.file_name, upload.content_type.as_deref())?;
    let name = match icon_name {
        Some(value) if !value.trim().is_empty() => validate_icon_name(&value)?,
        _ => filename_stem(&upload.file_name),
    };
    let icon_id = Uuid::new_v4().to_string();
    let md5 = file_md5(&upload.bytes);
    let (mut manifest, manifest_sha) = load_manifest(&github, set_id).await?;
    hydrate_missing_icon_md5(&github, &mut manifest).await?;
    ensure_unique_icon_md5(&manifest, &md5)?;
    let (name, path) =
        unique_icon_name_and_path(&github, &manifest, set_id, &name, &extension, None, None)
            .await?;

    github
        .put_file(&path, &upload.bytes, &format!("Add icon {name}"), None)
        .await?;

    manifest.icons.push(IconEntry {
        id: icon_id,
        name,
        path: path.clone(),
        url: github.raw_url(&path),
        md5,
    });
    normalize_manifest(&github, &set_id, &mut manifest);
    save_manifest(
        &github,
        &manifest,
        Some(&manifest_sha),
        &format!("Update icon set {set_id}"),
    )
    .await?;
    sync_set_summary(&github, &manifest).await?;

    Ok(Json(manifest))
}

/// 批量上传图片或 zip 压缩包并写入对应集合的 manifest。
async fn upload_icons_batch_to_set(
    state: AppState,
    owner_user_id: i64,
    set_id: &str,
    mut multipart: Multipart,
) -> AppResult<Json<IconManifest>> {
    let github = github_for_user(&state, owner_user_id).await?;
    validate_set_id(set_id)?;

    let mut uploads: Vec<BatchUploadFile> = Vec::new();
    let mut total_bytes: usize = 0;

    while let Some(field) = multipart.next_field().await? {
        let field_name = field.name().unwrap_or_default().to_string();
        let file_name = field.file_name().unwrap_or("upload").to_string();
        let bytes = field.bytes().await?.to_vec();

        if bytes.is_empty() {
            continue;
        }

        total_bytes = checked_total_upload_bytes(total_bytes, bytes.len())?;
        match field_name.as_str() {
            "files" => {
                ensure_allowed_image_file(&file_name)?;
                uploads.push(BatchUploadFile::new(file_name, bytes));
            }
            "archive" => {
                let archive_uploads = extract_zip_images(&file_name, bytes)?;
                uploads.extend(archive_uploads);
            }
            _ => {}
        }
    }

    if uploads.is_empty() {
        return Err(AppError::BadRequest("请选择图片或 zip 压缩包".to_string()));
    }

    let extracted_total = uploads.iter().try_fold(0usize, |total, upload| {
        checked_total_upload_bytes(total, upload.bytes.len())
    })?;
    if extracted_total > limits::BATCH_UPLOAD_MAX_BYTES {
        return Err(AppError::BadRequest(
            "批量图片总体积不能超过 10MB".to_string(),
        ));
    }

    let (mut manifest, manifest_sha) = load_manifest(&github, set_id).await?;
    hydrate_missing_icon_md5(&github, &mut manifest).await?;
    ensure_unique_batch_md5s(&manifest, &uploads)?;
    let mut new_icons = Vec::with_capacity(uploads.len());

    for upload in uploads {
        let extension = detect_extension(&upload.file_name, None)?;
        let requested_name = filename_stem(&upload.file_name);
        let (name, path) = unique_icon_name_and_path(
            &github,
            &manifest,
            set_id,
            &requested_name,
            &extension,
            None,
            None,
        )
        .await?;

        github
            .put_file(&path, &upload.bytes, &format!("Add icon {name}"), None)
            .await?;

        let icon = IconEntry {
            id: Uuid::new_v4().to_string(),
            name,
            path: path.clone(),
            url: github.raw_url(&path),
            md5: upload.md5,
        };
        manifest.icons.push(icon.clone());
        new_icons.push(icon);
    }

    manifest.updated_at = now_iso();
    normalize_manifest(&github, &set_id, &mut manifest);
    save_manifest(
        &github,
        &manifest,
        Some(&manifest_sha),
        &format!("Batch add {} icons to {set_id}", new_icons.len()),
    )
    .await?;
    sync_set_summary(&github, &manifest).await?;

    Ok(Json(manifest))
}

/// 修改指定图标的名称。
async fn rename_icon_in_set(
    state: AppState,
    owner_user_id: i64,
    set_id: &str,
    icon_id: &str,
    payload: RenameIconRequest,
) -> AppResult<Json<IconManifest>> {
    let github = github_for_user(&state, owner_user_id).await?;
    validate_set_id(set_id)?;

    let (mut manifest, manifest_sha) = load_manifest(&github, set_id).await?;
    let Some(icon_position) = manifest.icons.iter().position(|icon| icon.id == icon_id) else {
        return Err(AppError::NotFound);
    };
    let current_icon = manifest.icons[icon_position].clone();
    let requested_name = validate_icon_name(&payload.name)?;
    let extension = icon_extension(&current_icon.path)?;
    let (name, path) = unique_icon_name_and_path(
        &github,
        &manifest,
        set_id,
        &requested_name,
        &extension,
        Some(&icon_id),
        Some(&current_icon.path),
    )
    .await?;
    let url = github.raw_url(&path);
    let path_changed = current_icon.path != path;
    let mut md5 = current_icon.md5.clone();

    if path_changed {
        let Some(file) = github.get_file(&current_icon.path).await? else {
            return Err(AppError::NotFound);
        };
        if md5.is_empty() {
            md5 = file_md5(&file.content);
        }
        github
            .put_file(
                &path,
                &file.content,
                &format!("Rename icon file {name}"),
                None,
            )
            .await?;
    }

    let icon = &mut manifest.icons[icon_position];
    icon.name = name;
    icon.path = path;
    icon.url = url;
    icon.md5 = md5;
    manifest.updated_at = now_iso();
    sort_icons(&mut manifest.icons);

    save_manifest(
        &github,
        &manifest,
        Some(&manifest_sha),
        &format!("Rename icon {icon_id}"),
    )
    .await?;
    sync_set_summary(&github, &manifest).await?;
    if path_changed {
        delete_github_file_if_exists(
            &github,
            &current_icon.path,
            &format!("Delete old icon file {}", current_icon.name),
        )
        .await?;
    }

    Ok(Json(manifest))
}

struct UploadFile {
    file_name: String,
    content_type: Option<String>,
    bytes: Vec<u8>,
}

struct BatchUploadFile {
    file_name: String,
    bytes: Vec<u8>,
    md5: String,
}

impl BatchUploadFile {
    fn new(file_name: String, bytes: Vec<u8>) -> Self {
        let md5 = file_md5(&bytes);
        Self {
            file_name,
            bytes,
            md5,
        }
    }
}

async fn require_session(
    state: &AppState,
    headers: &HeaderMap,
) -> AppResult<crate::db::AuthSession> {
    auth::current_session(state, headers)
        .await?
        .ok_or(AppError::Unauthorized)
}

/// 根据 owner 用户 ID 读取其 GitHub 配置并构造客户端。
async fn github_for_user(state: &AppState, owner_user_id: i64) -> AppResult<GitHubClient> {
    let repo_config = state.db.repo_config(owner_user_id, &state.secrets).await?;
    Ok(GitHubClient::from_repo_config(repo_config))
}

async fn admin_github(state: &AppState, headers: &HeaderMap) -> AppResult<GitHubClient> {
    let session = require_session(state, headers).await?;
    github_for_user(state, session.user_id).await
}

/// 根据协作者会话解析出 owner 的 GitHub 客户端。
async fn share_access_github(
    state: &AppState,
    session: &crate::db::ShareAccessSession,
) -> AppResult<GitHubClient> {
    github_for_user(state, session.owner_user_id).await
}

async fn public_data_source(state: &AppState, headers: &HeaderMap) -> AppResult<PublicDataSource> {
    let Some(session) = auth::current_session(state, headers).await? else {
        return Ok(PublicDataSource::Demo);
    };

    let Some(repo_config) = state
        .db
        .repo_config_optional(session.user_id, &state.secrets)
        .await?
    else {
        return Ok(PublicDataSource::UserWithoutConfig);
    };

    Ok(PublicDataSource::UserRepo(GitHubClient::from_repo_config(
        repo_config,
    )))
}

/// 读取 sets.json，缺失时返回空列表。
async fn load_sets(github: &GitHubClient) -> AppResult<(Vec<IconSetSummary>, Option<String>)> {
    let Some(file) = github
        .get_json::<Vec<IconSetSummary>>(SETS_INDEX_PATH)
        .await?
    else {
        return Ok((Vec::new(), None));
    };
    let mut sets = file.value;
    sort_sets(&mut sets);

    Ok((sets, Some(file.sha)))
}

/// 将 sets.json 保存回 GitHub。
async fn save_sets(
    github: &GitHubClient,
    sets: &[IconSetSummary],
    sha: Option<&str>,
    message: &str,
) -> AppResult<()> {
    let content = serde_json::to_vec_pretty(sets)?;
    github
        .put_file(SETS_INDEX_PATH, &content, message, sha)
        .await
}

/// 读取并规范化某个集合的 manifest。
async fn load_manifest(github: &GitHubClient, set_id: &str) -> AppResult<(IconManifest, String)> {
    let path = manifest_path(set_id);
    let Some(file) = github.get_json::<IconManifest>(&path).await? else {
        return Err(AppError::NotFound);
    };
    let mut manifest = file.value;
    normalize_manifest(github, set_id, &mut manifest);

    Ok((manifest, file.sha))
}

/// 将 manifest 保存回 GitHub。
async fn save_manifest(
    github: &GitHubClient,
    manifest: &IconManifest,
    sha: Option<&str>,
    message: &str,
) -> AppResult<()> {
    let mut manifest = manifest.clone();
    sort_icons(&mut manifest.icons);
    let content = serde_json::to_vec_pretty(&manifest)?;
    github
        .put_file(&manifest_path(&manifest.id), &content, message, sha)
        .await
}

/// 根据 manifest 同步 sets.json 中的摘要信息。
async fn sync_set_summary(github: &GitHubClient, manifest: &IconManifest) -> AppResult<()> {
    let (mut sets, sets_sha) = load_sets(github).await?;
    let Some(summary) = sets.iter_mut().find(|set| set.id == manifest.id) else {
        return Ok(());
    };

    summary.name = manifest.name.clone();
    summary.description = manifest.description.clone();
    summary.icon_count = manifest.icons.len();
    summary.updated_at = manifest.updated_at.clone();
    sort_sets(&mut sets);
    save_sets(github, &sets, sets_sha.as_deref(), "Update sets index").await
}

/// 删除 GitHub 文件，文件不存在时视为已经删除。
async fn delete_github_file_if_exists(
    github: &GitHubClient,
    path: &str,
    message: &str,
) -> AppResult<()> {
    let Some(file) = github.get_file(path).await? else {
        return Ok(());
    };
    github.delete_file(path, &file.sha, message).await
}

/// 计算图片内容 MD5，用于上传去重校验。
fn file_md5(bytes: &[u8]) -> String {
    format!("{:x}", md5::compute(bytes))
}

/// 为历史 manifest 中缺失 MD5 的图标补齐内容哈希。
async fn hydrate_missing_icon_md5(
    github: &GitHubClient,
    manifest: &mut IconManifest,
) -> AppResult<()> {
    for icon in &mut manifest.icons {
        if !icon.md5.is_empty() || icon.path.is_empty() {
            continue;
        }

        if let Some(file) = github.get_file(&icon.path).await? {
            icon.md5 = file_md5(&file.content);
        }
    }

    Ok(())
}

/// 确认待上传图片内容没有在当前集合中出现过。
fn ensure_unique_icon_md5(manifest: &IconManifest, md5: &str) -> AppResult<()> {
    if md5.is_empty() {
        return Ok(());
    }

    if let Some(icon) = manifest
        .icons
        .iter()
        .find(|icon| !icon.md5.is_empty() && icon.md5.eq_ignore_ascii_case(md5))
    {
        return Err(AppError::Conflict(format!(
            "图片内容已存在，已登记为 {}（{}）",
            icon.name, icon.path
        )));
    }

    Ok(())
}

/// 确认批量上传图片之间、以及它们和当前集合之间都没有重复内容。
fn ensure_unique_batch_md5s(manifest: &IconManifest, uploads: &[BatchUploadFile]) -> AppResult<()> {
    let mut seen = HashMap::<String, String>::new();

    for upload in uploads {
        ensure_unique_icon_md5(manifest, &upload.md5)?;
        if let Some(existing_file_name) = seen.insert(upload.md5.clone(), upload.file_name.clone())
        {
            return Err(AppError::Conflict(format!(
                "批量上传中存在重复图片：{} 和 {}",
                existing_file_name, upload.file_name
            )));
        }
    }

    Ok(())
}

/// 规范化 manifest 中的派生字段和排序。
fn normalize_manifest(github: &GitHubClient, set_id: &str, manifest: &mut IconManifest) {
    manifest.id = set_id.to_string();
    manifest.updated_at = if manifest.updated_at.is_empty() {
        now_iso()
    } else {
        manifest.updated_at.clone()
    };

    for icon in &mut manifest.icons {
        if !icon.path.is_empty() {
            icon.url = github.raw_url(&icon.path);
        }
    }
    sort_icons(&mut manifest.icons);
}

/// 生成某个集合的 manifest 路径。
fn manifest_path(set_id: &str) -> String {
    format!("sets/{set_id}/manifest.json")
}

/// 校验分享 manifest 地址必须是公网 http/https 链接。
fn validate_share_manifest_url(value: &str) -> AppResult<String> {
    let manifest_url = validate_required_text(value, "icon_set_url", 2000)?;
    let parsed = reqwest::Url::parse(&manifest_url)
        .map_err(|err| AppError::BadRequest(format!("icon_set_url 不是合法 URL：{err}")))?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::BadRequest(
            "icon_set_url 只支持 http 或 https 地址".to_string(),
        ));
    }
    if parsed.host_str().is_none() {
        return Err(AppError::BadRequest(
            "icon_set_url 必须包含可访问的域名".to_string(),
        ));
    }
    if !parsed.path().ends_with("/manifest.json") {
        return Err(AppError::BadRequest(
            "icon_set_url 必须指向 manifest.json 文件".to_string(),
        ));
    }

    Ok(parsed.to_string())
}

/// 规范化分享 manifest，保证页面渲染所需字段完整可用。
fn normalize_shared_manifest(manifest_url: &str, manifest: &mut IconManifest) -> AppResult<()> {
    manifest.name = validate_required_text(&manifest.name, "分享 manifest 名称", 200)?;
    manifest.description = validate_optional_text(&manifest.description, 2000)?;
    if manifest.updated_at.trim().is_empty() {
        manifest.updated_at = now_iso();
    }
    if manifest.id.trim().is_empty() {
        manifest.id = shared_manifest_id(manifest_url);
    }

    for (index, icon) in manifest.icons.iter_mut().enumerate() {
        icon.name = validate_required_text(&icon.name, "分享图标名称", ICON_NAME_MAX_LEN)?;
        icon.url = validate_share_icon_url(&icon.url)?;

        if icon.id.trim().is_empty() {
            icon.id = format!("shared-icon-{}", index + 1);
        }
    }

    sort_icons(&mut manifest.icons);

    Ok(())
}

/// 校验分享 manifest 中的图标地址必须可直接访问。
fn validate_share_icon_url(value: &str) -> AppResult<String> {
    let icon_url = validate_required_text(value, "分享图标地址", 2000)?;
    let parsed = reqwest::Url::parse(&icon_url)
        .map_err(|err| AppError::BadRequest(format!("分享图标地址不是合法 URL：{err}")))?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::BadRequest(
            "分享图标地址只支持 http 或 https 协议".to_string(),
        ));
    }

    Ok(parsed.to_string())
}

/// 从分享 manifest 地址推导稳定的集合 ID。
fn shared_manifest_id(manifest_url: &str) -> String {
    let digest = format!("{:x}", md5::compute(manifest_url));
    format!("shared-{}", &digest[..12])
}

/// 计算协作分享 token 的固定摘要，用于数据库索引和比对。
fn digest_token(token: &str) -> String {
    format!("{:x}", md5::compute(token.trim()))
}

/// 计算协作分享 password 的固定摘要。
fn digest_password(password: &str) -> String {
    format!("{:x}", md5::compute(password.trim()))
}

/// 检查协作分享链接当前是否可用。
fn ensure_share_access_available(share: &crate::db::ShareAccessGrant) -> AppResult<()> {
    if share.revoked_at.is_some() {
        return Err(AppError::Forbidden);
    }
    if share
        .expires_at
        .map(|value| value <= Utc::now())
        .unwrap_or(false)
    {
        return Err(AppError::Forbidden);
    }

    Ok(())
}

/// 拼出协作者进入共享编辑页时使用的链接。
fn collab_share_url(token: &str) -> String {
    format!("/share/edit?token={}", urlencoding::encode(token))
}

/// 生成可直接复制给协作者的完整共享编辑链接。
fn absolute_collab_share_url(state: &AppState, token: &str) -> String {
    format!(
        "{}{}",
        state.config.app_base_url.trim_end_matches('/'),
        collab_share_url(token)
    )
}

/// 生成 ISO 8601 更新时间。
fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

/// 按集合名称排序。
fn sort_sets(sets: &mut [IconSetSummary]) {
    sets.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
}

/// 按图标名称升序排序。
fn sort_icons(icons: &mut [IconEntry]) {
    icons.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
}

/// 校验集合 ID 只能使用安全路径字符。
fn validate_set_id(set_id: &str) -> AppResult<()> {
    let valid = !set_id.is_empty()
        && set_id.len() <= 64
        && set_id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        });

    if valid {
        return Ok(());
    }

    Err(AppError::BadRequest(
        "集合 ID 只能包含小写字母、数字、-、_，长度不能超过 64".to_string(),
    ))
}

/// 校验必填文本并返回去除首尾空格后的内容。
fn validate_required_text(value: &str, field_name: &str, max_len: usize) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(format!("{field_name}不能为空")));
    }
    if trimmed.chars().count() > max_len {
        return Err(AppError::BadRequest(format!(
            "{field_name}不能超过 {max_len} 个字符"
        )));
    }

    Ok(trimmed.to_string())
}

/// 校验图标名称只能包含英文字母、数字、空格和 .-_。
fn validate_icon_name(value: &str) -> AppResult<String> {
    if value.ends_with(' ') {
        return Err(AppError::BadRequest("图标名称最后不能是空格".to_string()));
    }

    let name = validate_required_text(value, "图标名称", ICON_NAME_MAX_LEN)?;
    let valid = name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b' ' | b'.' | b'-' | b'_'));

    if valid {
        return Ok(name);
    }

    Err(AppError::BadRequest(
        "图标名称只能包含英文字母、数字、空格和 .、-、_".to_string(),
    ))
}

/// 校验可选文本并返回去除首尾空格后的内容。
fn validate_optional_text(value: &str, max_len: usize) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.chars().count() > max_len {
        return Err(AppError::BadRequest(format!(
            "描述不能超过 {max_len} 个字符"
        )));
    }

    Ok(trimmed.to_string())
}

/// 根据名称生成路径安全的 slug。
fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_sep = false;

    for ch in value.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_sep = false;
        } else if matches!(ch, '-' | '_' | ' ' | '.') && !last_was_sep {
            slug.push('-');
            last_was_sep = true;
        }
    }

    slug.trim_matches('-').to_string()
}

/// 根据文件名和 Content-Type 判断允许的图片扩展名。
fn detect_extension(file_name: &str, content_type: Option<&str>) -> AppResult<String> {
    let file_extension = file_name
        .rsplit('.')
        .next()
        .map(|value| value.to_lowercase())
        .filter(|value| value != file_name);

    if let Some(extension) = file_extension.filter(|value| is_allowed_extension(value)) {
        return Ok(extension);
    }

    match content_type.unwrap_or_default() {
        "image/png" => Ok("png".to_string()),
        "image/jpeg" => Ok("jpg".to_string()),
        "image/webp" => Ok("webp".to_string()),
        "image/svg+xml" => Ok("svg".to_string()),
        _ => Err(AppError::BadRequest(
            "只支持 png、jpg、jpeg、webp、svg 图片".to_string(),
        )),
    }
}

/// 确认批量上传累计体积不超过限制。
fn checked_total_upload_bytes(current: usize, next: usize) -> AppResult<usize> {
    let total = current
        .checked_add(next)
        .ok_or_else(|| AppError::BadRequest("批量图片总体积不能超过 10MB".to_string()))?;

    if total > limits::BATCH_UPLOAD_MAX_BYTES {
        return Err(AppError::BadRequest(
            "批量图片总体积不能超过 10MB".to_string(),
        ));
    }

    Ok(total)
}

/// 校验直接上传的文件必须是支持的图片。
fn ensure_allowed_image_file(file_name: &str) -> AppResult<()> {
    detect_extension(file_name, None).map(|_| ())
}

/// 从 zip 压缩包里提取支持的图片文件。
fn extract_zip_images(file_name: &str, bytes: Vec<u8>) -> AppResult<Vec<BatchUploadFile>> {
    let extension = file_name.rsplit('.').next().unwrap_or_default();
    if !extension.eq_ignore_ascii_case("zip") {
        return Err(AppError::BadRequest("压缩包只支持 zip 格式".to_string()));
    }

    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader)
        .map_err(|err| AppError::BadRequest(format!("zip 压缩包无效：{err}")))?;
    let mut uploads = Vec::new();
    let mut extracted_total = 0usize;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| AppError::BadRequest(format!("zip 文件读取失败：{err}")))?;
        if entry.is_dir() {
            continue;
        }

        let Some(entry_name) = archive_image_file_name(entry.name()) else {
            continue;
        };
        if detect_extension(&entry_name, None).is_err() {
            continue;
        }

        let mut content = Vec::new();
        entry
            .read_to_end(&mut content)
            .map_err(|err| AppError::BadRequest(format!("zip 图片读取失败：{err}")))?;
        if content.is_empty() {
            continue;
        }
        extracted_total = checked_total_upload_bytes(extracted_total, content.len())?;
        uploads.push(BatchUploadFile::new(entry_name, content));
    }

    Ok(uploads)
}

/// 从 zip entry 路径里提取可用文件名，跳过系统隐藏文件。
fn archive_image_file_name(path: &str) -> Option<String> {
    let file_name = path
        .rsplit(['/', '\\'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;

    if file_name.starts_with('.') || path.starts_with("__MACOSX/") {
        return None;
    }

    Some(file_name.to_string())
}

/// 判断扩展名是否在允许的图片类型内。
fn is_allowed_extension(extension: &str) -> bool {
    matches!(extension, "png" | "jpg" | "jpeg" | "webp" | "svg")
}

/// 从文件名提取展示名称。
fn filename_stem(file_name: &str) -> String {
    let stem = file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name);
    sanitize_icon_name(stem).unwrap_or_else(|| "Icon".to_string())
}

/// 从文件名中提取符合图标名称规则的安全名称。
fn sanitize_icon_name(value: &str) -> Option<String> {
    let mut name = String::new();

    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '.' | '-' | '_') {
            name.push(ch);
        }
    }

    let name = name.trim().chars().take(120).collect::<String>();
    if name.is_empty() { None } else { Some(name) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::{ZipWriter, write::SimpleFileOptions};

    fn icon(id: &str, name: &str) -> IconEntry {
        IconEntry {
            id: id.to_string(),
            name: name.to_string(),
            path: String::new(),
            url: String::new(),
            md5: String::new(),
        }
    }

    fn icon_with_md5(id: &str, name: &str, md5: &str) -> IconEntry {
        IconEntry {
            id: id.to_string(),
            name: name.to_string(),
            path: format!("sets/emby/icons/{name}.png"),
            url: String::new(),
            md5: md5.to_string(),
        }
    }

    #[test]
    fn sort_icons_orders_by_name_ascending_case_insensitive() {
        let mut icons = vec![
            icon("1", "Alpha"),
            icon("2", "zulu"),
            icon("3", "Echo Room"),
            icon("4", "bravo"),
        ];

        sort_icons(&mut icons);

        let names = icons.into_iter().map(|icon| icon.name).collect::<Vec<_>>();
        assert_eq!(names, vec!["Alpha", "bravo", "Echo Room", "zulu"]);
    }

    #[test]
    fn validate_icon_name_allows_only_letters_spaces_and_safe_symbols() {
        assert_eq!(
            validate_icon_name("  Emby Room 2._-").unwrap(),
            "Emby Room 2._-"
        );

        assert!(validate_icon_name("Emby Room ").is_err());
        assert!(validate_icon_name("Emby/Room").is_err());
        assert!(validate_icon_name("Emby中文").is_err());
        assert!(validate_icon_name("Emby\tRoom").is_err());
    }

    #[test]
    fn numbered_icon_name_appends_two_digit_suffix() {
        assert_eq!(numbered_icon_name("xx", 0), "xx");
        assert_eq!(numbered_icon_name("xx", 1), "xx_01");
        assert_eq!(numbered_icon_name("xx", 12), "xx_12");
    }

    #[test]
    fn icon_path_uses_slugified_name() {
        assert_eq!(icon_path("emby", "jack", "png"), "sets/emby/icons/jack.png");
        assert_eq!(
            icon_path("emby", "feiyue_caihong", "png"),
            "sets/emby/icons/feiyue-caihong.png"
        );
        assert_eq!(icon_path("emby", "fych", "png"), "sets/emby/icons/fych.png");
    }

    #[test]
    fn filename_stem_sanitizes_batch_names() {
        assert_eq!(filename_stem("jack.png"), "jack");
        assert_eq!(filename_stem("feiyue caihong.jpeg"), "feiyue caihong");
        assert_eq!(filename_stem("中文.png"), "Icon");
    }

    #[test]
    fn extract_zip_images_keeps_only_supported_images() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file("folder/jack.png", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"png").unwrap();
        writer
            .start_file("__MACOSX/._jack.png", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"hidden").unwrap();
        writer
            .start_file("folder/readme.txt", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"text").unwrap();
        let bytes = writer.finish().unwrap().into_inner();

        let uploads = extract_zip_images("icons.zip", bytes).unwrap();

        assert_eq!(uploads.len(), 1);
        assert_eq!(uploads[0].file_name, "jack.png");
        assert_eq!(uploads[0].bytes, b"png");
        assert_eq!(uploads[0].md5, file_md5(b"png"));
    }

    #[test]
    fn file_md5_returns_expected_digest() {
        assert_eq!(file_md5(b"hello"), "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn ensure_unique_icon_md5_rejects_existing_icon_in_current_manifest() {
        let manifest = IconManifest {
            id: "emby".to_string(),
            name: "Emby".to_string(),
            description: String::new(),
            icons: vec![icon_with_md5(
                "1",
                "alpha",
                "5d41402abc4b2a76b9719d911017c592",
            )],
            updated_at: String::new(),
        };

        assert!(ensure_unique_icon_md5(&manifest, file_md5(b"world").as_str()).is_ok());
        assert!(ensure_unique_icon_md5(&manifest, file_md5(b"hello").as_str()).is_err());
    }

    #[test]
    fn ensure_unique_batch_md5s_rejects_duplicates_inside_batch() {
        let manifest = IconManifest {
            id: "emby".to_string(),
            name: "Emby".to_string(),
            description: String::new(),
            icons: Vec::new(),
            updated_at: String::new(),
        };
        let uploads = vec![
            BatchUploadFile::new("alpha.png".to_string(), b"same".to_vec()),
            BatchUploadFile::new("beta.png".to_string(), b"same".to_vec()),
        ];

        assert!(ensure_unique_batch_md5s(&manifest, &uploads).is_err());
    }
}

/// 生成集合内唯一的图标名称和图片路径。
async fn unique_icon_name_and_path(
    github: &GitHubClient,
    manifest: &IconManifest,
    set_id: &str,
    requested_name: &str,
    extension: &str,
    current_icon_id: Option<&str>,
    current_path: Option<&str>,
) -> AppResult<(String, String)> {
    let mut index = 0;

    loop {
        let candidate_name = numbered_icon_name(requested_name, index);
        let candidate_path = icon_path(set_id, &candidate_name, extension);
        let name_exists = manifest.icons.iter().any(|icon| {
            Some(icon.id.as_str()) != current_icon_id
                && icon.name.eq_ignore_ascii_case(&candidate_name)
        });
        let path_exists = if Some(candidate_path.as_str()) == current_path {
            false
        } else {
            github.get_file(&candidate_path).await?.is_some()
        };

        if !name_exists && !path_exists {
            return Ok((candidate_name, candidate_path));
        }

        index += 1;
    }
}

/// 按 xx、xx_01、xx_02 的规则生成候选图标名称。
fn numbered_icon_name(base_name: &str, index: usize) -> String {
    if index == 0 {
        return base_name.to_string();
    }

    let suffix = format!("_{index:02}");
    let max_base_len = ICON_NAME_MAX_LEN.saturating_sub(suffix.chars().count());
    let mut base = base_name
        .chars()
        .take(max_base_len)
        .collect::<String>()
        .trim_end()
        .to_string();

    if base.is_empty() {
        base = "icon".to_string();
    }

    format!("{base}{suffix}")
}

/// 生成图标图片在仓库中的路径。
fn icon_path(set_id: &str, name: &str, extension: &str) -> String {
    let slug = {
        let slug = slugify(name);
        if slug.is_empty() {
            "icon".to_string()
        } else {
            slug
        }
    };

    format!("sets/{set_id}/icons/{slug}.{extension}")
}

/// 从已有图标路径里提取图片扩展名。
fn icon_extension(path: &str) -> AppResult<String> {
    let extension = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_lowercase())
        .filter(|extension| is_allowed_extension(extension));

    extension.ok_or_else(|| AppError::BadRequest("无法识别原图片扩展名".to_string()))
}
