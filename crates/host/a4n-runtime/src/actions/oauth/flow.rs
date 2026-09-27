use a4n_api_contract::auth::{
    OAuth2AvailableProvidersInput, UserSummary, required_authorization_code,
};
use a4n_config::required_non_empty_env_string;
use a4n_error::AppError;
use url::Url;

use crate::AppState;

use super::provider::oauth2_provider_from_env;
use super::{OAuth2AuthorizeInput, OAuth2CallbackInput};

pub fn oauth2_authorize_url(input: OAuth2AuthorizeInput) -> Result<String, AppError> {
    validate_oauth_token(&input.state, "state", 32, 128)?;
    validate_oauth_token(&input.code_challenge, "code_challenge", 43, 43)?;
    let redirect_uri = oauth_redirect_uri(input.redirect_uri)?;
    let provider = oauth2_provider_from_env(input.provider.trim(), &redirect_uri)?;
    Ok(provider.authorize_url(&input.state, &input.code_challenge))
}

pub fn oauth2_available_providers(
    input: OAuth2AvailableProvidersInput,
) -> Result<Vec<String>, AppError> {
    let redirect_uri = oauth_redirect_uri(input.redirect_uri)?;
    validate_redirect_uri(&redirect_uri)?;
    super::provider::oauth2_available_providers()
}

fn oauth_redirect_uri(requested: Option<String>) -> Result<String, AppError> {
    let redirect_uri = match requested {
        Some(value) => value,
        None => required_non_empty_env_string("A4N_OAUTH_REDIRECT_URI")?,
    };
    validate_redirect_uri(&redirect_uri)?;
    Ok(redirect_uri)
}

pub(crate) async fn oauth2_login_user(
    state: &AppState,
    input: OAuth2CallbackInput,
) -> Result<UserSummary, AppError> {
    let code = required_authorization_code(&input.code)?;
    validate_redirect_uri(&input.redirect_uri)?;
    validate_oauth_token(&input.code_verifier, "code_verifier", 43, 128)?;
    let provider = oauth2_provider_from_env(input.provider.trim(), &input.redirect_uri)?;
    let token = provider.exchange_token(code, &input.code_verifier).await?;
    let user_info = provider.get_user_info(&token).await?;

    state
        .auth_mutation_application()
        .login_with_oauth2(
            &user_info.provider,
            &user_info.provider_id,
            user_info.email.as_deref(),
            user_info.username.as_deref(),
            user_info.avatar_url.as_deref(),
        )
        .await
}

pub(super) fn validate_redirect_uri(value: &str) -> Result<(), AppError> {
    let uri =
        Url::parse(value).map_err(|_| AppError::InvalidArgument("OAuth2 回调地址无效".into()))?;
    let loopback = matches!(uri.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if (uri.scheme() != "https" && !(uri.scheme() == "http" && loopback))
        || !uri.username().is_empty()
        || uri.password().is_some()
        || uri.query().is_some()
        || uri.fragment().is_some()
        || !uri.path().ends_with("/auth/callback")
    {
        return Err(AppError::InvalidArgument(
            "OAuth2 回调地址必须是 HTTPS 或本机 HTTP、以 /auth/callback 结尾，且不能包含凭据、查询或片段"
                .into(),
        ));
    }
    Ok(())
}

fn validate_oauth_token(value: &str, field: &str, min: usize, max: usize) -> Result<(), AppError> {
    if !(min..=max).contains(&value.len())
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._~".contains(&byte))
    {
        return Err(AppError::InvalidArgument(format!(
            "OAuth2 {field} 格式无效"
        )));
    }
    Ok(())
}
