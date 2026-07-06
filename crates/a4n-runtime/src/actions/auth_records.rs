use crate::AppState;
use a4n_core::error::AppError;
use a4n_core::models::user::{user, user_session};
use a4n_services::auth::jwt;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSummary {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub oauth_provider: Option<String>,
    pub oauth_subject: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub last_login_at: Option<String>,
}

impl UserSummary {
    pub fn with_display_name(mut self) -> Self {
        self.display_name = Some(self.username.clone());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub invite_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateProfileInput {
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub email: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub avatar_url: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangePasswordInput {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub user_id: i64,
    pub user_agent: Option<String>,
    pub is_revoked: bool,
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    pub user: UserSummary,
    pub tokens: TokenPairSummary,
    pub session: SessionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshTokenInput {
    pub refresh_token: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPairSummary {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2LoginResult {
    pub user: UserSummary,
    pub tokens: TokenPairSummary,
    pub session: SessionSummary,
}

pub async fn get_user(state: &AppState, user_id: i64) -> Result<UserSummary, AppError> {
    let user = state.auth.get(user_id).await?;
    Ok(to_user_summary(&user))
}

pub async fn get_current_user(state: &AppState, user_id: i64) -> Result<UserSummary, AppError> {
    let user = state.auth.get_current_user(user_id).await?;
    Ok(to_user_summary(&user))
}

pub async fn user_from_access_token(
    state: &AppState,
    access_token: &str,
) -> Result<user::Model, AppError> {
    let claims = jwt::verify_token(access_token, jwt_secret(state)?)?;
    if claims.token_type != "access" {
        return Err(AppError::Forbidden("令牌类型错误：需要访问令牌".into()));
    }
    let user_id = claims
        .sub
        .parse::<i64>()
        .map_err(|_| AppError::Forbidden("访问令牌 subject 不是有效用户 ID".into()))?;
    state.auth.get_current_user(user_id).await
}

pub async fn register_user(
    state: &AppState,
    input: RegisterInput,
    user_agent: Option<&str>,
) -> Result<AuthResult, AppError> {
    let user = state
        .auth
        .register_with_invite(
            &input.username,
            &input.email,
            &input.password,
            &input.invite_code,
        )
        .await?;
    issue_auth_result(state, user, user_agent).await
}

pub async fn login_user(
    state: &AppState,
    input: LoginInput,
    user_agent: Option<&str>,
) -> Result<AuthResult, AppError> {
    let user = state.auth.login(&input.username, &input.password).await?;
    issue_auth_result(state, user, user_agent).await
}

pub async fn issue_auth_result(
    state: &AppState,
    user: user::Model,
    user_agent: Option<&str>,
) -> Result<AuthResult, AppError> {
    let secret = jwt_secret(state)?;
    let pair = jwt::create_token_pair_with_role(
        &user.id.to_string(),
        Some(&user.username),
        &user.role,
        secret,
    )?;
    let session = state
        .auth
        .create_session(user.id, &pair.refresh_token, user_agent)
        .await?;
    let session_id = session.id.clone();

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

pub async fn update_user_profile_for_user(
    state: &AppState,
    caller_user_id: i64,
    target_user_id: i64,
    input: UpdateProfileInput,
) -> Result<UserSummary, AppError> {
    ensure_same_user(caller_user_id, target_user_id, "只能更新当前登录用户资料")?;
    let email = profile_email_update(input.email.as_ref())?;
    let avatar_url = input.avatar_url.as_ref().map(|value| value.as_deref());
    let user = state
        .auth
        .update_profile(target_user_id, email, avatar_url)
        .await?;
    Ok(to_user_summary(&user))
}

pub async fn change_user_password_for_user(
    state: &AppState,
    caller_user_id: i64,
    target_user_id: i64,
    input: ChangePasswordInput,
) -> Result<(), AppError> {
    ensure_same_user(caller_user_id, target_user_id, "只能修改当前登录用户密码")?;
    state
        .auth
        .change_password(target_user_id, &input.old_password, &input.new_password)
        .await
}

pub async fn logout_user(
    state: &AppState,
    user_id: Option<i64>,
    session_id: Option<&str>,
) -> Result<(), AppError> {
    let (user_id, session_id) = required_logout_identity(user_id, session_id)?;
    state.auth.revoke_session(user_id, &session_id).await
}

pub async fn refresh_token(
    state: &AppState,
    input: RefreshTokenInput,
) -> Result<TokenPairSummary, AppError> {
    let session_id = required_session_id(&input.session_id, "刷新令牌")?;
    let secret = jwt_secret(state)?;
    let claims = jwt::verify_token(&input.refresh_token, secret)?;
    if claims.token_type != "refresh" {
        return Err(AppError::Forbidden("令牌类型错误：需要刷新令牌".into()));
    }
    let user_id = claims
        .sub
        .parse::<i64>()
        .map_err(|_| AppError::Forbidden("刷新令牌 subject 不是有效用户 ID".into()))?;
    let user_id = required_session_user_id(user_id, "刷新令牌")?;

    let session = state
        .auth
        .verify_session_refresh_token(user_id, &session_id, &input.refresh_token)
        .await?;
    let user = state.auth.get_current_user(user_id).await?;
    let pair = jwt::refresh_access_token_with_role(
        &input.refresh_token,
        Some(&user.username),
        &user.role,
        secret,
    )?;
    state
        .auth
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

pub async fn list_sessions_for_user(
    state: &AppState,
    caller_user_id: i64,
    target_user_id: i64,
) -> Result<Vec<SessionSummary>, AppError> {
    ensure_same_user(caller_user_id, target_user_id, "只能列出当前登录用户会话")?;
    let user_id = required_session_user_id(target_user_id, "列出会话")?;
    let sessions = state.auth.list_sessions(user_id).await?;
    Ok(sessions.iter().map(to_session_summary).collect())
}

pub async fn revoke_session_for_user(
    state: &AppState,
    caller_user_id: i64,
    target_user_id: i64,
    session_id: &str,
) -> Result<(), AppError> {
    ensure_same_user(caller_user_id, target_user_id, "只能撤销当前登录用户会话")?;
    let user_id = required_session_user_id(target_user_id, "撤销会话")?;
    let session_id = required_session_id(session_id, "撤销会话")?;
    state.auth.revoke_session(user_id, &session_id).await
}

pub fn to_user_summary(u: &user::Model) -> UserSummary {
    UserSummary {
        id: u.id,
        username: u.username.clone(),
        email: u.email.clone(),
        role: u.role.clone(),
        oauth_provider: u.oauth_provider.clone(),
        oauth_subject: u.oauth_subject.clone(),
        avatar_url: u.avatar_url.clone(),
        is_active: u.is_active,
        display_name: None,
        created_at: Some(u.created_at.to_string()),
        updated_at: Some(u.updated_at.to_string()),
        last_login_at: u.last_login_at.map(|d| d.to_string()),
    }
}

fn to_session_summary(s: &user_session::Model) -> SessionSummary {
    SessionSummary {
        id: s.id.clone(),
        user_id: s.user_id,
        user_agent: s.user_agent.clone(),
        is_revoked: s.is_revoked,
        created_at: Some(s.created_at.to_string()),
        expires_at: Some(s.expires_at.to_string()),
        revoked_at: s.revoked_at.map(|d| d.to_string()),
        last_used_at: s.last_used_at.map(|d| d.to_string()),
    }
}

fn jwt_secret(state: &AppState) -> Result<&str, AppError> {
    state
        .config
        .security
        .jwt_secret
        .as_deref()
        .map(str::trim)
        .filter(|secret| !secret.is_empty())
        .ok_or_else(|| AppError::ConfigError("A4N_JWT_SECRET 不能为空".into()))
}

fn ensure_same_user(
    caller_user_id: i64,
    target_user_id: i64,
    message: &str,
) -> Result<(), AppError> {
    if caller_user_id == target_user_id {
        Ok(())
    } else {
        Err(AppError::Forbidden(message.to_string()))
    }
}

fn profile_email_update(email: Option<&Option<String>>) -> Result<Option<&str>, AppError> {
    match email {
        None => Ok(None),
        Some(Some(email)) => Ok(Some(email.as_str())),
        Some(None) => Err(AppError::InvalidArgument("email 不能为空".into())),
    }
}

fn required_logout_identity(
    user_id: Option<i64>,
    session_id: Option<&str>,
) -> Result<(i64, String), AppError> {
    let user_id = user_id
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError::InvalidArgument("登出缺少有效用户 ID".into()))?;
    let session_id = session_id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("登出缺少会话 ID".into()))?;
    Ok((user_id, session_id.to_string()))
}

fn required_session_user_id(user_id: i64, operation: &str) -> Result<i64, AppError> {
    if user_id > 0 {
        Ok(user_id)
    } else {
        Err(AppError::InvalidArgument(format!(
            "{operation}缺少有效用户 ID"
        )))
    }
}

fn required_session_id(session_id: &str, operation: &str) -> Result<String, AppError> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        Err(AppError::InvalidArgument(format!("{operation}缺少会话 ID")))
    } else {
        Ok(session_id.to_string())
    }
}

fn deserialize_nullable_update_field<'de, D, T>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::<T>::deserialize(deserializer)?))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        UpdateProfileInput, ensure_same_user, profile_email_update, required_logout_identity,
        required_session_id, required_session_user_id,
    };

    #[test]
    fn update_profile_input_distinguishes_missing_from_null_avatar() {
        let missing: UpdateProfileInput = serde_json::from_value(json!({})).unwrap();
        let null: UpdateProfileInput =
            serde_json::from_value(json!({ "avatar_url": null })).unwrap();
        let value: UpdateProfileInput =
            serde_json::from_value(json!({ "avatar_url": "https://cdn/avatar.png" })).unwrap();

        assert_eq!(missing.avatar_url, None);
        assert_eq!(null.avatar_url, Some(None));
        assert_eq!(
            value.avatar_url,
            Some(Some("https://cdn/avatar.png".to_string()))
        );
    }

    #[test]
    fn update_profile_input_distinguishes_missing_from_null_email() {
        let missing: UpdateProfileInput = serde_json::from_value(json!({})).unwrap();
        let null: UpdateProfileInput = serde_json::from_value(json!({ "email": null })).unwrap();
        let value: UpdateProfileInput =
            serde_json::from_value(json!({ "email": "alice@example.com" })).unwrap();

        assert_eq!(missing.email, None);
        assert_eq!(null.email, Some(None));
        assert_eq!(value.email, Some(Some("alice@example.com".to_string())));
    }

    #[test]
    fn profile_email_update_allows_missing_email() {
        assert_eq!(profile_email_update(None).unwrap(), None);
    }

    #[test]
    fn profile_email_update_rejects_explicit_null_email() {
        let err = profile_email_update(Some(&None)).expect_err("email null must not be a no-op");

        assert!(err.to_string().contains("email"));
    }

    #[test]
    fn profile_email_update_keeps_email_value_for_service_validation() {
        let email = Some(" alice@example.com ".to_string());

        assert_eq!(
            profile_email_update(Some(&email)).unwrap(),
            Some(" alice@example.com ")
        );
    }

    #[test]
    fn same_user_guard_rejects_mismatch() {
        let err = ensure_same_user(1, 2, "只能操作当前登录用户")
            .expect_err("self-service action must reject a different target user");

        assert!(err.to_string().contains("只能操作当前登录用户"));
    }

    #[test]
    fn logout_identity_requires_user_id() {
        let err = required_logout_identity(None, Some("session-1"))
            .expect_err("logout must not silently succeed without user id");

        assert!(err.to_string().contains("用户 ID"));
    }

    #[test]
    fn logout_identity_requires_positive_user_id() {
        let err = required_logout_identity(Some(0), Some("session-1"))
            .expect_err("logout must reject invalid user id before revoke");

        assert!(err.to_string().contains("用户 ID"));
    }

    #[test]
    fn logout_identity_requires_session_id() {
        let err = required_logout_identity(Some(7), None)
            .expect_err("logout must not silently succeed without session id");

        assert!(err.to_string().contains("会话 ID"));
    }

    #[test]
    fn logout_identity_rejects_blank_session_id() {
        let err = required_logout_identity(Some(7), Some("   "))
            .expect_err("logout must reject blank session id before revoke");

        assert!(err.to_string().contains("会话 ID"));
    }

    #[test]
    fn logout_identity_trims_session_id() {
        let (user_id, session_id) =
            required_logout_identity(Some(7), Some(" session-1 ")).expect("valid logout identity");

        assert_eq!(user_id, 7);
        assert_eq!(session_id, "session-1");
    }

    #[test]
    fn session_user_id_requires_positive_id() {
        let err = required_session_user_id(0, "列出会话")
            .expect_err("session command must reject invalid user id before service call");

        assert!(err.to_string().contains("用户 ID"));
    }

    #[test]
    fn session_id_requires_nonblank_value() {
        let err = required_session_id("   ", "撤销会话")
            .expect_err("session command must reject blank session id before service call");

        assert!(err.to_string().contains("会话 ID"));
    }

    #[test]
    fn session_id_is_trimmed_at_command_boundary() {
        let session_id = required_session_id(" session-1 ", "撤销会话").expect("valid session id");

        assert_eq!(session_id, "session-1");
    }
}
