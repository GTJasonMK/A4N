use super::tokens::issue_auth_result;
use super::{
    AuthResult, ChangePasswordInput, LoginInput, RegisterInput, UpdateProfileInput, UserSummary,
};
use crate::AppState;
use a4n_error::AppError;

pub async fn get_current_user(state: &AppState, user_id: i64) -> Result<UserSummary, AppError> {
    state.auth_application().current_user(user_id).await
}

pub async fn require_existing_user_or_bootstrap_admin(
    state: &AppState,
    bootstrap: Option<(&str, &str, &str)>,
) -> Result<(), AppError> {
    state
        .auth_application()
        .require_existing_user_or_bootstrap_admin(bootstrap)
        .await
}

pub async fn register_user(
    state: &AppState,
    input: RegisterInput,
    user_agent: Option<&str>,
) -> Result<AuthResult, AppError> {
    let user = state.auth_application().register_user(input).await?;
    issue_auth_result(state, user, user_agent).await
}

pub async fn login_user(
    state: &AppState,
    input: LoginInput,
    user_agent: Option<&str>,
) -> Result<AuthResult, AppError> {
    let user = state.auth_application().login_user(input).await?;
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

async fn oauth2_callback_user(
    state: &AppState,
    input: crate::actions::oauth::OAuth2CallbackInput,
) -> Result<UserSummary, AppError> {
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
    state
        .auth_application()
        .update_profile(user_id, input)
        .await
}

pub async fn change_user_password(
    state: &AppState,
    user_id: i64,
    input: ChangePasswordInput,
) -> Result<(), AppError> {
    state
        .auth_application()
        .change_password(user_id, input)
        .await
}
