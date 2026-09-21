use super::{AuthResult, RefreshTokenInput, TokenPairSummary};
use crate::AppState;
use crate::jwt;
use a4n_api_contract::auth::{UserSummary, required_session_id, required_session_user_id};
use a4n_error::AppError;
use a4n_storage::canonicalize_path_in_data_dir;
use std::path::{Component, Path, PathBuf};

pub async fn user_from_access_token(
    state: &AppState,
    access_token: &str,
) -> Result<UserSummary, AppError> {
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
        .auth_read_application()
        .verify_active_session(user_id, session_id)
        .await?;
    state.auth_read_application().current_user(user_id).await
}

pub async fn verify_media_token_for_path(
    state: &AppState,
    token: &str,
    path: &Path,
) -> Result<UserSummary, AppError> {
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
    let expected_path = a4n_media::relative_media_path(expected).map_err(AppError::Unauthorized)?;
    if expected_path != path {
        return Err(AppError::Unauthorized("媒体令牌与请求文件不匹配".into()));
    }
    let user_id = claims
        .sub
        .parse::<i64>()
        .map_err(|_| AppError::Unauthorized("媒体令牌 subject 不是有效用户 ID".into()))?;
    state.auth_read_application().current_user(user_id).await
}

pub async fn authorize_media_file_for_user(
    state: &AppState,
    user_id: i64,
    relative_path: &Path,
) -> Result<PathBuf, AppError> {
    let data_dir = state.data_dir().to_path_buf();
    let candidate = data_dir.join(relative_path);
    let (path, relative) = tokio::task::spawn_blocking(move || {
        let data_dir = data_dir.canonicalize().map_err(AppError::Io)?;
        let path =
            canonicalize_path_in_data_dir(&data_dir, &candidate, "媒体文件").map_err(|error| {
                if error.is_outside_data_directory() {
                    AppError::Forbidden(error.to_string())
                } else {
                    AppError::InvalidArgument(error.to_string())
                }
            })?;
        let relative = path
            .strip_prefix(&data_dir)
            .map_err(|_| AppError::Forbidden("媒体文件不在数据目录内".to_string()))?
            .to_path_buf();
        Ok::<_, AppError>((path, relative))
    })
    .await
    .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))??;

    // Authorize the resolved target, including links into a different project/theme.
    let mut components = relative.components();
    let (Some(Component::Normal(scope)), Some(Component::Normal(owner)), Some(_)) =
        (components.next(), components.next(), components.next())
    else {
        return Err(AppError::Forbidden("媒体文件缺少可授权的归属".to_string()));
    };
    let owner_id = owner
        .to_str()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError::Forbidden("媒体文件归属 ID 无效".to_string()))?;
    match scope.to_str() {
        Some("projects") => {
            crate::actions::access::verify_project_access(state, user_id, owner_id).await?
        }
        Some("themes") => {
            state
                .theme_read_application()
                .export_theme(user_id, owner_id)
                .await?;
        }
        _ => {
            return Err(AppError::Forbidden(
                "媒体文件不在可授权的资源目录内".to_string(),
            ));
        }
    }
    Ok(path)
}

pub async fn issue_auth_result(
    state: &AppState,
    user: UserSummary,
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
        .auth_mutation_application()
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
        user,
        tokens: TokenPairSummary {
            access_token: pair.access_token,
            refresh_token: pair.refresh_token,
            expires_in: pair.expires_in,
            token_type: pair.token_type,
            session_id,
        },
        session,
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

    let user = state.auth_read_application().current_user(user_id).await?;
    let pair = jwt::refresh_access_token_with_role(
        &input.refresh_token,
        Some(&user.username),
        &user.role,
        &session_id,
        secret,
    )?;
    state
        .auth_mutation_application()
        .rotate_session_refresh_token(
            user_id,
            &session_id,
            &input.refresh_token,
            &pair.refresh_token,
        )
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
