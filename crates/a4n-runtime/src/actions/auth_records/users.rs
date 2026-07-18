use super::contracts::{
    AuthResult, BootstrapAdminInput, ChangePasswordInput, LoginInput, OAuth2LoginResult,
    RegisterInput, UpdateProfileInput, UserSummary,
};
use super::projections::to_user_summary;
use super::rules::{ensure_same_user, profile_email_update};
use super::tokens::issue_auth_result;
use crate::AppState;
use a4n_core::error::AppError;
use a4n_core::models::user::user;
use a4n_orchestrator::auth::BootstrapAdminInput as OrchestratorBootstrapAdminInput;

pub async fn get_user(state: &AppState, user_id: i64) -> Result<UserSummary, AppError> {
    let user = state.orchestrator.auth_user_by_id(user_id).await?;
    Ok(to_user_summary(&user))
}

pub async fn get_current_user(state: &AppState, user_id: i64) -> Result<UserSummary, AppError> {
    let user = state.orchestrator.current_user(user_id).await?;
    Ok(to_user_summary(&user))
}

pub async fn require_existing_user_or_bootstrap_admin(
    state: &AppState,
    bootstrap: Option<BootstrapAdminInput<'_>>,
) -> Result<(), AppError> {
    let bootstrap = bootstrap.map(|input| OrchestratorBootstrapAdminInput {
        username: input.username,
        email: input.email,
        password: input.password,
    });
    state
        .orchestrator
        .require_existing_user_or_bootstrap_admin(bootstrap)
        .await
}

pub async fn register_user(
    state: &AppState,
    input: RegisterInput,
    user_agent: Option<&str>,
) -> Result<AuthResult, AppError> {
    let user = state
        .orchestrator
        .register_user_with_invite(
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
    let user = state
        .orchestrator
        .login_user(&input.username, &input.password)
        .await?;
    issue_auth_result(state, user, user_agent).await
}

pub async fn oauth2_callback_login(
    state: &AppState,
    input: crate::actions::oauth::OAuth2CallbackInput,
    user_agent: Option<&str>,
) -> Result<OAuth2LoginResult, AppError> {
    let user = oauth2_callback_user(state, input).await?;
    let result = issue_auth_result(state, user, user_agent).await?;
    Ok(OAuth2LoginResult {
        user: result.user,
        tokens: result.tokens,
        session: result.session,
    })
}

pub async fn oauth2_callback_user(
    state: &AppState,
    input: crate::actions::oauth::OAuth2CallbackInput,
) -> Result<user::Model, AppError> {
    crate::actions::oauth::oauth2_login_user(state, input).await
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
        .orchestrator
        .update_user_profile(target_user_id, email, avatar_url)
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
        .orchestrator
        .change_user_password(target_user_id, &input.old_password, &input.new_password)
        .await
}
