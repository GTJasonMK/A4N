mod parsing;
mod requests;

use super::contracts::{OAuth2Provider, OAuth2UserInfo};
use a4n_error::AppError;
use async_trait::async_trait;

pub struct GitHubOAuth2 {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    http_client: reqwest::Client,
}

impl GitHubOAuth2 {
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
impl OAuth2Provider for GitHubOAuth2 {
    fn authorize_url(&self, state: &str) -> String {
        format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=read:user%20user:email&state={state}",
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
        let (user_result, emails_result) = futures::join!(
            requests::fetch_user_info(&self.http_client, access_token),
            requests::fetch_emails(&self.http_client, access_token),
        );
        let user = user_result?;
        let emails = emails_result?;

        Ok(OAuth2UserInfo {
            provider_id: parsing::github_provider_id(&user)?,
            provider: "github".to_string(),
            username: user["login"].as_str().map(|s| s.to_string()),
            email: Some(parsing::github_verified_primary_email(&emails)?),
            avatar_url: user["avatar_url"].as_str().map(|s| s.to_string()),
            display_name: user["name"].as_str().map(|s| s.to_string()),
        })
    }
}
