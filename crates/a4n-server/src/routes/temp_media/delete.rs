use a4n_api_contract::temp_media::TempMediaDeleteRequest;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use sha2::{Digest, Sha256};

use super::super::error::ApiError;
use super::rules::validate_temp_media_artifact_id;
use super::storage::find_temp_media_path;
use crate::state::ServerState;

pub(in crate::routes) async fn delete(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(artifact_id): Path<String>,
    Json(input): Json<TempMediaDeleteRequest>,
) -> Result<StatusCode, ApiError> {
    super::authorize(&headers)?;
    let artifact_id =
        validate_temp_media_artifact_id(&artifact_id).map_err(ApiError::bad_request)?;
    let expected_sha256 = input.sha256.trim().to_ascii_lowercase();
    if expected_sha256.len() != 64 || !expected_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(ApiError::bad_request("临时媒体删除 sha256 无效"));
    }
    let Some(path) = find_temp_media_path(&state, &artifact_id) else {
        return Err(ApiError::not_found("临时媒体不存在或已删除"));
    };
    let bytes = std::fs::read(&path)
        .map_err(|error| ApiError::bad_request(format!("读取临时媒体失败：{error}")))?;
    let actual_sha256 = format!("{:x}", Sha256::digest(&bytes));
    if actual_sha256 != expected_sha256 {
        return Err(ApiError::bad_request("临时媒体内容 hash 与删除请求不一致"));
    }
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(ApiError::not_found("临时媒体不存在或已删除"))
        }
        Err(error) => Err(ApiError::bad_request(format!("删除临时媒体失败：{error}"))),
    }
}
