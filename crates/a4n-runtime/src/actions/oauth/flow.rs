use a4n_api_contract::auth::{UserSummary, required_authorization_code};
use a4n_error::AppError;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::AppState;

use super::provider::{DEFAULT_OAUTH2_REDIRECT_URI, oauth2_provider_from_env};
use super::{OAuth2AuthorizeInput, OAuth2CallbackInput};

pub fn oauth2_authorize_url(input: OAuth2AuthorizeInput) -> Result<String, AppError> {
    let provider = oauth2_provider_from_env(input.provider.trim(), DEFAULT_OAUTH2_REDIRECT_URI)?;
    Ok(provider.authorize_url(&oauth2_state()?))
}

pub(crate) async fn oauth2_login_user(
    state: &AppState,
    input: OAuth2CallbackInput,
) -> Result<UserSummary, AppError> {
    let code = required_authorization_code(&input.code)?;
    let provider = oauth2_provider_from_env(input.provider.trim(), DEFAULT_OAUTH2_REDIRECT_URI)?;
    let token = provider.exchange_token(code).await?;
    let user_info = provider.get_user_info(&token).await?;

    state
        .auth_application()
        .login_with_oauth2(
            &user_info.provider,
            &user_info.provider_id,
            user_info.email.as_deref(),
            user_info.username.as_deref(),
            user_info.avatar_url.as_deref(),
        )
        .await
}

fn oauth2_state() -> Result<String, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .map_err(|error| AppError::Internal(anyhow::anyhow!("生成 OAuth2 state 失败：{error}")))
}
