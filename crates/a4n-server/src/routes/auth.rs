use super::error::ApiError;
use crate::state::ServerState;
use a4n_api_contract::auth::UserSummary;
use a4n_runtime::host_api::auth_records;
use axum::http::HeaderMap;

pub(crate) async fn authenticated_user(
    state: &ServerState,
    headers: &HeaderMap,
) -> Result<UserSummary, ApiError> {
    let token = bearer_token(headers)?;
    authenticated_user_from_access_token(state, token).await
}

pub(crate) async fn authenticated_user_from_access_token(
    state: &ServerState,
    token: &str,
) -> Result<UserSummary, ApiError> {
    auth_records::user_from_access_token(state.runtime(), token)
        .await
        .map_err(ApiError::from)
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
