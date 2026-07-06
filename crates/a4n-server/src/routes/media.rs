use super::auth::authenticated_user_from_access_token;
use super::error::ApiError;
use crate::state::ServerState;
use a4n_runtime::{file_utils, media_file};
use a4n_services::auth::jwt;
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
    let path = state.config.data_dir.join(path);
    let path =
        file_utils::canonicalize_path_in_data_dir(&state.config.data_dir, &path, MEDIA_LABEL)
            .map_err(|err| {
                if err.contains("不在应用数据目录内") {
                    ApiError::forbidden(err)
                } else {
                    ApiError::bad_request(err)
                }
            })?;
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
    let secret = state
        .config
        .security
        .jwt_secret
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::bad_request("A4N_JWT_SECRET 不能为空"))?;
    let claims = jwt::verify_token(token, secret).map_err(|err| {
        let message = err.to_string();
        if message.contains("JWT") {
            ApiError::unauthorized(message)
        } else {
            ApiError::forbidden(message)
        }
    })?;
    if claims.token_type != "media" {
        return Err(ApiError::forbidden("令牌类型错误：需要媒体令牌"));
    }
    let expected = claims
        .media_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::forbidden("媒体令牌缺少绑定路径"))?;
    let expected_path = media_file::relative_media_path(expected).map_err(ApiError::forbidden)?;
    if expected_path != path {
        return Err(ApiError::forbidden("媒体令牌与请求文件不匹配"));
    }
    let user_id = claims
        .sub
        .parse::<i64>()
        .map_err(|_| ApiError::forbidden("媒体令牌 subject 不是有效用户 ID"))?;
    state
        .auth
        .get_current_user(user_id)
        .await
        .map_err(|err| ApiError::forbidden(err.to_string()))?;
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
