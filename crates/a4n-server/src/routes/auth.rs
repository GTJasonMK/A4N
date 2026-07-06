use super::error::ApiError;
use crate::state::ServerState;
use a4n_core::error::AppError;
use a4n_core::models::user::user;
use a4n_runtime::actions::auth_records;
use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LogoutRequest {
    session_id: String,
}

pub async fn login(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<auth_records::LoginInput>,
) -> Result<Json<auth_records::AuthResult>, ApiError> {
    let response = auth_records::login_user(&state, input, user_agent(&headers).as_deref()).await?;
    Ok(Json(response))
}

pub async fn register(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<auth_records::RegisterInput>,
) -> Result<Json<auth_records::AuthResult>, ApiError> {
    let response =
        auth_records::register_user(&state, input, user_agent(&headers).as_deref()).await?;
    Ok(Json(response))
}

pub async fn me(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<auth_records::UserSummary>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(auth_records::to_user_summary(&user)))
}

pub(super) async fn logout(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<LogoutRequest>,
) -> Result<(), ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    auth_records::logout_user(&state, Some(user.id), Some(&input.session_id)).await?;
    Ok(())
}

pub async fn refresh(
    State(state): State<ServerState>,
    Json(input): Json<auth_records::RefreshTokenInput>,
) -> Result<Json<auth_records::TokenPairSummary>, ApiError> {
    Ok(Json(
        auth_records::refresh_token(&state, input)
            .await
            .map_err(auth_token_error)?,
    ))
}

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
    auth_records::user_from_access_token(state, token)
        .await
        .map_err(auth_token_error)
}

pub(crate) async fn issue_auth_response(
    state: &ServerState,
    headers: &HeaderMap,
    user: user::Model,
) -> Result<auth_records::AuthResult, ApiError> {
    auth_records::issue_auth_result(state, user, user_agent(headers).as_deref())
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

fn user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn auth_token_error(error: AppError) -> ApiError {
    match &error {
        AppError::Forbidden(message)
            if message.contains("JWT")
                || message.contains("令牌类型")
                || message.contains("subject") =>
        {
            ApiError::unauthorized(error.to_string())
        }
        _ => error.into(),
    }
}
