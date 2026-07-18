use a4n_core::error::AppError;
use serde_json::Value;

use super::super::super::oauth_token_error;

pub(in crate::oauth_provider::google) async fn exchange_token(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<String, AppError> {
    let response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!("Google OAuth 请求失败: {error}")))?;

    let body: Value = response.json().await.map_err(|error| {
        AppError::Internal(anyhow::anyhow!("Google OAuth 响应解析失败: {error}"))
    })?;

    body["access_token"]
        .as_str()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| oauth_token_error("Google", &body))
}
