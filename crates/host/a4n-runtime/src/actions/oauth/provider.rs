use a4n_config::{optional_non_empty_env_string, required_non_empty_env_string};
use a4n_error::AppError;

use crate::oauth_provider::{GitHubOAuth2, GoogleOAuth2, OAuth2Provider};

#[derive(Clone, Copy)]
enum OAuthProviderKind {
    GitHub,
    Google,
}

impl OAuthProviderKind {
    const ALL: [Self; 2] = [Self::GitHub, Self::Google];

    const fn as_str(self) -> &'static str {
        match self {
            Self::GitHub => "github",
            Self::Google => "google",
        }
    }

    const fn env_keys(self) -> (&'static str, &'static str) {
        match self {
            Self::GitHub => ("GITHUB_CLIENT_ID", "GITHUB_CLIENT_SECRET"),
            Self::Google => ("GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET"),
        }
    }

    fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "github" => Ok(Self::GitHub),
            "google" => Ok(Self::Google),
            "" => Err(AppError::InvalidArgument("OAuth2 提供商不能为空".into())),
            other => Err(AppError::InvalidArgument(format!(
                "不支持的 OAuth2 提供商: {other}"
            ))),
        }
    }
}

pub fn oauth2_available_providers() -> Result<Vec<String>, AppError> {
    let mut available = Vec::new();
    for provider in OAuthProviderKind::ALL {
        let (client_id_env, client_secret_env) = provider.env_keys();
        let client_id = optional_non_empty_env_string(client_id_env)?;
        let client_secret = optional_non_empty_env_string(client_secret_env)?;
        if client_id.is_some() && client_secret.is_some() {
            available.push(provider.as_str().to_string());
        }
    }
    Ok(available)
}

pub(super) fn oauth2_provider_from_env(
    provider: &str,
    redirect_uri: &str,
) -> Result<Box<dyn OAuth2Provider>, AppError> {
    let provider = OAuthProviderKind::parse(provider)?;
    let (client_id_env, client_secret_env) = provider.env_keys();
    let client_id = required_non_empty_env_string(client_id_env)?;
    let client_secret = required_non_empty_env_string(client_secret_env)?;
    match provider {
        OAuthProviderKind::GitHub => Ok(Box::new(GitHubOAuth2::new(
            client_id,
            client_secret,
            redirect_uri.to_string(),
        ))),
        OAuthProviderKind::Google => Ok(Box::new(GoogleOAuth2::new(
            client_id,
            client_secret,
            redirect_uri.to_string(),
        ))),
    }
}
