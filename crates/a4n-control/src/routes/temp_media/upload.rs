use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use a4n_storage::write_new_data_file;

use super::super::error::ApiError;
use super::config::{temp_media_max_bytes, temp_media_public_base_url, temp_media_ttl_seconds};
use super::rules::{
    decode_base64_media, normalize_temp_media_type, validate_media_magic,
    validate_upload_file_name_hint,
};
use super::storage::{TEMP_MEDIA_RELATIVE_DIR, cleanup_expired_temp_media};
use crate::state::ServerState;
use a4n_api_contract::temp_media::{TempMediaUploadRequest, TempMediaUploadResponse};

pub(in crate::routes) async fn upload(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<TempMediaUploadRequest>,
) -> Result<Json<TempMediaUploadResponse>, ApiError> {
    super::authorize(&headers)?;
    cleanup_expired_temp_media(&state);

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
    let bytes = decode_base64_media(&input.data_base64).map_err(ApiError::bad_request)?;
    let max_bytes = temp_media_max_bytes().map_err(ApiError::bad_request)?;
    if bytes.len() > max_bytes {
        return Err(ApiError::bad_request(format!(
            "临时媒体超过大小限制：{} bytes > {} bytes",
            bytes.len(),
            max_bytes
        )));
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
