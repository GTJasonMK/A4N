use super::error::ApiError;
use crate::state::ServerState;
use a4n_core::models::user::user;
use a4n_runtime::host_api::auth_records;
use axum::http::HeaderMap;

pub async fn authenticated_user(
    state: &ServerState,
    headers: &HeaderMap,
) -> Result<user::Model, ApiError> {
    let token = bearer_token(headers)?;
    authenticated_user_from_access_token(state, token).await
}

pub async fn authenticated_user_from_access_token(
    state: &ServerState,
    token: &str,
) -> Result<user::Model, ApiError> {
    auth_records::user_from_access_token(state.runtime(), token)
        .await
        .map_err(ApiError::from)
}

pub(crate) async fn issue_auth_response(
    state: &ServerState,
    headers: &HeaderMap,
    user: user::Model,
) -> Result<auth_records::AuthResult, ApiError> {
    auth_records::issue_auth_result(state.runtime(), user, user_agent(headers).as_deref())
        .await
        .map_err(Into::into)
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| ApiError::unauthorized("缺少 Authorization 头"))?
        .to_str()
        .map_err(|_| ApiError::unauthorized("Authorization 头不是有效字符串"))?;
    value
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| ApiError::unauthorized("Authorization 必须使用 Bearer token"))
}

pub(crate) fn user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}
