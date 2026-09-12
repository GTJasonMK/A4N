use super::auth::{authenticated_user, authenticated_user_from_access_token};
use super::error::ApiError;
use crate::state::ServerState;
use a4n_error::AppError;
use a4n_media::relative_media_path;
use a4n_runtime::host_api::auth_records;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, Request, header};
use axum::response::Response;
use serde::Deserialize;
use std::path::Path;
use tower_http::services::ServeFile;

#[cfg(test)]
mod tests;

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
    let path = relative_media_path(&query.path).map_err(ApiError::bad_request)?;
    let user_id = authenticate_media_request(&state, &headers, &query, &path).await?;
    let path = auth_records::authorize_media_file_for_user(state.runtime(), user_id, &path).await?;
    file_response(&path, headers.get(header::RANGE)).await
}

async fn authenticate_media_request(
    state: &ServerState,
    headers: &HeaderMap,
    query: &MediaQuery,
    path: &Path,
) -> Result<i64, ApiError> {
    if headers.contains_key(header::AUTHORIZATION) {
        return Ok(authenticated_user(state, headers).await?.id);
    }
    if let Some(token) = query
        .access_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(authenticated_user_from_access_token(state, token).await?.id);
    }

    let token = query
        .media_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::unauthorized("缺少媒体访问令牌"))?;
    auth_records::verify_media_token_for_path(state.runtime(), token, path)
        .await
        .map(|user| user.id)
        .map_err(ApiError::from)
}

pub(super) async fn file_response(
    path: &Path,
    range_header: Option<&header::HeaderValue>,
) -> Result<Response, ApiError> {
    let content_type =
        a4n_domain::media::media_mime_from_path(path).map_err(ApiError::bad_request)?;
    let mut request = Request::new(Body::empty());
    if let Some(range) = range_header {
        request.headers_mut().insert(header::RANGE, range.clone());
    }
    let mut response = ServeFile::new(path)
        .try_call(request)
        .await
        .map_err(|error| ApiError::from(AppError::Io(error)))?
        .map(Body::new);
    if response.status().is_success() {
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static(content_type),
        );
    }
    Ok(response)
}
