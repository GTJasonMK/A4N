mod parsing;
mod requests;

use super::contracts::{OAuth2Provider, OAuth2UserInfo};
use a4n_error::AppError;
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
    fn authorize_url(&self, state: &str, code_challenge: &str) -> String {
        let query = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs([
                ("client_id", self.client_id.as_str()),
                ("redirect_uri", self.redirect_uri.as_str()),
                ("response_type", "code"),
                ("scope", "openid profile email"),
                ("state", state),
                ("code_challenge", code_challenge),
                ("code_challenge_method", "S256"),
            ])
            .finish();
        format!("https://accounts.google.com/o/oauth2/v2/auth?{query}")
    }

    async fn exchange_token(&self, code: &str, code_verifier: &str) -> Result<String, AppError> {
        requests::exchange_token(
            &self.http_client,
            &self.client_id,
            &self.client_secret,
            code,
            &self.redirect_uri,
            code_verifier,
        )
        .await
    }

    async fn get_user_info(&self, access_token: &str) -> Result<OAuth2UserInfo, AppError> {
        let user = requests::fetch_user_info(&self.http_client, access_token).await?;
        parsing::google_oauth_user_info(&user)
    }
}
