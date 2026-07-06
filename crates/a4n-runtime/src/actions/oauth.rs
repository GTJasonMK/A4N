use a4n_core::error::AppError;
use a4n_core::models::user::user;
use a4n_services::auth::AuthService;
use a4n_services::auth::oauth::{GitHubOAuth2, GoogleOAuth2, OAuth2Provider};
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_OAUTH2_REDIRECT_URI: &str = "http://localhost:1420/auth/callback";

#[derive(Debug, Clone, Deserialize)]
pub struct OAuth2AuthorizeInput {
    /// OAuth2 提供商：github 或 google
    pub provider: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OAuth2CallbackInput {
    /// OAuth2 提供商：github 或 google
    pub provider: String,
    /// 授权码
    pub code: String,
}

pub fn oauth2_authorize_url(input: OAuth2AuthorizeInput) -> Result<String, AppError> {
    let provider = oauth2_provider_from_env(input.provider.trim(), DEFAULT_OAUTH2_REDIRECT_URI)?;
    Ok(provider.authorize_url(&oauth2_state()))
}

pub async fn oauth2_login_user(
    auth: &AuthService,
    input: OAuth2CallbackInput,
) -> Result<user::Model, AppError> {
    let code = required_authorization_code(&input.code)?;
    let provider = oauth2_provider_from_env(input.provider.trim(), DEFAULT_OAUTH2_REDIRECT_URI)?;
    let token = provider.exchange_token(code).await?;
    let user_info = provider.get_user_info(&token).await?;

    auth.login_with_oauth2(
        &user_info.provider,
        &user_info.provider_id,
        user_info.email.as_deref(),
        user_info.username.as_deref(),
        user_info.avatar_url.as_deref(),
    )
    .await
}

fn oauth2_provider_from_env(
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

fn oauth2_provider_from_credentials(
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

fn required_authorization_code(code: &str) -> Result<&str, AppError> {
    let code = code.trim();
    if code.is_empty() {
        Err(AppError::InvalidArgument("OAuth2 授权码不能为空".into()))
    } else {
        Ok(code)
    }
}

fn oauth2_state() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_url_uses_provider_specific_endpoint_and_redirect_uri() {
        let provider = oauth2_provider_from_credentials(
            "github",
            "github-client".to_string(),
            "github-secret".to_string(),
            "http://localhost:1420/auth/callback".to_string(),
        )
        .expect("provider");

        let url = provider.authorize_url("state-1");

        assert!(url.starts_with("https://github.com/login/oauth/authorize?"));
        assert!(url.contains("client_id=github-client"));
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A1420%2Fauth%2Fcallback"));
        assert!(url.contains("state=state-1"));
    }

    #[test]
    fn unsupported_provider_is_rejected_before_env_lookup() {
        let err = match oauth2_provider_from_env("wechat", DEFAULT_OAUTH2_REDIRECT_URI) {
            Ok(_) => panic!("unsupported provider should fail"),
            Err(err) => err,
        };

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn authorization_code_must_not_be_blank() {
        let err = required_authorization_code(" \n\t ").expect_err("blank code should fail");

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }
}
