use crate::AppState;
use crate::actions::access;
use crate::file_utils::{
    canonicalize_data_file_path, data_dir_relative_file_path, deletable_data_file_path,
};
use a4n_core::files::paths::ProjectPaths;
use a4n_core::models::ai::{generated_image, generated_video};
use a4n_services::auth::jwt;
use a4n_services::image::CreateImageRecord;
use a4n_services::video::is_project_reusable_video_material_image;
use base64::Engine as _;
use reqwest::header::{CONTENT_TYPE, RANGE};
use sea_orm::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const MAX_REFERENCE_IMAGE_BYTES: u64 = 20 * 1024 * 1024;
const MAX_REFERENCE_IMAGE_TOTAL_BYTES: u64 = 32 * 1024 * 1024;
const DEFAULT_MEDIA_TOKEN_TTL_MINUTES: u64 = 360;
const MEDIA_TOKEN_TTL_ENV: &str = "A4N_VIDEO_MEDIA_TOKEN_TTL_MINUTES";
const REMOTE_TEMP_UPLOAD_URL_ENV: &str = "A4N_VIDEO_REMOTE_TEMP_UPLOAD_URL";
const REMOTE_TEMP_UPLOAD_TOKEN_ENV: &str = "A4N_VIDEO_REMOTE_TEMP_UPLOAD_TOKEN";
const PROVIDER_IMAGE_URL_PROBE_TIMEOUT_SECS: u64 = 20;

#[derive(Debug, Default)]
pub(crate) struct ReferenceImagePreviewPlaceholders {
    pub(crate) urls: Vec<String>,
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderImageInputTransport {
    PublicUrl,
    InlineDataUrl,
    RemoteTempUpload,
}

#[derive(Debug, Serialize)]
struct RemoteTempMediaUploadRequest {
    file_name: String,
    mime_type: String,
    data_base64: String,
}

#[derive(Debug, Deserialize)]
struct RemoteTempMediaUploadResponse {
    url: String,
}

fn required_trim_chapter_id(source: &generated_video::Model) -> Result<i64, String> {
    source
        .chapter_id
        .filter(|id| *id > 0)
        .ok_or_else(|| format!("源视频 #{} 缺少 chapter_id，无法裁剪", source.id))
}

pub(crate) async fn reference_image_preview_placeholders(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    value: Option<&Value>,
) -> Result<ReferenceImagePreviewPlaceholders, String> {
    let ids = reference_image_ids(value)?;
    if ids.is_empty() {
        return Ok(ReferenceImagePreviewPlaceholders::default());
    }
    reference_image_preview_placeholders_for_ids(state, user_id, project_id, chapter_id, ids).await
}

pub(crate) async fn reference_image_preview_placeholders_by_ids(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    ids: Vec<i64>,
) -> Result<ReferenceImagePreviewPlaceholders, String> {
    ensure_reference_image_ids_valid(&ids)?;
    if ids.is_empty() {
        return Ok(ReferenceImagePreviewPlaceholders::default());
    }
    reference_image_preview_placeholders_for_ids(state, user_id, project_id, chapter_id, ids).await
}

pub(crate) async fn reference_image_provider_inputs(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    value: Option<&Value>,
    transport: ProviderImageInputTransport,
) -> Result<Vec<String>, String> {
    let ids = reference_image_ids(value)?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    reference_image_provider_inputs_for_ids(state, user_id, project_id, chapter_id, ids, transport)
        .await
}

pub(crate) async fn reference_image_provider_inputs_by_ids(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    ids: Vec<i64>,
    transport: ProviderImageInputTransport,
) -> Result<Vec<String>, String> {
    ensure_reference_image_ids_valid(&ids)?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    reference_image_provider_inputs_for_ids(state, user_id, project_id, chapter_id, ids, transport)
        .await
}

pub(crate) async fn reference_image_provider_input_by_id(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    id: Option<i64>,
    transport: ProviderImageInputTransport,
) -> Result<Option<String>, String> {
    let Some(id) = id else {
        return Ok(None);
    };
    let mut urls = reference_image_provider_inputs_for_ids(
        state,
        user_id,
        project_id,
        chapter_id,
        vec![id],
        transport,
    )
    .await?;
    Ok(urls.pop())
}

pub(crate) async fn reference_image_preview_placeholder_by_id(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    id: Option<i64>,
) -> Result<(Option<String>, Vec<String>), String> {
    let Some(id) = id else {
        return Ok((None, Vec::new()));
    };
    let mut placeholders = reference_image_preview_placeholders_for_ids(
        state,
        user_id,
        project_id,
        chapter_id,
        vec![id],
    )
    .await?;
    Ok((placeholders.urls.pop(), placeholders.notes))
}

async fn reference_image_provider_inputs_for_ids(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    ids: Vec<i64>,
    transport: ProviderImageInputTransport,
) -> Result<Vec<String>, String> {
    let images =
        validated_reference_images_for_ids(state, user_id, project_id, chapter_id, ids).await?;
    let mut urls = Vec::with_capacity(images.len());
    let media_base_url = match transport {
        ProviderImageInputTransport::PublicUrl => Some(public_media_base_url()?),
        ProviderImageInputTransport::InlineDataUrl
        | ProviderImageInputTransport::RemoteTempUpload => None,
    };
    let mut inline_total_bytes = 0_u64;
    for image in images {
        let input = image_file_to_provider_input(
            state,
            user_id,
            media_base_url.as_deref(),
            &image,
            &mut inline_total_bytes,
            transport,
        )
        .await?;
        urls.push(input);
    }
    Ok(urls)
}

pub(crate) fn remote_temp_upload_configured() -> bool {
    remote_temp_upload_env(REMOTE_TEMP_UPLOAD_URL_ENV).is_some()
        && remote_temp_upload_env(REMOTE_TEMP_UPLOAD_TOKEN_ENV).is_some()
}

fn remote_temp_upload_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn remote_temp_upload_config() -> Result<(String, String), String> {
    let upload_url = remote_temp_upload_env(REMOTE_TEMP_UPLOAD_URL_ENV).ok_or_else(|| {
        format!(
            "{REMOTE_TEMP_UPLOAD_URL_ENV} 不能为空，remote_temp_upload 需要远端临时媒体上传地址"
        )
    })?;
    let token = remote_temp_upload_env(REMOTE_TEMP_UPLOAD_TOKEN_ENV).ok_or_else(|| {
        format!(
            "{REMOTE_TEMP_UPLOAD_TOKEN_ENV} 不能为空，remote_temp_upload 需要远端临时媒体上传令牌"
        )
    })?;
    Ok((upload_url, token))
}

async fn reference_image_preview_placeholders_for_ids(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    ids: Vec<i64>,
) -> Result<ReferenceImagePreviewPlaceholders, String> {
    let allowed_project_ids = state
        .video
        .video_asset_project_ids(project_id)
        .await
        .map_err(|e| e.to_string())?;
    let mut result = ReferenceImagePreviewPlaceholders {
        urls: Vec::with_capacity(ids.len()),
        notes: Vec::new(),
    };

    let images_by_id = load_reference_images_by_id(state, &ids).await?;
    let project_ids = reference_image_project_ids_in_reference_order(&ids, &images_by_id);
    verify_reference_image_project_editors(state, user_id, project_ids).await?;

    for id in ids {
        if let Some(image) = images_by_id.get(&id) {
            result.notes.extend(reference_image_preview_notes(
                project_id,
                chapter_id,
                &allowed_project_ids,
                image,
            ));
            result.urls.push(reference_image_preview_placeholder(image));
        } else {
            result.notes.push(format!(
                "参考图片 #{id} 不存在；预览使用占位媒体 URL 展示请求结构，真实生成会拒绝该输入"
            ));
            result.urls.push(format!(
                "https://example.invalid/a4n-media/preview/generated-image-{id}?missing_row=1"
            ));
        }
    }

    Ok(result)
}

async fn validated_reference_images_for_ids(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    ids: Vec<i64>,
) -> Result<Vec<generated_image::Model>, String> {
    let images = state
        .video
        .validated_video_reference_images(project_id, chapter_id, Some(&json!(ids)))
        .await
        .map_err(|e| e.to_string())?;
    let project_ids = unique_project_ids_from_images(&images);
    verify_reference_image_project_editors(state, user_id, project_ids).await?;
    Ok(images)
}

async fn load_reference_images_by_id(
    state: &AppState,
    ids: &[i64],
) -> Result<HashMap<i64, generated_image::Model>, String> {
    let rows = generated_image::Entity::find()
        .filter(generated_image::Column::Id.is_in(ids.iter().copied()))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|image| (image.id, image)).collect())
}

fn reference_image_project_ids_in_reference_order(
    ids: &[i64],
    images_by_id: &HashMap<i64, generated_image::Model>,
) -> Vec<i64> {
    let mut seen = HashSet::new();
    let mut project_ids = Vec::new();
    for id in ids {
        let Some(image) = images_by_id.get(id) else {
            continue;
        };
        if seen.insert(image.project_id) {
            project_ids.push(image.project_id);
        }
    }
    project_ids
}

fn unique_project_ids_from_images(images: &[generated_image::Model]) -> Vec<i64> {
    let mut seen = HashSet::new();
    let mut project_ids = Vec::new();
    for image in images {
        if seen.insert(image.project_id) {
            project_ids.push(image.project_id);
        }
    }
    project_ids
}

async fn verify_reference_image_project_editors(
    state: &AppState,
    user_id: i64,
    project_ids: Vec<i64>,
) -> Result<(), String> {
    for project_id in project_ids {
        access::verify_project_editor_string(state, user_id, project_id).await?;
    }
    Ok(())
}

fn reference_image_preview_notes(
    project_id: i64,
    chapter_id: i64,
    allowed_project_ids: &[i64],
    image: &generated_image::Model,
) -> Vec<String> {
    let mut notes = Vec::new();
    if !allowed_project_ids.contains(&image.project_id) {
        notes.push(format!(
            "参考图片 #{} 属于项目 #{}，不属于当前视频项目 #{} 或其根文本项目；真实生成会拒绝该输入",
            image.id, image.project_id, project_id
        ));
    }
    let project_reusable =
        is_project_reusable_video_material_image(&image.image_type, image.metadata_json.as_ref());
    if let Some(image_chapter_id) = image
        .chapter_id
        .filter(|id| *id != chapter_id && !project_reusable)
    {
        notes.push(format!(
            "参考图片 #{} 属于章节 #{}，不属于当前章节 #{}；真实生成会拒绝该输入",
            image.id, image_chapter_id, chapter_id
        ));
    }
    if image.file_path.trim().is_empty() {
        notes.push(format!(
            "参考图片 #{} 文件路径为空；预览使用占位媒体 URL，真实生成会拒绝该输入",
            image.id
        ));
    } else if let Err(err) = image_mime_from_path(Path::new(&image.file_path)) {
        notes.push(format!(
            "参考图片 #{} 文件类型无法作为视频图片输入：{err}；预览使用占位媒体 URL，真实生成会拒绝该输入",
            image.id
        ));
    }
    notes
}

fn reference_image_preview_placeholder(image: &generated_image::Model) -> String {
    let size_hint = image
        .file_size
        .filter(|size| *size > 0)
        .map(|size| format!("{size} bytes"))
        .unwrap_or_else(|| "size unknown".to_string());
    format!(
        "https://example.invalid/a4n-media/preview/generated-image-{}?size_hint={}&validation=deferred",
        image.id,
        encode_query_component(&size_hint)
    )
}

pub(super) fn reference_image_ids(value: Option<&Value>) -> Result<Vec<i64>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = value
        .as_array()
        .ok_or_else(|| "reference_image_ids 必须是数组".to_string())?;
    let mut ids = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let id = value
            .as_i64()
            .filter(|id| *id > 0)
            .ok_or_else(|| format!("reference_image_ids[{index}] 必须是正整数"))?;
        if ids.contains(&id) {
            return Err(format!("reference_image_ids 包含重复 ID：{id}"));
        }
        ids.push(id);
    }
    Ok(ids)
}

fn ensure_reference_image_ids_valid(ids: &[i64]) -> Result<(), String> {
    let mut seen = Vec::with_capacity(ids.len());
    for (index, id) in ids.iter().copied().enumerate() {
        if id <= 0 {
            return Err(format!("reference_image_ids[{index}] 必须是正整数"));
        }
        if seen.contains(&id) {
            return Err(format!("reference_image_ids 包含重复 ID：{id}"));
        }
        seen.push(id);
    }
    Ok(())
}

async fn image_file_to_provider_input(
    state: &AppState,
    user_id: i64,
    media_base_url: Option<&str>,
    image: &generated_image::Model,
    inline_total_bytes: &mut u64,
    transport: ProviderImageInputTransport,
) -> Result<String, String> {
    let canonical_path = canonicalize_data_file_path(
        &state.config.data_dir,
        &image.file_path,
        &format!("参考图片 #{}", image.id),
    )?;
    let metadata = std::fs::metadata(&canonical_path)
        .map_err(|e| format!("无法读取参考图片 #{} 元数据：{e}", image.id))?;
    if metadata.len() > MAX_REFERENCE_IMAGE_BYTES {
        return Err(format!(
            "参考图片 #{} 超过 {}MB，无法作为视频参考图",
            image.id,
            MAX_REFERENCE_IMAGE_BYTES / 1024 / 1024
        ));
    }
    let mime = image_mime_from_path(&canonical_path)?;
    match transport {
        ProviderImageInputTransport::PublicUrl => {
            let Some(base) = media_base_url else {
                return Err("public_url 传输缺少公网媒体入口".to_string());
            };
            let url = image_file_to_provider_media_url(state, user_id, base, image)?;
            validate_provider_public_image_url(&url, &format!("参考图片 #{}", image.id)).await?;
            Ok(url)
        }
        ProviderImageInputTransport::RemoteTempUpload => {
            image_file_to_remote_temp_media_url(&canonical_path, image, mime).await
        }
        ProviderImageInputTransport::InlineDataUrl => {
            let bytes = std::fs::read(&canonical_path)
                .map_err(|e| format!("无法读取参考图片 #{}：{e}", image.id))?;
            *inline_total_bytes = inline_total_bytes.saturating_add(metadata.len());
            if *inline_total_bytes > MAX_REFERENCE_IMAGE_TOTAL_BYTES {
                return Err(format!(
                    "参考图片总大小超过 {}MB，无法作为视频参考图",
                    MAX_REFERENCE_IMAGE_TOTAL_BYTES / 1024 / 1024
                ));
            }
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            Ok(format!("data:{mime};base64,{encoded}"))
        }
    }
}

async fn image_file_to_remote_temp_media_url(
    canonical_path: &Path,
    image: &generated_image::Model,
    mime: &str,
) -> Result<String, String> {
    let (upload_url, token) = remote_temp_upload_config()?;
    let bytes = std::fs::read(canonical_path)
        .map_err(|e| format!("无法读取参考图片 #{}：{e}", image.id))?;
    let file_name = canonical_path
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("video-input.png")
        .to_string();
    let payload = RemoteTempMediaUploadRequest {
        file_name,
        mime_type: mime.to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    };
    let response = reqwest::Client::new()
        .post(upload_url.trim())
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|err| format!("上传远端临时媒体失败：{err}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|err| format!("读取远端临时媒体响应失败：{err}"))?;
    if !status.is_success() {
        return Err(format!(
            "上传远端临时媒体失败：HTTP {}：{}",
            status.as_u16(),
            truncate_error_body(&text)
        ));
    }
    let parsed: RemoteTempMediaUploadResponse = serde_json::from_str(&text).map_err(|err| {
        format!(
            "解析远端临时媒体响应失败：{err}；响应：{}",
            truncate_error_body(&text)
        )
    })?;
    let url = parsed.url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("远端临时媒体响应 url 必须以 http:// 或 https:// 开头".to_string());
    }
    validate_provider_public_image_url(url, &format!("远端临时媒体参考图片 #{}", image.id)).await?;
    Ok(url.to_string())
}

async fn validate_provider_public_image_url(url: &str, label: &str) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(PROVIDER_IMAGE_URL_PROBE_TIMEOUT_SECS))
        .build()
        .map_err(|err| format!("创建公网图片探测客户端失败：{err}"))?;
    let head_probe = probe_provider_public_image_url(&client, url, label, false).await?;
    if head_probe.supported {
        return Ok(());
    }
    let get_probe = probe_provider_public_image_url(&client, url, label, true).await?;
    if get_probe.supported {
        return Ok(());
    }
    Err(format!(
        "{label} 的公网 URL 无法作为视频模型图片输入：{}。请检查 A4N_TEMP_MEDIA_PUBLIC_BASE_URL/A4N_PUBLIC_APP_URL/A4N_PUBLIC_MEDIA_BASE_URL 是否指向可公网访问的图片路由，且没有被 Cloudflare、DNSPod、Nginx 或登录页重定向到 HTML/JSON 页面。url={}",
        get_probe.message,
        truncate_url_for_error(url)
    ))
}

#[derive(Debug)]
struct ProviderImageUrlProbe {
    supported: bool,
    message: String,
}

async fn probe_provider_public_image_url(
    client: &reqwest::Client,
    url: &str,
    label: &str,
    use_get: bool,
) -> Result<ProviderImageUrlProbe, String> {
    let request = if use_get {
        client.get(url).header(RANGE, "bytes=0-0")
    } else {
        client.head(url)
    };
    let response = request
        .send()
        .await
        .map_err(|err| format!("{label} 公网 URL 探测失败：{err}"))?;
    let status = response.status();
    let final_url = response.url().to_string();
    if !status.is_success() {
        return Ok(ProviderImageUrlProbe {
            supported: false,
            message: format!(
                "{} 返回 HTTP {}，最终 URL={}",
                if use_get { "GET" } else { "HEAD" },
                status.as_u16(),
                truncate_url_for_error(&final_url)
            ),
        });
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if content_type
        .as_deref()
        .is_some_and(is_provider_supported_image_content_type)
    {
        return Ok(ProviderImageUrlProbe {
            supported: true,
            message: String::new(),
        });
    }
    let content_type = content_type.unwrap_or_else(|| "<missing>".to_string());
    Ok(ProviderImageUrlProbe {
        supported: false,
        message: format!(
            "{} 返回不支持的 Content-Type：{}，最终 URL={}",
            if use_get { "GET" } else { "HEAD" },
            content_type,
            truncate_url_for_error(&final_url)
        ),
    })
}

fn is_provider_supported_image_content_type(value: &str) -> bool {
    let content_type = value
        .split(';')
        .next()
        .unwrap_or(value)
        .trim()
        .to_ascii_lowercase();
    matches!(
        content_type.as_str(),
        "image/jpeg"
            | "image/jpg"
            | "image/png"
            | "image/webp"
            | "image/x-icon"
            | "image/vnd.microsoft.icon"
    )
}

fn truncate_error_body(value: &str) -> String {
    const MAX_LEN: usize = 600;
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let preview = chars.by_ref().take(MAX_LEN).collect::<String>();
    if chars.next().is_some() {
        format!("{preview}…")
    } else {
        preview
    }
}

fn truncate_url_for_error(value: &str) -> String {
    const MAX_LEN: usize = 240;
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let preview = chars.by_ref().take(MAX_LEN).collect::<String>();
    if chars.next().is_some() {
        format!("{preview}…")
    } else {
        preview
    }
}

fn image_file_to_provider_media_url(
    state: &AppState,
    user_id: i64,
    base: &str,
    image: &generated_image::Model,
) -> Result<String, String> {
    let media_path = image.file_path.trim();
    let secret = state
        .config
        .security
        .jwt_secret
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "A4N_JWT_SECRET 不能为空".to_string())?;
    let token = jwt::create_media_token(
        &user_id.to_string(),
        media_path,
        secret,
        media_token_ttl_minutes(),
    )
    .map_err(|err| err.to_string())?;
    Ok(format!(
        "{base}?path={}&media_token={}",
        encode_query_component(media_path),
        encode_query_component(&token)
    ))
}

fn media_token_ttl_minutes() -> u64 {
    std::env::var(MEDIA_TOKEN_TTL_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_MEDIA_TOKEN_TTL_MINUTES)
}

fn public_media_base_url() -> Result<String, String> {
    let Some(raw) = std::env::var("A4N_PUBLIC_MEDIA_BASE_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::env::var("A4N_PUBLIC_APP_URL")
                .ok()
                .map(|value| format!("{}/api/media", value.trim().trim_end_matches('/')))
        })
    else {
        return Err(
            "当前视频图片输入传输模式为 public_url，需要公网媒体入口：请设置 A4N_PUBLIC_APP_URL 或 A4N_PUBLIC_MEDIA_BASE_URL；本地生成通常应使用默认 auto/inline_data_url"
                .to_string(),
        );
    };
    let value = raw.trim().trim_end_matches('/').to_string();
    if !(value.starts_with("https://") || value.starts_with("http://")) {
        return Err("公网媒体入口必须以 http:// 或 https:// 开头".to_string());
    }
    if value.contains("://127.0.0.1")
        || value.contains("://localhost")
        || value.contains("://0.0.0.0")
    {
        return Err("公网媒体入口不能使用 localhost、127.0.0.1 或 0.0.0.0".to_string());
    }
    Ok(value)
}

fn encode_query_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn image_mime_from_path(path: &Path) -> Result<&'static str, String> {
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            format!(
                "参考图片缺少文件扩展名，无法判断 MIME 类型：{}",
                path.display()
            )
        })?;
    match extension.to_ascii_lowercase().as_str() {
        "png" => Ok("image/png"),
        "jpg" | "jpeg" => Ok("image/jpeg"),
        "webp" => Ok("image/webp"),
        _ => Err(format!("参考图片扩展名不支持作为视频参考图：{}", extension)),
    }
}

pub(crate) fn save_video_bytes(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
    bytes: &[u8],
    format: &str,
) -> Result<PathBuf, String> {
    let dir = ensure_chapter_videos_dir(state, project_id, chapter_id)?;
    let extension = required_video_file_extension(format)?;
    let path = dir.join(format!("video_{video_id}.{extension}"));
    std::fs::write(&path, bytes).map_err(|e| format!("无法写入视频：{e}"))?;
    Ok(path)
}

pub(crate) fn ensure_chapter_videos_dir(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
) -> Result<PathBuf, String> {
    let dir = chapter_videos_dir(&state.config.data_dir, project_id, chapter_id)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建视频目录 {dir:?}：{e}"))?;
    Ok(dir)
}

fn generate_video_thumbnail(
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
    video_path: &Path,
) -> Result<PathBuf, String> {
    let dir = video_path
        .parent()
        .ok_or_else(|| format!("视频路径缺少父目录：{}", video_path.display()))?;
    let output = dir.join(format!("thumb_{video_id}.jpg"));
    let result = Command::new("ffmpeg")
        .arg("-y")
        .arg("-ss")
        .arg("0.8")
        .arg("-i")
        .arg(video_path)
        .arg("-frames:v")
        .arg("1")
        .arg("-vf")
        .arg("scale=640:-2")
        .arg("-q:v")
        .arg("3")
        .arg("-update")
        .arg("1")
        .arg(&output)
        .output()
        .map_err(|err| format!("生成视频缩略图失败：未能启动 FFmpeg：{err}"))?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!(
            "生成视频缩略图失败 project={project_id} chapter={chapter_id} video={video_id}：{}",
            stderr.chars().take(500).collect::<String>()
        ));
    }
    Ok(output)
}

fn video_tail_frame_path(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
) -> Result<PathBuf, String> {
    let dir = chapter_tail_frame_images_dir(&state.config.data_dir, project_id, chapter_id)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建图片目录 {dir:?}：{e}"))?;
    Ok(dir.join(format!("video_tail_frame_{video_id}.jpg")))
}

fn capture_video_tail_frame(video_path: &Path, output: &Path) -> Result<(), String> {
    let result = Command::new("ffmpeg")
        .arg("-y")
        .arg("-sseof")
        .arg("-0.08")
        .arg("-i")
        .arg(video_path)
        .arg("-frames:v")
        .arg("1")
        .arg("-q:v")
        .arg("2")
        .arg("-update")
        .arg("1")
        .arg(output)
        .output()
        .map_err(|err| format!("采集视频尾帧失败：未能启动 FFmpeg：{err}"))?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!(
            "采集视频尾帧失败：{}",
            stderr.chars().take(500).collect::<String>()
        ));
    }
    Ok(())
}

pub(crate) async fn capture_video_tail_frame_asset(
    state: &AppState,
    video: &generated_video::Model,
    chapter_id: i64,
    prompt_id: Option<i64>,
    video_path: &Path,
) -> Result<Value, String> {
    let output = video_tail_frame_path(state, video.project_id, chapter_id, video.id)?;
    capture_video_tail_frame(video_path, &output)?;
    let output_path_str = data_dir_relative_file_path(&state.config.data_dir, &output, "尾帧图片")?;
    let file_size = std::fs::metadata(&output)
        .map_err(|err| format!("无法读取尾帧图片元数据：{err}"))?
        .len() as i64;
    let shot_number = video_shot_number(video)?;
    let source = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("source"))
        .cloned();
    let metadata = json!({
        "generation_context": {
            "source": "video_tail_frame_capture",
            "surface": "novel_detail.video",
            "frame_kind": "actual_tail_frame",
            "project_id": video.project_id,
            "chapter_id": chapter_id,
            "video_id": video.id,
            "prompt_id": prompt_id,
            "shot_number": shot_number,
            "source_content_sha256": video_source_content_hash(video.metadata_json.as_ref()),
        },
        "source": source,
    });
    let image = match state
        .image
        .create_record(CreateImageRecord {
            project_id: video.project_id,
            image_type: "video_tail_frame".to_string(),
            file_path: output_path_str,
            prompt: format!("视频 #{} 第 {} 段真实尾帧采集", video.id, shot_number),
            model: "ffmpeg".to_string(),
            width: video.width.max(1),
            height: video.height.max(1),
            file_size: Some(file_size),
            chapter_id: Some(chapter_id),
            panel_id: None,
            metadata_json: Some(metadata),
            created_at: None,
        })
        .await
    {
        Ok(image) => image,
        Err(err) => {
            return Err(append_secondary_error(
                format!("尾帧图片记录写入失败：{err}"),
                remove_partial_video_file(&output, "尾帧图片").err(),
            ));
        }
    };
    Ok(json!({
        "image_id": image.id,
        "file_path": image.file_path,
        "shot_number": shot_number,
        "source": "video_tail_frame_capture",
    }))
}

pub(crate) async fn delete_video_tail_frame_assets(
    state: &AppState,
    video_id: i64,
) -> Result<(), String> {
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ImageType.eq("video_tail_frame"))
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("查询视频真实尾帧图片失败：{err}"))?
        .into_iter()
        .filter(|image| tail_frame_image_belongs_to_video(image, video_id))
        .collect::<Vec<_>>();

    let mut error = None;
    for image in images {
        let path = image.file_path.trim();
        let image_path = if path.is_empty() {
            None
        } else {
            deletable_data_file_path(
                &state.config.data_dir,
                path,
                &format!("视频 #{} 真实尾帧图片 #{}", video_id, image.id),
            )?
        };
        match state.image.delete(image.id).await {
            Ok(()) => {
                if let Some(path) = image_path.as_ref() {
                    error = combine_optional_errors(
                        error,
                        remove_partial_video_file(
                            path,
                            &format!("视频 #{} 真实尾帧图片 #{}", video_id, image.id),
                        )
                        .err(),
                    );
                }
            }
            Err(err) => {
                error = combine_optional_errors(
                    error,
                    Some(format!(
                        "视频 #{} 真实尾帧图片 #{} 记录删除失败：{err}",
                        video_id, image.id
                    )),
                );
            }
        }
    }
    match error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn tail_frame_image_belongs_to_video(image: &generated_image::Model, video_id: i64) -> bool {
    let Some(context) = image
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("generation_context"))
    else {
        return false;
    };
    context
        .get("source")
        .and_then(Value::as_str)
        .is_some_and(|source| source.trim() == "video_tail_frame_capture")
        && context.get("video_id").and_then(Value::as_i64) == Some(video_id)
}

fn video_shot_number(video: &generated_video::Model) -> Result<i32, String> {
    video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("shot_number"))
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("视频 #{} 缺少有效 shot_number，无法采集真实尾帧", video.id))
}

fn video_source_content_hash(metadata: Option<&Value>) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("source"))
        .and_then(|source| source.get("content_sha256"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

pub(crate) fn best_effort_video_thumbnail_path(
    data_dir: &Path,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
    video_path: &Path,
) -> Option<String> {
    match generate_video_thumbnail(project_id, chapter_id, video_id, video_path) {
        Ok(path) => match data_dir_relative_file_path(data_dir, &path, "视频缩略图") {
            Ok(path) => Some(path),
            Err(err) => {
                tracing::warn!(
                    %err,
                    project_id,
                    chapter_id,
                    video_id,
                    "视频缩略图已生成但无法规范化为相对路径"
                );
                None
            }
        },
        Err(err) => {
            tracing::warn!(
                %err,
                project_id,
                chapter_id,
                video_id,
                "生成视频缩略图失败，继续保留视频结果"
            );
            None
        }
    }
}

pub(crate) fn required_video_file_extension(format: &str) -> Result<&'static str, String> {
    let format = format.trim();
    if format.is_empty() {
        return Err("视频 provider 未返回文件格式，无法保存视频".to_string());
    }
    match format.to_ascii_lowercase().as_str() {
        "mp4" => Ok("mp4"),
        "webm" => Ok("webm"),
        "mov" => Ok("mov"),
        "m4v" => Ok("m4v"),
        _ => Err(format!("视频 provider 返回了不支持的文件格式：{format}")),
    }
}

pub(crate) fn video_mime_from_path(path: &Path) -> Result<&'static str, String> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("mp4" | "m4v") => Ok("video/mp4"),
        Some("webm") => Ok("video/webm"),
        Some("mov") => Ok("video/quicktime"),
        Some(other) => Err(format!("视频文件扩展名不支持播放：{other}")),
        None => Err("视频文件缺少扩展名，无法确定 MIME 类型".to_string()),
    }
}

pub(crate) fn remove_partial_video_file(path: &Path, label: &str) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("{label}删除失败：{err}")),
    }
}

pub(crate) fn append_secondary_error(mut message: String, secondary: Option<String>) -> String {
    if let Some(secondary) = secondary {
        message.push('；');
        message.push_str(&secondary);
    }
    message
}

pub(crate) fn combine_optional_errors(
    primary: Option<String>,
    secondary: Option<String>,
) -> Option<String> {
    match primary {
        Some(primary) => Some(append_secondary_error(primary, secondary)),
        None => secondary,
    }
}

pub(crate) fn prompt_id_from_video(video: &generated_video::Model) -> Option<i64> {
    video
        .metadata_json
        .as_ref()
        .and_then(|m| m.get("prompt_id"))
        .and_then(|v| v.as_i64())
}

pub(crate) fn ensure_ffmpeg() -> Result<(), String> {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map_err(|_| "未检测到 FFmpeg，无法裁剪视频".to_string())
        .and_then(|out| {
            if out.status.success() {
                Ok(())
            } else {
                Err("FFmpeg 不可用，无法裁剪视频".to_string())
            }
        })
}

pub(crate) fn trimmed_video_path(
    state: &AppState,
    source: &generated_video::Model,
    start_secs: f32,
    end_secs: f32,
) -> Result<PathBuf, String> {
    let chapter_id = required_trim_chapter_id(source)?;
    let dir = ensure_chapter_videos_dir(state, source.project_id, chapter_id)?;
    Ok(dir.join(format!(
        "trim_{}_{}_{}.mp4",
        source.id,
        millis_label(start_secs),
        millis_label(end_secs)
    )))
}

fn millis_label(v: f32) -> i32 {
    (v * 1000.0).round() as i32
}

fn chapter_videos_dir(
    data_dir: &Path,
    project_id: i64,
    chapter_id: i64,
) -> Result<PathBuf, String> {
    required_asset_chapter_id(chapter_id)?;
    Ok(ProjectPaths::new(data_dir, project_id).chapter_videos_dir(chapter_id))
}

fn chapter_tail_frame_images_dir(
    data_dir: &Path,
    project_id: i64,
    chapter_id: i64,
) -> Result<PathBuf, String> {
    required_asset_chapter_id(chapter_id)?;
    Ok(ProjectPaths::new(data_dir, project_id).chapter_images_dir(chapter_id))
}

fn required_asset_chapter_id(chapter_id: i64) -> Result<(), String> {
    if chapter_id > 0 {
        Ok(())
    } else {
        Err(format!(
            "章节资产目录需要有效 chapter_id，收到 {chapter_id}"
        ))
    }
}

pub(crate) fn run_ffmpeg_trim(
    input: &Path,
    output: &Path,
    start_secs: f32,
    end_secs: f32,
) -> Result<(), String> {
    let start = format!("{start_secs:.3}");
    let end = format!("{end_secs:.3}");
    let copy = Command::new("ffmpeg")
        .args(["-y", "-ss", &start, "-to", &end, "-i"])
        .arg(input)
        .args(["-c", "copy"])
        .arg(output)
        .output()
        .map_err(|e| format!("启动 FFmpeg 失败：{e}"))?;
    if copy.status.success() {
        return Ok(());
    }

    let recode = Command::new("ffmpeg")
        .args(["-y", "-ss", &start, "-to", &end, "-i"])
        .arg(input)
        .args(["-c:v", "libx264", "-c:a", "aac"])
        .arg(output)
        .output()
        .map_err(|e| format!("启动 FFmpeg 重编码失败：{e}"))?;
    if recode.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&recode.stderr);
        Err(format!(
            "FFmpeg 裁剪失败：{}",
            stderr.chars().take(500).collect::<String>()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::{Value, json};

    fn video_with_chapter(chapter_id: Option<i64>) -> generated_video::Model {
        generated_video::Model {
            id: 42,
            project_id: 1,
            chapter_id,
            video_type: "chapter_clip".to_string(),
            workflow_mode: None,
            file_path: None,
            thumbnail_path: None,
            prompt: "prompt".to_string(),
            negative_prompt: None,
            model: "model".to_string(),
            width: 1,
            height: 1,
            duration_secs: 1.0,
            fps: 24,
            file_size: None,
            source_video_id: None,
            source_image_id: None,
            status: "completed".to_string(),
            provider_job_id: None,
            error_message: None,
            metadata_json: None,
            is_favorite: false,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    fn video_with_shot_number(shot_number: Option<Value>) -> generated_video::Model {
        let mut video = video_with_chapter(Some(2));
        video.video_type = "chapter_video".to_string();
        video.workflow_mode = Some("first_frame_chain".to_string());
        video.width = 1280;
        video.height = 720;
        video.duration_secs = 8.0;
        video.metadata_json = shot_number.map(|shot_number| json!({ "shot_number": shot_number }));
        video
    }

    fn image_for_preview(
        project_id: i64,
        chapter_id: Option<i64>,
        file_path: &str,
        file_size: Option<i64>,
    ) -> generated_image::Model {
        generated_image::Model {
            id: 7,
            project_id,
            image_type: "illustration".to_string(),
            file_path: file_path.to_string(),
            prompt: "prompt".to_string(),
            model: "model".to_string(),
            width: 1280,
            height: 720,
            file_size,
            ai_score: None,
            is_favorite: false,
            chapter_id,
            panel_id: None,
            metadata_json: None,
            created_at: Utc::now().naive_utc(),
        }
    }

    #[test]
    fn tail_frame_image_binds_to_capture_video_id() {
        let mut image = image_for_preview(1, Some(2), "images/tail.jpg", Some(100));
        image.image_type = "video_tail_frame".to_string();
        image.metadata_json = Some(json!({
            "generation_context": {
                "source": "video_tail_frame_capture",
                "video_id": 42,
            }
        }));

        assert!(tail_frame_image_belongs_to_video(&image, 42));
        assert!(!tail_frame_image_belongs_to_video(&image, 41));

        image.metadata_json = Some(json!({
            "generation_context": {
                "source": "video_boundary_frame_prompt",
                "video_id": 42,
            }
        }));
        assert!(!tail_frame_image_belongs_to_video(&image, 42));
    }

    #[test]
    fn reference_image_ids_accepts_missing_and_integer_array() {
        assert_eq!(reference_image_ids(None), Ok(Vec::new()));
        assert_eq!(
            reference_image_ids(Some(&serde_json::json!([3, 5]))),
            Ok(vec![3, 5])
        );
    }

    #[test]
    fn reference_image_ids_rejects_invalid_or_duplicate_values() {
        for value in [
            serde_json::json!("3"),
            serde_json::json!(["3"]),
            serde_json::json!([0]),
            serde_json::json!([3, 3]),
        ] {
            reference_image_ids(Some(&value))
                .expect_err("invalid reference image ids should be rejected");
        }
    }

    #[test]
    fn reference_image_project_ids_preserve_reference_order_and_deduplicate_projects() {
        let mut first = image_for_preview(20, Some(1), "first.png", Some(10));
        first.id = 11;
        let mut second = image_for_preview(10, Some(1), "second.png", Some(10));
        second.id = 12;
        let mut third_same_project = image_for_preview(20, Some(1), "third.png", Some(10));
        third_same_project.id = 13;
        let images_by_id = HashMap::from([
            (first.id, first),
            (second.id, second),
            (third_same_project.id, third_same_project),
        ]);

        let project_ids =
            reference_image_project_ids_in_reference_order(&[11, 404, 12, 13], &images_by_id);

        assert_eq!(project_ids, vec![20, 10]);
    }

    #[test]
    fn image_mime_from_path_requires_supported_extension() {
        for path in ["image", "image.bmp", "image.gif"] {
            let err = image_mime_from_path(std::path::Path::new(path))
                .expect_err("unsupported image extension should be rejected");

            assert!(err.contains("参考图片"));
        }
    }

    #[test]
    fn image_mime_from_path_accepts_supported_extensions() {
        assert_eq!(
            image_mime_from_path(std::path::Path::new("image.PNG")),
            Ok("image/png")
        );
        assert_eq!(
            image_mime_from_path(std::path::Path::new("image.jpeg")),
            Ok("image/jpeg")
        );
        assert_eq!(
            image_mime_from_path(std::path::Path::new("image.webp")),
            Ok("image/webp")
        );
    }

    #[test]
    fn provider_image_content_type_accepts_provider_supported_images() {
        for content_type in [
            "image/png",
            "image/jpeg",
            "image/jpg",
            "image/webp",
            "image/x-icon",
            "image/vnd.microsoft.icon",
            "image/png; charset=binary",
            " IMAGE/WEBP ",
        ] {
            assert!(
                is_provider_supported_image_content_type(content_type),
                "{content_type} should be accepted"
            );
        }
    }

    #[test]
    fn provider_image_content_type_rejects_non_images() {
        for content_type in [
            "",
            "application/json",
            "application/json; charset=utf-8",
            "text/html",
            "application/octet-stream",
        ] {
            assert!(
                !is_provider_supported_image_content_type(content_type),
                "{content_type} should be rejected"
            );
        }
    }

    #[test]
    fn preview_placeholder_records_invalid_reference_notes_without_blocking_url() {
        let image = image_for_preview(2, Some(9), "bad.bmp", None);

        let notes = reference_image_preview_notes(1, 9, &[1], &image);
        let placeholder = reference_image_preview_placeholder(&image);

        assert!(notes.iter().any(|note| note.contains("不属于当前视频项目")));
        assert!(
            notes
                .iter()
                .any(|note| note.contains("文件类型无法作为视频图片输入"))
        );
        assert!(
            placeholder.starts_with("https://example.invalid/a4n-media/preview/generated-image-7")
        );
        assert!(placeholder.contains("validation=deferred"));
    }

    #[test]
    fn preview_placeholder_uses_image_metadata_without_reading_file() {
        let image = image_for_preview(1, Some(9), "still.jpeg", Some(1234));

        let notes = reference_image_preview_notes(1, 9, &[1], &image);
        let placeholder = reference_image_preview_placeholder(&image);

        assert!(notes.is_empty());
        assert!(
            placeholder.starts_with("https://example.invalid/a4n-media/preview/generated-image-7")
        );
        assert!(placeholder.contains("1234%20bytes"));
    }

    #[test]
    fn required_video_file_extension_rejects_missing_or_unknown_format() {
        for format in ["", "   ", "avi"] {
            let err = required_video_file_extension(format)
                .expect_err("unsupported video format should be rejected");

            assert!(err.contains("文件格式"));
        }
    }

    #[test]
    fn required_video_file_extension_accepts_supported_formats() {
        assert_eq!(required_video_file_extension("mp4"), Ok("mp4"));
        assert_eq!(required_video_file_extension(" WEBM "), Ok("webm"));
        assert_eq!(required_video_file_extension("mov"), Ok("mov"));
        assert_eq!(required_video_file_extension("m4v"), Ok("m4v"));
    }

    #[test]
    fn required_trim_chapter_id_rejects_missing_or_invalid_id() {
        for id in [None, Some(0), Some(-1)] {
            let video = video_with_chapter(id);
            let err =
                required_trim_chapter_id(&video).expect_err("trim chapter id should be required");

            assert!(err.contains("源视频 #42 缺少 chapter_id"));
        }
    }

    #[test]
    fn required_trim_chapter_id_accepts_positive_id() {
        let video = video_with_chapter(Some(7));

        assert_eq!(required_trim_chapter_id(&video), Ok(7));
    }

    #[test]
    fn chapter_videos_dir_scopes_assets_by_chapter() {
        let dir = chapter_videos_dir(Path::new("/tmp/a4n"), 42, 7)
            .expect("chapter video dir should resolve");

        assert_eq!(dir, Path::new("/tmp/a4n/projects/42/videos/chapter_7"));
    }

    #[test]
    fn chapter_tail_frame_images_dir_scopes_assets_by_chapter() {
        let dir = chapter_tail_frame_images_dir(Path::new("/tmp/a4n"), 42, 7)
            .expect("chapter tail frame image dir should resolve");

        assert_eq!(dir, Path::new("/tmp/a4n/projects/42/images/chapter_7"));
    }

    #[test]
    fn chapter_asset_dirs_reject_invalid_chapter_id() {
        let err = chapter_videos_dir(Path::new("/tmp/a4n"), 42, 0)
            .expect_err("invalid chapter id should be rejected");

        assert!(err.contains("有效 chapter_id"));
    }

    #[test]
    fn media_url_query_component_encodes_path_separators_and_spaces() {
        assert_eq!(
            encode_query_component("projects/1/images/chapter 1/frame.png"),
            "projects%2F1%2Fimages%2Fchapter%201%2Fframe.png"
        );
    }

    #[test]
    fn video_shot_number_requires_explicit_positive_metadata() {
        assert_eq!(
            video_shot_number(&video_with_shot_number(Some(json!(3)))).unwrap(),
            3
        );

        let missing = video_shot_number(&video_with_shot_number(None))
            .expect_err("missing shot number must not fall back to first shot");
        assert!(missing.contains("缺少有效 shot_number"));

        let zero = video_shot_number(&video_with_shot_number(Some(json!(0))))
            .expect_err("zero shot number must be rejected");
        assert!(zero.contains("缺少有效 shot_number"));

        let string_value = video_shot_number(&video_with_shot_number(Some(json!("3"))))
            .expect_err("string shot number must be rejected");
        assert!(string_value.contains("缺少有效 shot_number"));
    }
}
