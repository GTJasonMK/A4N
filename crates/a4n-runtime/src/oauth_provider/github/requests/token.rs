use a4n_error::AppError;
use serde_json::Value;

use crate::oauth_provider::contracts::oauth_token_error;

pub(in crate::oauth_provider::github) async fn exchange_token(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<String, AppError> {
    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "client_id": client_id,
            "client_secret": client_secret,
            "code": code,
            "redirect_uri": redirect_uri,
        }))
        .send()
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!("GitHub OAuth 请求失败: {error}")))?;
    let body: Value = response.json().await.map_err(|error| {
        AppError::Internal(anyhow::anyhow!("GitHub OAuth 响应解析失败: {error}"))
    })?;
    body["access_token"]
        .as_str()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| oauth_token_error("GitHub", &body))
}
