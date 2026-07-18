use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use std::path::Path as FsPath;

use super::super::error::ApiError;
use super::rules::validate_temp_media_file_name;
use super::storage::{is_expired_file, temp_media_dir};
use crate::state::ServerState;

pub(in crate::routes) async fn serve(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(file_name): Path<String>,
) -> Result<Response, ApiError> {
    let file_name = validate_temp_media_file_name(&file_name).map_err(ApiError::bad_request)?;
    let path = temp_media_dir(&state).join(file_name);
    if is_expired_file(&path).map_err(ApiError::bad_request)? {
        std::fs::remove_file(&path).map_err(|error| {
            ApiError::bad_request(format!(
                "临时媒体已过期，但删除失败：{}：{error}",
                path.display()
            ))
        })?;
        return Err(ApiError::not_found("临时媒体不存在或已过期"));
    }
    if !path.is_file() {
        return Err(ApiError::not_found("临时媒体不存在或已过期"));
    }
    file_response(&path, headers.get(header::RANGE))
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
