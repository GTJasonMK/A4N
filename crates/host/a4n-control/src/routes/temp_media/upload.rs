use axum::Json;
use axum::extract::{Multipart, State};
use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use a4n_storage::write_new_data_file;

use super::super::error::ApiError;
use super::config::{temp_media_max_bytes, temp_media_public_base_url, temp_media_ttl_seconds};
use super::rules::{
    normalize_temp_media_type, validate_media_magic, validate_upload_file_name_hint,
};
use super::storage::{TEMP_MEDIA_RELATIVE_DIR, cleanup_expired_temp_media};
use crate::state::ServerState;
use a4n_api_contract::temp_media::{
    TEMP_MEDIA_FILE_PART, TEMP_MEDIA_METADATA_PART, TempMediaUploadRequest, TempMediaUploadResponse,
};

pub(in crate::routes) async fn upload(
    State(state): State<ServerState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<TempMediaUploadResponse>, ApiError> {
    super::authorize(&headers)?;
    cleanup_expired_temp_media(&state);
    let metadata = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::bad_request(format!("读取临时媒体 metadata 失败：{error}")))?
        .ok_or_else(|| ApiError::bad_request("临时媒体缺少 metadata"))?;
    if metadata.name() != Some(TEMP_MEDIA_METADATA_PART) || metadata.file_name().is_some() {
        return Err(ApiError::bad_request("临时媒体第一部分必须是 metadata"));
    }
    let metadata = read_part(metadata, 16 * 1024, TEMP_MEDIA_METADATA_PART).await?;
    let input: TempMediaUploadRequest = serde_json::from_slice(&metadata)
        .map_err(|error| ApiError::bad_request(format!("临时媒体 metadata 无效：{error}")))?;

    for (label, value) in [
        ("owner_user_id", input.owner_user_id),
        ("project_id", input.project_id),
        ("source_media_id", input.source_media_id),
    ] {
        if value <= 0 {
            return Err(ApiError::bad_request(format!(
                "临时媒体 {label} 必须是正整数"
            )));
        }
    }
    if input.source_media_kind.trim().is_empty() {
        return Err(ApiError::bad_request("临时媒体 source_media_kind 不能为空"));
    }

    let (mime_type, extension) =
        normalize_temp_media_type(&input.mime_type).map_err(ApiError::bad_request)?;
    validate_upload_file_name_hint(&input.file_name, extension).map_err(ApiError::bad_request)?;
    let max_bytes = temp_media_max_bytes().map_err(ApiError::bad_request)?;
    let file = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::bad_request(format!("读取临时媒体文件失败：{error}")))?
        .ok_or_else(|| ApiError::bad_request("临时媒体缺少 file"))?;
    if file.name() != Some(TEMP_MEDIA_FILE_PART) {
        return Err(ApiError::bad_request("临时媒体第二部分必须是 file"));
    }
    let file_mime = file
        .content_type()
        .ok_or_else(|| ApiError::bad_request("临时媒体 file 缺少 Content-Type"))?;
    if normalize_temp_media_type(file_mime)
        .map_err(ApiError::bad_request)?
        .0
        != mime_type
    {
        return Err(ApiError::bad_request(
            "临时媒体 file 与 metadata MIME 不一致",
        ));
    }
    let bytes = read_part(file, max_bytes, TEMP_MEDIA_FILE_PART).await?;
    if multipart
        .next_field()
        .await
        .map_err(|error| ApiError::bad_request(format!("读取临时媒体上传尾部失败：{error}")))?
        .is_some()
    {
        return Err(ApiError::bad_request(
            "临时媒体只允许一个 metadata 和一个 file",
        ));
    }
    validate_media_magic(mime_type, &bytes).map_err(ApiError::bad_request)?;
    let ttl_seconds = temp_media_ttl_seconds().map_err(ApiError::bad_request)?;
    let public_base_url = temp_media_public_base_url()?;
    let byte_size = u64::try_from(bytes.len())
        .map_err(|_| ApiError::bad_request("临时媒体大小超出 u64 范围"))?;
    let sha256 = format!("{:x}", Sha256::digest(&bytes));

    let artifact_id = Uuid::new_v4().simple().to_string();
    let file_name = format!("{artifact_id}.{extension}");
    let relative_path = format!("{TEMP_MEDIA_RELATIVE_DIR}/{file_name}");
    write_new_data_file(
        state.runtime().data_dir(),
        &relative_path,
        &bytes,
        "临时媒体",
    )
    .map_err(ApiError::bad_request)?;

    let now = Utc::now();
    let expires_at = now + Duration::seconds(ttl_seconds);
    tracing::info!(
        artifact_id = %artifact_id,
        owner_user_id = input.owner_user_id,
        project_id = input.project_id,
        source_media_kind = input.source_media_kind.as_str(),
        source_media_id = input.source_media_id,
        content_type = mime_type,
        byte_size,
        expires_at = %expires_at,
        "临时媒体 artifact 创建完成"
    );
    Ok(Json(TempMediaUploadResponse {
        artifact_id,
        url: format!("{public_base_url}/{file_name}"),
        expires_at: expires_at.to_rfc3339(),
        owner_user_id: input.owner_user_id,
        project_id: input.project_id,
        source_media_kind: input.source_media_kind,
        source_media_id: input.source_media_id,
        content_type: mime_type.to_string(),
        sha256,
        byte_size,
    }))
}

async fn read_part(
    mut field: axum::extract::multipart::Field<'_>,
    max_bytes: usize,
    label: &str,
) -> Result<Vec<u8>, ApiError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|error| ApiError::bad_request(format!("读取临时媒体 {label} 失败：{error}")))?
    {
        if bytes
            .len()
            .checked_add(chunk.len())
            .is_none_or(|size| size > max_bytes)
        {
            return Err(ApiError::bad_request(format!(
                "临时媒体 {label} 超过大小限制"
            )));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
