use a4n_core::error::AppError;
use serde_json::Value;

pub(in crate::oauth_provider::google) async fn fetch_user_info(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<Value, AppError> {
    let response = client
        .get("https://openidconnect.googleapis.com/v1/userinfo")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!("Google 用户信息请求失败: {error}")))?;

    response
        .json()
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!("Google 用户信息解析失败: {error}")))
}
