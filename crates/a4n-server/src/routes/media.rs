use super::auth::authenticated_user_from_access_token;
use super::error::ApiError;
use crate::state::ServerState;
use a4n_runtime::host_api::{auth_records, storage_paths};
use a4n_runtime::{file_utils, media_file};
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use serde::Deserialize;
use std::path::Path;

const MEDIA_LABEL: &str = "媒体文件";

#[derive(Debug, Deserialize)]
pub(super) struct MediaQuery {
    path: String,
    access_token: Option<String>,
    media_token: Option<String>,
}

pub(super) async fn serve(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<MediaQuery>,
) -> Result<Response, ApiError> {
    let path = media_file::relative_media_path(&query.path).map_err(ApiError::bad_request)?;
    authorize_media_request(&state, &query, &path).await?;
    let data_dir = storage_paths::data_dir_path(state.runtime());
    let path = data_dir.join(path);
    let path = file_utils::canonicalize_path_in_data_dir(&data_dir, &path, MEDIA_LABEL).map_err(
        |err| {
            if err.is_outside_data_directory() {
                ApiError::forbidden(err.to_string())
            } else {
                ApiError::bad_request(err.to_string())
            }
        },
    )?;
    file_response(&path, headers.get(header::RANGE))
}

async fn authorize_media_request(
    state: &ServerState,
    query: &MediaQuery,
    path: &Path,
) -> Result<(), ApiError> {
    if let Some(token) = query
        .access_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        authenticated_user_from_access_token(state, token).await?;
        return Ok(());
    }

    let token = query
        .media_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::unauthorized("缺少媒体访问令牌"))?;
    auth_records::verify_media_token_for_path(state.runtime(), token, path)
        .await
        .map_err(ApiError::from)?;
    Ok(())
}

fn file_response(
    path: &Path,
    range_header: Option<&header::HeaderValue>,
) -> Result<Response, ApiError> {
    let media = media_file::read_media_file_response(
        path,
        range_header.map(header::HeaderValue::to_str),
        MEDIA_LABEL,
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
