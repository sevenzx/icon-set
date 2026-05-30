use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState, auth,
    error::{AppError, AppResult},
    models::{
        CreateSetRequest, IconEntry, IconManifest, IconSetSummary, LoginRequest, RenameIconRequest,
        SessionResponse, UpdateSetRequest,
    },
};

const SETS_INDEX_PATH: &str = "sets.json";

/// 返回健康检查状态。
pub async fn health() -> Json<serde_json::Value> {
    Json(json!({ "ok": true }))
}

/// 列出公开图标集合。
pub async fn list_sets(State(state): State<AppState>) -> AppResult<Json<Vec<IconSetSummary>>> {
    let (sets, _) = load_sets(&state).await?;
    Ok(Json(sets))
}

/// 读取某个公开图标集合的 manifest。
pub async fn get_set(
    State(state): State<AppState>,
    Path(set_id): Path<String>,
) -> AppResult<Json<IconManifest>> {
    validate_set_id(&set_id)?;
    let (manifest, _) = load_manifest(&state, &set_id).await?;
    Ok(Json(manifest))
}

/// 使用简单密码登录管理员后台。
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Response> {
    if !auth::password_matches(&payload.password, &state.config.admin_password) {
        return Err(AppError::Unauthorized);
    }

    let session = auth::create_session(&state).await;
    let cookie = auth::session_cookie_value(&state, &session.cookie_token);
    let mut response = Json(SessionResponse {
        authenticated: true,
        admin_token: Some(session.admin_token),
    })
    .into_response();
    auth::set_cookie_header(response.headers_mut(), cookie)?;

    Ok(response)
}

/// 清理当前管理员会话。
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    auth::destroy_session(&state, &headers).await;
    let cookie = auth::expired_session_cookie_value(&state);
    let mut response = Json(SessionResponse {
        authenticated: false,
        admin_token: None,
    })
    .into_response();
    auth::set_cookie_header(response.headers_mut(), cookie)?;

    Ok(response)
}

/// 查询当前请求是否已经登录。
pub async fn session(State(state): State<AppState>, headers: HeaderMap) -> Json<SessionResponse> {
    Json(SessionResponse {
        authenticated: auth::is_authenticated(&state, &headers).await,
        admin_token: None,
    })
}

/// 创建一个新的图标集合。
pub async fn create_set(
    State(state): State<AppState>,
    Json(payload): Json<CreateSetRequest>,
) -> AppResult<Json<IconSetSummary>> {
    let set_id = slugify(&payload.id);
    validate_set_id(&set_id)?;
    let name = validate_required_text(&payload.name, "集合名称", 120)?;
    let description = validate_optional_text(&payload.description, 800)?;
    let (mut sets, sets_sha) = load_sets(&state).await?;

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

    if state.github.get_file(&manifest_path).await?.is_some() {
        return Err(AppError::Conflict(manifest_path));
    }

    save_manifest(
        &state,
        &manifest,
        None,
        &format!("Create icon set {set_id}"),
    )
    .await?;
    sets.push(summary.clone());
    sort_sets(&mut sets);
    save_sets(&state, &sets, sets_sha.as_deref(), "Update sets index").await?;

    Ok(Json(summary))
}

/// 更新图标集合的名称和描述。
pub async fn update_set(
    State(state): State<AppState>,
    Path(set_id): Path<String>,
    Json(payload): Json<UpdateSetRequest>,
) -> AppResult<Json<IconSetSummary>> {
    validate_set_id(&set_id)?;

    let (mut sets, sets_sha) = load_sets(&state).await?;
    let Some(summary) = sets.iter_mut().find(|set| set.id == set_id) else {
        return Err(AppError::NotFound);
    };
    let (mut manifest, manifest_sha) = load_manifest(&state, &set_id).await?;

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
        &state,
        &manifest,
        Some(&manifest_sha),
        &format!("Update icon set {set_id}"),
    )
    .await?;
    sort_sets(&mut sets);
    save_sets(&state, &sets, sets_sha.as_deref(), "Update sets index").await?;

    Ok(Json(updated_summary))
}

/// 删除一个图标集合及其 manifest 中登记的图片。
pub async fn delete_set(
    State(state): State<AppState>,
    Path(set_id): Path<String>,
) -> AppResult<Json<Vec<IconSetSummary>>> {
    validate_set_id(&set_id)?;

    let (mut sets, sets_sha) = load_sets(&state).await?;
    let original_len = sets.len();
    sets.retain(|set| set.id != set_id);
    if sets.len() == original_len {
        return Err(AppError::NotFound);
    }

    if let Ok((manifest, manifest_sha)) = load_manifest(&state, &set_id).await {
        for icon in manifest.icons.iter().filter(|icon| !icon.path.is_empty()) {
            delete_github_file_if_exists(&state, &icon.path, &format!("Delete icon {}", icon.name))
                .await?;
        }
        state
            .github
            .delete_file(
                &manifest_path(&set_id),
                &manifest_sha,
                &format!("Delete icon set {set_id}"),
            )
            .await?;
    }

    save_sets(&state, &sets, sets_sha.as_deref(), "Update sets index").await?;
    Ok(Json(sets))
}

/// 上传图片并写入对应集合的 manifest。
pub async fn upload_icon(
    State(state): State<AppState>,
    Path(set_id): Path<String>,
    mut multipart: Multipart,
) -> AppResult<Json<IconManifest>> {
    validate_set_id(&set_id)?;

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
    let (mut manifest, manifest_sha) = load_manifest(&state, &set_id).await?;
    let icon_id = Uuid::new_v4().to_string();
    let path = unique_icon_path(&state, &set_id, &name, &extension, &icon_id).await?;

    state
        .github
        .put_file(&path, &upload.bytes, &format!("Add icon {name}"), None)
        .await?;

    manifest.icons.push(IconEntry {
        id: icon_id,
        name,
        path: path.clone(),
        url: state.github.raw_url(&path),
    });
    normalize_manifest(&state, &set_id, &mut manifest);
    save_manifest(
        &state,
        &manifest,
        Some(&manifest_sha),
        &format!("Update icon set {set_id}"),
    )
    .await?;
    sync_set_summary(&state, &manifest).await?;

    Ok(Json(manifest))
}

/// 修改指定图标的名称。
pub async fn rename_icon(
    State(state): State<AppState>,
    Path((set_id, icon_id)): Path<(String, String)>,
    Json(payload): Json<RenameIconRequest>,
) -> AppResult<Json<IconManifest>> {
    validate_set_id(&set_id)?;

    let name = validate_icon_name(&payload.name)?;
    let (mut manifest, manifest_sha) = load_manifest(&state, &set_id).await?;
    let Some(icon) = manifest.icons.iter_mut().find(|icon| icon.id == icon_id) else {
        return Err(AppError::NotFound);
    };
    icon.name = name;
    manifest.updated_at = now_iso();
    sort_icons(&mut manifest.icons);

    save_manifest(
        &state,
        &manifest,
        Some(&manifest_sha),
        &format!("Rename icon {icon_id}"),
    )
    .await?;
    sync_set_summary(&state, &manifest).await?;

    Ok(Json(manifest))
}

/// 删除指定图标及其 GitHub 图片文件。
pub async fn delete_icon(
    State(state): State<AppState>,
    Path((set_id, icon_id)): Path<(String, String)>,
) -> AppResult<Json<IconManifest>> {
    validate_set_id(&set_id)?;

    let (mut manifest, manifest_sha) = load_manifest(&state, &set_id).await?;
    let Some(position) = manifest.icons.iter().position(|icon| icon.id == icon_id) else {
        return Err(AppError::NotFound);
    };
    let icon = manifest.icons.remove(position);

    if !icon.path.is_empty() {
        delete_github_file_if_exists(&state, &icon.path, &format!("Delete icon {}", icon.name))
            .await?;
    }

    manifest.updated_at = now_iso();
    save_manifest(
        &state,
        &manifest,
        Some(&manifest_sha),
        &format!("Update icon set {set_id}"),
    )
    .await?;
    sync_set_summary(&state, &manifest).await?;

    Ok(Json(manifest))
}

struct UploadFile {
    file_name: String,
    content_type: Option<String>,
    bytes: Vec<u8>,
}

/// 读取 sets.json，缺失时返回空列表。
async fn load_sets(state: &AppState) -> AppResult<(Vec<IconSetSummary>, Option<String>)> {
    let Some(file) = state
        .github
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
    state: &AppState,
    sets: &[IconSetSummary],
    sha: Option<&str>,
    message: &str,
) -> AppResult<()> {
    let content = serde_json::to_vec_pretty(sets)?;
    state
        .github
        .put_file(SETS_INDEX_PATH, &content, message, sha)
        .await
}

/// 读取并规范化某个集合的 manifest。
async fn load_manifest(state: &AppState, set_id: &str) -> AppResult<(IconManifest, String)> {
    let path = manifest_path(set_id);
    let Some(file) = state.github.get_json::<IconManifest>(&path).await? else {
        return Err(AppError::NotFound);
    };
    let mut manifest = file.value;
    normalize_manifest(state, set_id, &mut manifest);

    Ok((manifest, file.sha))
}

/// 将 manifest 保存回 GitHub。
async fn save_manifest(
    state: &AppState,
    manifest: &IconManifest,
    sha: Option<&str>,
    message: &str,
) -> AppResult<()> {
    let content = serde_json::to_vec_pretty(manifest)?;
    state
        .github
        .put_file(&manifest_path(&manifest.id), &content, message, sha)
        .await
}

/// 根据 manifest 同步 sets.json 中的摘要信息。
async fn sync_set_summary(state: &AppState, manifest: &IconManifest) -> AppResult<()> {
    let (mut sets, sets_sha) = load_sets(state).await?;
    let Some(summary) = sets.iter_mut().find(|set| set.id == manifest.id) else {
        return Ok(());
    };

    summary.name = manifest.name.clone();
    summary.description = manifest.description.clone();
    summary.icon_count = manifest.icons.len();
    summary.updated_at = manifest.updated_at.clone();
    sort_sets(&mut sets);
    save_sets(state, &sets, sets_sha.as_deref(), "Update sets index").await
}

/// 删除 GitHub 文件，文件不存在时视为已经删除。
async fn delete_github_file_if_exists(
    state: &AppState,
    path: &str,
    message: &str,
) -> AppResult<()> {
    let Some(file) = state.github.get_file(path).await? else {
        return Ok(());
    };
    state.github.delete_file(path, &file.sha, message).await
}

/// 规范化 manifest 中的派生字段和排序。
fn normalize_manifest(state: &AppState, set_id: &str, manifest: &mut IconManifest) {
    manifest.id = set_id.to_string();
    manifest.updated_at = if manifest.updated_at.is_empty() {
        now_iso()
    } else {
        manifest.updated_at.clone()
    };

    for icon in &mut manifest.icons {
        if !icon.path.is_empty() {
            icon.url = state.github.raw_url(&icon.path);
        }
    }
    sort_icons(&mut manifest.icons);
}

/// 生成某个集合的 manifest 路径。
fn manifest_path(set_id: &str) -> String {
    format!("sets/{set_id}/manifest.json")
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

/// 按图标名称降序排序。
fn sort_icons(icons: &mut [IconEntry]) {
    icons.sort_by(|left, right| {
        right
            .name
            .to_lowercase()
            .cmp(&left.name.to_lowercase())
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

/// 校验图标名称只能包含英文字母、空格和 .-_。
fn validate_icon_name(value: &str) -> AppResult<String> {
    if value.ends_with(' ') {
        return Err(AppError::BadRequest("图标名称最后不能是空格".to_string()));
    }

    let name = validate_required_text(value, "图标名称", 120)?;
    let valid = name
        .bytes()
        .all(|byte| byte.is_ascii_alphabetic() || matches!(byte, b' ' | b'.' | b'-' | b'_'));

    if valid {
        return Ok(name);
    }

    Err(AppError::BadRequest(
        "图标名称只能包含英文字母、空格和 .、-、_".to_string(),
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
        if ch.is_ascii_alphabetic() || matches!(ch, ' ' | '.' | '-' | '_') {
            name.push(ch);
        }
    }

    let name = name.trim().chars().take(120).collect::<String>();
    if name.is_empty() { None } else { Some(name) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon(id: &str, name: &str) -> IconEntry {
        IconEntry {
            id: id.to_string(),
            name: name.to_string(),
            path: String::new(),
            url: String::new(),
        }
    }

    #[test]
    fn sort_icons_orders_by_name_descending_case_insensitive() {
        let mut icons = vec![
            icon("1", "Alpha"),
            icon("2", "zulu"),
            icon("3", "Echo Room"),
            icon("4", "bravo"),
        ];

        sort_icons(&mut icons);

        let names = icons.into_iter().map(|icon| icon.name).collect::<Vec<_>>();
        assert_eq!(names, vec!["zulu", "Echo Room", "bravo", "Alpha"]);
    }

    #[test]
    fn validate_icon_name_allows_only_letters_spaces_and_safe_symbols() {
        assert_eq!(
            validate_icon_name("  Emby Room._-").unwrap(),
            "Emby Room._-"
        );

        assert!(validate_icon_name("Emby Room ").is_err());
        assert!(validate_icon_name("Emby2").is_err());
        assert!(validate_icon_name("Emby/Room").is_err());
        assert!(validate_icon_name("Emby中文").is_err());
        assert!(validate_icon_name("Emby\tRoom").is_err());
    }
}

/// 生成不会与仓库现有文件冲突的图片路径。
async fn unique_icon_path(
    state: &AppState,
    set_id: &str,
    name: &str,
    extension: &str,
    icon_id: &str,
) -> AppResult<String> {
    let base_slug = {
        let slug = slugify(name);
        if slug.is_empty() {
            "icon".to_string()
        } else {
            slug
        }
    };
    let mut path = format!("sets/{set_id}/icons/{base_slug}.{extension}");

    // GitHub 创建文件时不能覆盖已有路径，冲突时使用短 ID 后缀兜底。
    if state.github.get_file(&path).await?.is_some() {
        let short_id = icon_id.chars().take(8).collect::<String>();
        path = format!("sets/{set_id}/icons/{base_slug}-{short_id}.{extension}");
    }

    Ok(path)
}
