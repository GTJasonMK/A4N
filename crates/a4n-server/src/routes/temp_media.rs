use super::error::ApiError;
use crate::state::ServerState;
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use base64::Engine as _;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::path::{Path as FsPath, PathBuf};
use uuid::Uuid;

const TEMP_MEDIA_DIR: &str = "temp-media/video-inputs";
const TEMP_MEDIA_UPLOAD_TOKEN_ENV: &str = "A4N_TEMP_MEDIA_UPLOAD_TOKEN";
const TEMP_MEDIA_PUBLIC_BASE_URL_ENV: &str = "A4N_TEMP_MEDIA_PUBLIC_BASE_URL";
const PUBLIC_APP_URL_ENV: &str = "A4N_PUBLIC_APP_URL";
const TEMP_MEDIA_TTL_ENV: &str = "A4N_TEMP_MEDIA_TTL_MINUTES";
const TEMP_MEDIA_MAX_BYTES_ENV: &str = "A4N_TEMP_MEDIA_MAX_BYTES";
const DEFAULT_TEMP_MEDIA_TTL_MINUTES: i64 = 360;
const DEFAULT_TEMP_MEDIA_MAX_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub(super) struct TempMediaUploadRequest {
    file_name: String,
    mime_type: String,
    data_base64: String,
}

#[derive(Debug, Serialize)]
pub(super) struct TempMediaUploadResponse {
    url: String,
    expires_at: DateTime<Utc>,
    size_bytes: usize,
}

pub(super) async fn upload(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<TempMediaUploadRequest>,
) -> Result<Json<TempMediaUploadResponse>, ApiError> {
    authorize_temp_media_upload(&headers)?;
    cleanup_expired_temp_media(&state);

    let mime_type = normalize_image_mime(&input.mime_type).map_err(ApiError::bad_request)?;
    let extension = extension_for_mime(mime_type);
    validate_upload_file_name_hint(&input.file_name, extension).map_err(ApiError::bad_request)?;
    let bytes = decode_base64_image(&input.data_base64).map_err(ApiError::bad_request)?;
    let max_bytes = temp_media_max_bytes();
    if bytes.len() > max_bytes {
        return Err(ApiError::bad_request(format!(
            "临时媒体超过大小限制：{} bytes > {} bytes",
            bytes.len(),
            max_bytes
        )));
    }
    validate_image_magic(mime_type, &bytes).map_err(ApiError::bad_request)?;

    let dir = temp_media_dir(&state);
    std::fs::create_dir_all(&dir)
        .map_err(|err| ApiError::bad_request(format!("无法创建临时媒体目录：{err}")))?;
    let file_name = format!("{}.{}", Uuid::new_v4().simple(), extension);
    let path = dir.join(&file_name);
    std::fs::write(&path, &bytes)
        .map_err(|err| ApiError::bad_request(format!("无法写入临时媒体：{err}")))?;

    let now = Utc::now();
    Ok(Json(TempMediaUploadResponse {
        url: format!("{}/{}", temp_media_public_base_url()?, file_name),
        expires_at: now + Duration::minutes(temp_media_ttl_minutes()),
        size_bytes: bytes.len(),
    }))
}

pub(super) async fn serve(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(file_name): Path<String>,
) -> Result<Response, ApiError> {
    let file_name = validate_temp_media_file_name(&file_name).map_err(ApiError::bad_request)?;
    let path = temp_media_dir(&state).join(file_name);
    if is_expired_file(&path).map_err(ApiError::bad_request)? {
        let _ = std::fs::remove_file(&path);
        return Err(ApiError::not_found("临时媒体不存在或已过期"));
    }
    if !path.is_file() {
        return Err(ApiError::not_found("临时媒体不存在或已过期"));
    }
    file_response(&path, headers.get(header::RANGE))
}

fn authorize_temp_media_upload(headers: &HeaderMap) -> Result<(), ApiError> {
    let expected = std::env::var(TEMP_MEDIA_UPLOAD_TOKEN_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::bad_request("A4N_TEMP_MEDIA_UPLOAD_TOKEN 不能为空"))?;
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::unauthorized("缺少临时媒体上传令牌"))?;
    if token != expected {
        return Err(ApiError::forbidden("临时媒体上传令牌无效"));
    }
    Ok(())
}

fn decode_base64_image(value: &str) -> Result<Vec<u8>, String> {
    let trimmed = value.trim();
    let payload = trimmed
        .strip_prefix("data:")
        .and_then(|data_uri| data_uri.split_once(',').map(|(_, payload)| payload))
        .unwrap_or(trimmed);
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|err| format!("临时媒体 data_base64 无效：{err}"))
}

fn normalize_image_mime(value: &str) -> Result<&'static str, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "image/png" => Ok("image/png"),
        "image/jpeg" | "image/jpg" => Ok("image/jpeg"),
        "image/webp" => Ok("image/webp"),
        other => Err(format!("临时媒体 MIME 不支持：{other}")),
    }
}

fn extension_for_mime(mime_type: &str) -> &'static str {
    match mime_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        _ => "bin",
    }
}

fn validate_upload_file_name_hint(file_name: &str, expected_extension: &str) -> Result<(), String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("临时媒体 file_name 不能包含路径分隔符".to_string());
    }
    let path = FsPath::new(trimmed);
    if path.components().count() != 1 {
        return Err("临时媒体 file_name 不能包含路径分隔符".to_string());
    }
    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase);
    if let Some(extension) = extension
        && extension != expected_extension
        && !(expected_extension == "jpg" && extension == "jpeg")
    {
        return Err(format!(
            "临时媒体 file_name 扩展名与 MIME 不一致：期望 {expected_extension}，实际 {extension}"
        ));
    }
    Ok(())
}

fn validate_image_magic(mime_type: &str, bytes: &[u8]) -> Result<(), String> {
    let ok = match mime_type {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
        "image/webp" => bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP",
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err("临时媒体内容与 MIME 类型不一致".to_string())
    }
}

fn validate_temp_media_file_name(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let Some((uuid, extension)) = trimmed.rsplit_once('.') else {
        return Err("临时媒体文件名无效".to_string());
    };
    if uuid.len() != 32 || !uuid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("临时媒体文件名无效".to_string());
    }
    match extension.to_ascii_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "webp" => Ok(trimmed.to_string()),
        _ => Err("临时媒体文件扩展名不支持".to_string()),
    }
}

fn temp_media_dir(state: &ServerState) -> PathBuf {
    state.config.data_dir.join(TEMP_MEDIA_DIR)
}

fn temp_media_public_base_url() -> Result<String, ApiError> {
    let raw = std::env::var(TEMP_MEDIA_PUBLIC_BASE_URL_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::env::var(PUBLIC_APP_URL_ENV)
                .ok()
                .map(|value| format!("{}/api/temp-media", value.trim().trim_end_matches('/')))
        })
        .ok_or_else(|| {
            ApiError::bad_request("A4N_TEMP_MEDIA_PUBLIC_BASE_URL 或 A4N_PUBLIC_APP_URL 不能为空")
        })?;
    let value = raw.trim().trim_end_matches('/').to_string();
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        return Err(ApiError::bad_request(
            "临时媒体公网地址必须以 http:// 或 https:// 开头",
        ));
    }
    Ok(value)
}

fn temp_media_ttl_minutes() -> i64 {
    std::env::var(TEMP_MEDIA_TTL_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_TEMP_MEDIA_TTL_MINUTES)
}

fn temp_media_max_bytes() -> usize {
    std::env::var(TEMP_MEDIA_MAX_BYTES_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_TEMP_MEDIA_MAX_BYTES)
}

fn is_expired_file(path: &FsPath) -> Result<bool, String> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(err) => return Err(format!("无法读取临时媒体信息：{err}")),
    };
    let modified = metadata
        .modified()
        .map_err(|err| format!("无法读取临时媒体修改时间：{err}"))?;
    let modified: DateTime<Utc> = modified.into();
    Ok(Utc::now() > modified + Duration::minutes(temp_media_ttl_minutes()))
}

fn cleanup_expired_temp_media(state: &ServerState) {
    let dir = temp_media_dir(state);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && matches!(is_expired_file(&path), Ok(true)) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn file_response(
    path: &FsPath,
    range_header: Option<&header::HeaderValue>,
) -> Result<Response, ApiError> {
    let media = a4n_runtime::media_file::read_media_file_response(
        path,
        range_header.map(header::HeaderValue::to_str),
        "临时媒体",
    )
    .map_err(ApiError::bad_request)?;
    let mut response = Response::builder()
        .status(
            StatusCode::from_u16(media.status.as_u16())
                .map_err(|err| ApiError::bad_request(err.to_string()))?,
        )
        .header(header::CONTENT_TYPE, media.content_type)
        .header(header::ACCEPT_RANGES, "bytes");
    if let Some(content_length) = media.content_length {
        response = response.header(header::CONTENT_LENGTH, content_length.to_string());
    }
    if let Some(content_range) = media.content_range {
        response = response.header(header::CONTENT_RANGE, content_range);
    }
    response
        .body(Body::from(media.body))
        .map_err(|err| ApiError::bad_request(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_temp_media_file_name_accepts_uuid_images() {
        let name = "0123456789abcdef0123456789abcdef.png";
        assert_eq!(validate_temp_media_file_name(name), Ok(name.to_string()));
    }

    #[test]
    fn validate_temp_media_file_name_rejects_paths_and_non_uuid_names() {
        for value in [
            "../x.png",
            "nested/x.png",
            "not-a-uuid.png",
            "0123456789abcdef0123456789abcdef.exe",
        ] {
            assert!(
                validate_temp_media_file_name(value).is_err(),
                "{value} should be rejected"
            );
        }
    }

    #[test]
    fn validate_image_magic_rejects_mime_mismatch() {
        assert!(validate_image_magic("image/png", b"\x89PNG\r\n\x1a\nabc").is_ok());
        assert!(validate_image_magic("image/jpeg", b"\xFF\xD8\xFFabc").is_ok());
        assert!(validate_image_magic("image/webp", b"RIFFxxxxWEBPabc").is_ok());
        assert!(validate_image_magic("image/png", b"\xFF\xD8\xFFabc").is_err());
    }

    #[test]
    fn validate_upload_file_name_hint_rejects_paths_and_mime_mismatch() {
        assert!(validate_upload_file_name_hint("local.png", "png").is_ok());
        assert!(validate_upload_file_name_hint("local.jpeg", "jpg").is_ok());
        assert!(validate_upload_file_name_hint("../local.png", "png").is_err());
        assert!(validate_upload_file_name_hint("nested/local.png", "png").is_err());
        assert!(validate_upload_file_name_hint("nested\\local.png", "png").is_err());
        assert!(validate_upload_file_name_hint("local.webp", "png").is_err());
    }
}
