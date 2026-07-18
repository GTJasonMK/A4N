use a4n_core::error::AppError;

use crate::oauth_provider::{GitHubOAuth2, GoogleOAuth2, OAuth2Provider};

pub(super) const DEFAULT_OAUTH2_REDIRECT_URI: &str = "http://localhost:1420/auth/callback";

pub(super) fn oauth2_provider_from_env(
    provider: &str,
    redirect_uri: &str,
) -> Result<Box<dyn OAuth2Provider>, AppError> {
    match provider {
        "github" => oauth2_provider_from_credentials(
            "github",
            required_env("GITHUB_CLIENT_ID")?,
            required_env("GITHUB_CLIENT_SECRET")?,
            redirect_uri.to_string(),
        ),
        "google" => oauth2_provider_from_credentials(
            "google",
            required_env("GOOGLE_CLIENT_ID")?,
            required_env("GOOGLE_CLIENT_SECRET")?,
            redirect_uri.to_string(),
        ),
        "" => Err(AppError::InvalidArgument("OAuth2 提供商不能为空".into())),
        other => Err(AppError::InvalidArgument(format!(
            "不支持的 OAuth2 提供商: {other}"
        ))),
    }
}

pub(super) fn oauth2_provider_from_credentials(
    provider: &str,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
) -> Result<Box<dyn OAuth2Provider>, AppError> {
    match provider {
        "github" => Ok(Box::new(GitHubOAuth2::new(
            client_id,
            client_secret,
            redirect_uri,
        ))),
        "google" => Ok(Box::new(GoogleOAuth2::new(
            client_id,
            client_secret,
            redirect_uri,
        ))),
        other => Err(AppError::InvalidArgument(format!(
            "不支持的 OAuth2 提供商: {other}"
        ))),
    }
}

fn required_env(name: &str) -> Result<String, AppError> {
    let value =
        std::env::var(name).map_err(|_| AppError::ConfigError(format!("{name} 不能为空")))?;
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::ConfigError(format!("{name} 不能为空")));
    }
    Ok(value.to_string())
}
