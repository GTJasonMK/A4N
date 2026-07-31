use super::contracts::{AuthResult, RefreshTokenInput, TokenPairSummary};
use super::projections::{to_session_summary, to_user_summary};
use crate::AppState;
use crate::jwt;
use a4n_core::error::AppError;
use a4n_core::models::user::user;
use a4n_orchestrator::auth::{required_session_id, required_session_user_id};
use std::path::Path;

pub async fn user_from_access_token(
    state: &AppState,
    access_token: &str,
) -> Result<user::Model, AppError> {
    let claims = jwt::verify_token(access_token, jwt_secret(state)?)?;
    if claims.token_type != "access" {
        return Err(AppError::Unauthorized("令牌类型错误：需要访问令牌".into()));
    }
    let user_id = claims
        .sub
        .parse::<i64>()
        .map_err(|_| AppError::Unauthorized("访问令牌 subject 不是有效用户 ID".into()))?;
    let session_id = claims
        .session_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Unauthorized("访问令牌缺少会话绑定，请重新登录".into()))?;
    state
        .auth()
        .verify_active_session(user_id, session_id)
        .await?;
    state.auth().current_user(user_id).await
}

pub async fn verify_media_token_for_path(
    state: &AppState,
    token: &str,
    path: &Path,
) -> Result<(), AppError> {
    let claims = jwt::verify_token(token, jwt_secret(state)?)?;
    if claims.token_type != "media" {
        return Err(AppError::Unauthorized("令牌类型错误：需要媒体令牌".into()));
    }
    let expected = claims
        .media_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Unauthorized("媒体令牌缺少绑定路径".into()))?;
    let expected_path =
        crate::media_file::relative_media_path(expected).map_err(AppError::Unauthorized)?;
    if expected_path != path {
        return Err(AppError::Unauthorized("媒体令牌与请求文件不匹配".into()));
    }
    let user_id = claims
        .sub
        .parse::<i64>()
        .map_err(|_| AppError::Unauthorized("媒体令牌 subject 不是有效用户 ID".into()))?;
    state.auth().current_user(user_id).await?;
    Ok(())
}

pub async fn issue_auth_result(
    state: &AppState,
    user: user::Model,
    user_agent: Option<&str>,
) -> Result<AuthResult, AppError> {
    let secret = jwt_secret(state)?;
    let mut pair = jwt::create_token_pair_with_role(
        &user.id.to_string(),
        Some(&user.username),
        &user.role,
        secret,
    )?;
    let session = state
        .auth()
        .create_session(user.id, &pair.refresh_token, user_agent)
        .await?;
    let session_id = session.id.clone();
    pair.access_token = jwt::create_session_access_token_with_role(
        &user.id.to_string(),
        Some(&user.username),
        &user.role,
        &session_id,
        secret,
    )?;

    Ok(AuthResult {
        user: to_user_summary(&user),
        tokens: TokenPairSummary {
            access_token: pair.access_token,
            refresh_token: pair.refresh_token,
            expires_in: pair.expires_in,
            token_type: pair.token_type,
            session_id,
        },
        session: to_session_summary(&session),
    })
}

pub async fn refresh_token(
    state: &AppState,
    input: RefreshTokenInput,
) -> Result<TokenPairSummary, AppError> {
    let session_id = required_session_id(&input.session_id, "刷新令牌")?;
    let secret = jwt_secret(state)?;
    let claims = jwt::verify_token(&input.refresh_token, secret)?;
    if claims.token_type != "refresh" {
        return Err(AppError::Unauthorized("令牌类型错误：需要刷新令牌".into()));
    }
    let user_id = claims
        .sub
        .parse::<i64>()
        .map_err(|_| AppError::Unauthorized("刷新令牌 subject 不是有效用户 ID".into()))?;
    let user_id = required_session_user_id(user_id, "刷新令牌")?;

    let session = state
        .auth()
        .verify_session_refresh_token(user_id, &session_id, &input.refresh_token)
        .await?;
    let user = state.auth().current_user(user_id).await?;
    let pair = jwt::refresh_access_token_with_role(
        &input.refresh_token,
        Some(&user.username),
        &user.role,
        &session_id,
        secret,
    )?;
    state
        .auth()
        .rotate_session_refresh_token(session, &input.refresh_token, &pair.refresh_token)
        .await?;

    Ok(TokenPairSummary {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
        expires_in: pair.expires_in,
        token_type: pair.token_type,
        session_id,
    })
}

fn jwt_secret(state: &AppState) -> Result<&str, AppError> {
    state.required_jwt_secret()
}
