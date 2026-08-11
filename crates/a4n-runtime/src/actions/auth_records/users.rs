use super::contracts::{
    AuthResult, BootstrapAdminInput, ChangePasswordInput, LoginInput, RegisterInput,
    UpdateProfileInput, UserSummary,
};
use super::projections::to_user_summary;
use super::tokens::issue_auth_result;
use crate::AppState;
use a4n_core::error::AppError;
use a4n_core::models::user::user;
use a4n_orchestrator::auth::BootstrapAdminInput as OrchestratorBootstrapAdminInput;

pub async fn get_current_user(state: &AppState, user_id: i64) -> Result<UserSummary, AppError> {
    let user = state.orchestrator().current_user(user_id).await?;
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
        .orchestrator()
        .require_existing_user_or_bootstrap_admin(bootstrap)
        .await
}

pub async fn register_user(
    state: &AppState,
    input: RegisterInput,
    user_agent: Option<&str>,
) -> Result<AuthResult, AppError> {
    let user = state
        .orchestrator()
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
        .orchestrator()
        .login_user(&input.username, &input.password)
        .await?;
    issue_auth_result(state, user, user_agent).await
}

pub async fn oauth2_callback_login(
    state: &AppState,
    input: crate::actions::oauth::OAuth2CallbackInput,
    user_agent: Option<&str>,
) -> Result<AuthResult, AppError> {
    let user = oauth2_callback_user(state, input).await?;
    issue_auth_result(state, user, user_agent).await
}

pub async fn oauth2_callback_user(
    state: &AppState,
    input: crate::actions::oauth::OAuth2CallbackInput,
) -> Result<user::Model, AppError> {
    crate::actions::oauth::oauth2_login_user(state, input).await
}

pub async fn update_user_profile(
    state: &AppState,
    user_id: i64,
    input: UpdateProfileInput,
) -> Result<UserSummary, AppError> {
    if input.is_empty() {
        return Err(AppError::InvalidArgument(
            "更新用户资料至少需要一个字段".to_string(),
        ));
    }
    let avatar_url = input.avatar_url.as_ref().map(|value| value.as_deref());
    let user = state
        .orchestrator()
        .update_user_profile(user_id, input.email.as_deref(), avatar_url)
        .await?;
    Ok(to_user_summary(&user))
}

pub async fn change_user_password(
    state: &AppState,
    user_id: i64,
    input: ChangePasswordInput,
) -> Result<(), AppError> {
    state
        .orchestrator()
        .change_user_password(user_id, &input.old_password, &input.new_password)
        .await
}
