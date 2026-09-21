use a4n_config::required_non_empty_env_string;
use a4n_error::AppError;

use crate::oauth_provider::{GitHubOAuth2, GoogleOAuth2, OAuth2Provider};

pub(super) const DEFAULT_OAUTH2_REDIRECT_URI: &str = "http://localhost:1420/auth/callback";

pub(super) fn oauth2_provider_from_env(
    provider: &str,
    redirect_uri: &str,
) -> Result<Box<dyn OAuth2Provider>, AppError> {
    match provider {
        "github" => Ok(Box::new(GitHubOAuth2::new(
            required_non_empty_env_string("GITHUB_CLIENT_ID")?,
            required_non_empty_env_string("GITHUB_CLIENT_SECRET")?,
            redirect_uri.to_string(),
        ))),
        "google" => Ok(Box::new(GoogleOAuth2::new(
            required_non_empty_env_string("GOOGLE_CLIENT_ID")?,
            required_non_empty_env_string("GOOGLE_CLIENT_SECRET")?,
            redirect_uri.to_string(),
        ))),
        "" => Err(AppError::InvalidArgument("OAuth2 提供商不能为空".into())),
        other => Err(AppError::InvalidArgument(format!(
            "不支持的 OAuth2 提供商: {other}"
        ))),
    }
}
