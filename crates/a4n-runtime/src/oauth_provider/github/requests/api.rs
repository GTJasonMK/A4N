use a4n_core::error::AppError;
use serde::de::DeserializeOwned;
use serde_json::Value;

async fn fetch_github_api_json<T>(
    client: &reqwest::Client,
    access_token: &str,
    url: &str,
    label: &str,
) -> Result<T, AppError>
where
    T: DeserializeOwned,
{
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("User-Agent", "A4N-App")
        .send()
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!("GitHub {label}请求失败: {error}")))?;
    response
        .json()
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!("GitHub {label}解析失败: {error}")))
}

pub(in crate::oauth_provider::github) async fn fetch_user_info(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<Value, AppError> {
    fetch_github_api_json(
        client,
        access_token,
        "https://api.github.com/user",
        "用户信息",
    )
    .await
}

pub(in crate::oauth_provider::github) async fn fetch_emails(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<Vec<Value>, AppError> {
    fetch_github_api_json(
        client,
        access_token,
        "https://api.github.com/user/emails",
        "邮箱",
    )
    .await
}
