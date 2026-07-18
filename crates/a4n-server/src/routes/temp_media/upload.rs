use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, header};
use chrono::{Duration, Utc};
use uuid::Uuid;

use super::super::error::ApiError;
use super::config::{
    temp_media_max_bytes, temp_media_public_base_url, temp_media_ttl_minutes,
    temp_media_upload_token,
};
use super::contracts::{TempMediaUploadRequest, TempMediaUploadResponse};
use super::rules::{
    decode_base64_image, extension_for_mime, normalize_image_mime, validate_image_magic,
    validate_upload_file_name_hint,
};
use super::storage::{cleanup_expired_temp_media, temp_media_dir};
use crate::state::ServerState;

pub(in crate::routes) async fn upload(
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
    let max_bytes = temp_media_max_bytes().map_err(ApiError::bad_request)?;
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
    let ttl_minutes = temp_media_ttl_minutes().map_err(ApiError::bad_request)?;
    Ok(Json(TempMediaUploadResponse {
        url: format!("{}/{}", temp_media_public_base_url()?, file_name),
        expires_at: now + Duration::minutes(ttl_minutes),
        size_bytes: bytes.len(),
    }))
}

fn authorize_temp_media_upload(headers: &HeaderMap) -> Result<(), ApiError> {
    let expected = temp_media_upload_token()?;
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
