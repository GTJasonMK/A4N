use a4n_core::error::AppError;
use a4n_core::models::user::user;
use a4n_orchestrator::auth::{oauth2_state, required_authorization_code};

use crate::AppState;

use super::contracts::{OAuth2AuthorizeInput, OAuth2CallbackInput};
use super::provider::{DEFAULT_OAUTH2_REDIRECT_URI, oauth2_provider_from_env};

pub fn oauth2_authorize_url(input: OAuth2AuthorizeInput) -> Result<String, AppError> {
    let provider = oauth2_provider_from_env(input.provider.trim(), DEFAULT_OAUTH2_REDIRECT_URI)?;
    Ok(provider.authorize_url(&oauth2_state()?))
}

pub(crate) async fn oauth2_login_user(
    state: &AppState,
    input: OAuth2CallbackInput,
) -> Result<user::Model, AppError> {
    let code = required_authorization_code(&input.code)?;
    let provider = oauth2_provider_from_env(input.provider.trim(), DEFAULT_OAUTH2_REDIRECT_URI)?;
    let token = provider.exchange_token(code).await?;
    let user_info = provider.get_user_info(&token).await?;

    state
        .orchestrator()
        .login_with_oauth2(
            &user_info.provider,
            &user_info.provider_id,
            user_info.email.as_deref(),
            user_info.username.as_deref(),
            user_info.avatar_url.as_deref(),
        )
        .await
}
