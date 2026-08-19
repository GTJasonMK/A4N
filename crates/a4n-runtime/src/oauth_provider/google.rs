mod parsing;
mod requests;

use super::contracts::{OAuth2Provider, OAuth2UserInfo};
use a4n_core::error::AppError;
use async_trait::async_trait;

/// Google OAuth2 提供者
pub struct GoogleOAuth2 {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    http_client: reqwest::Client,
}

impl GoogleOAuth2 {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri,
            http_client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl OAuth2Provider for GoogleOAuth2 {
    fn authorize_url(&self, state: &str) -> String {
        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20profile%20email&state={state}",
            self.client_id,
            super::contracts::urlencoding::encode(&self.redirect_uri),
        )
    }

    async fn exchange_token(&self, code: &str) -> Result<String, AppError> {
        requests::exchange_token(
            &self.http_client,
            &self.client_id,
            &self.client_secret,
            code,
            &self.redirect_uri,
        )
        .await
    }

    async fn get_user_info(&self, access_token: &str) -> Result<OAuth2UserInfo, AppError> {
        let user = requests::fetch_user_info(&self.http_client, access_token).await?;
        parsing::google_oauth_user_info(&user)
    }
}
