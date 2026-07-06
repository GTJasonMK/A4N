//! OAuth2 社交登录集成
//!
//! 支持 GitHub 和 Google 的 OAuth2 授权码流程。

use a4n_core::error::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// OAuth2 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2UserInfo {
    /// 外部平台用户 ID
    pub provider_id: String,
    /// 提供者名称（github / google）
    pub provider: String,
    /// 用户名
    pub username: Option<String>,
    /// 邮箱
    pub email: Option<String>,
    /// 头像 URL
    pub avatar_url: Option<String>,
    /// 显示名称
    pub display_name: Option<String>,
}

/// OAuth2 提供者 trait
#[async_trait]
pub trait OAuth2Provider: Send + Sync {
    /// 生成授权 URL（用户浏览器跳转到此地址授权）
    fn authorize_url(&self, state: &str) -> String;

    /// 用授权码交换访问令牌
    async fn exchange_token(&self, code: &str) -> Result<String, AppError>;

    /// 获取用户信息
    async fn get_user_info(&self, access_token: &str) -> Result<OAuth2UserInfo, AppError>;
}

/// GitHub OAuth2 提供者
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
            urlencoding::encode(&self.redirect_uri),
        )
    }

    async fn exchange_token(&self, code: &str) -> Result<String, AppError> {
        let response = self
            .http_client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .json(&serde_json::json!({
                "client_id": self.client_id,
                "client_secret": self.client_secret,
                "code": code,
                "redirect_uri": self.redirect_uri,
            }))
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("GitHub OAuth 请求失败: {e}")))?;

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("GitHub OAuth 响应解析失败: {e}")))?;

        body["access_token"]
            .as_str()
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(ToString::to_string)
            .ok_or_else(|| oauth_token_error("GitHub", &body))
    }

    async fn get_user_info(&self, access_token: &str) -> Result<OAuth2UserInfo, AppError> {
        let user_response = self
            .http_client
            .get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("User-Agent", "A4N-App")
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("GitHub 用户信息请求失败: {e}")))?;

        let user: serde_json::Value = user_response
            .json()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("GitHub 用户信息解析失败: {e}")))?;

        let email = Some(self.fetch_primary_email(access_token).await?);

        Ok(OAuth2UserInfo {
            provider_id: github_provider_id(&user)?,
            provider: "github".to_string(),
            username: user["login"].as_str().map(|s| s.to_string()),
            email,
            avatar_url: user["avatar_url"].as_str().map(|s| s.to_string()),
            display_name: user["name"].as_str().map(|s| s.to_string()),
        })
    }
}

impl GitHubOAuth2 {
    async fn fetch_primary_email(&self, access_token: &str) -> Result<String, AppError> {
        let response = self
            .http_client
            .get("https://api.github.com/user/emails")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("User-Agent", "A4N-App")
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("GitHub 邮箱请求失败: {e}")))?;

        let emails: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("GitHub 邮箱解析失败: {e}")))?;

        github_verified_primary_email(&emails)
    }
}

fn oauth_token_error(provider: &str, body: &serde_json::Value) -> AppError {
    if let Some(error) = body["error_description"]
        .as_str()
        .map(str::trim)
        .filter(|error| !error.is_empty())
    {
        return AppError::Forbidden(format!("{provider} 令牌交换失败: {error}"));
    }

    AppError::Forbidden(format!(
        "{provider} 令牌交换失败: 响应缺少 access_token，body={body}"
    ))
}

fn github_provider_id(user: &serde_json::Value) -> Result<String, AppError> {
    user["id"]
        .as_i64()
        .map(|id| id.to_string())
        .filter(|id| !id.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("GitHub 用户信息缺少数字字段 id".into()))
}

fn github_verified_primary_email(emails: &[serde_json::Value]) -> Result<String, AppError> {
    emails
        .iter()
        .find(|email| {
            email["primary"].as_bool() == Some(true) && email["verified"].as_bool() == Some(true)
        })
        .and_then(|email| email["email"].as_str())
        .map(str::trim)
        .filter(|email| !email.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| AppError::NotFound("未找到 GitHub 已验证主邮箱".into()))
}

/// Google OAuth2 提供者
pub struct GoogleOAuth2 {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    http_client: reqwest::Client,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn github_email_requires_verified_primary_address() {
        let emails = vec![
            json!({"email": "secondary@example.com", "primary": false, "verified": true}),
            json!({"email": "primary@example.com", "primary": true, "verified": true}),
        ];

        let email = github_verified_primary_email(&emails).expect("verified primary email");

        assert_eq!(email, "primary@example.com");
    }

    #[test]
    fn github_email_rejects_unverified_or_non_primary_addresses() {
        let emails = vec![
            json!({"email": "unverified@example.com", "primary": true, "verified": false}),
            json!({"email": "secondary@example.com", "primary": false, "verified": true}),
        ];

        let err = github_verified_primary_email(&emails).expect_err("email should be rejected");

        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn oauth_token_error_uses_response_body_when_description_is_missing() {
        let err = oauth_token_error("GitHub", &json!({"error": "bad_verification_code"}));
        let message = err.to_string();

        assert!(message.contains("access_token"));
        assert!(message.contains("bad_verification_code"));
    }

    #[test]
    fn github_provider_id_requires_numeric_id() {
        let err = github_provider_id(&json!({"login": "octo"}))
            .expect_err("missing id should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn google_user_info_requires_verified_email() {
        let err = google_oauth_user_info(&json!({
            "sub": "google-1",
            "email": "victim@example.com",
            "email_verified": false
        }))
        .expect_err("unverified Google email must not be trusted for account linking");

        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[test]
    fn google_user_info_rejects_missing_email_verified() {
        let err = google_oauth_user_info(&json!({
            "sub": "google-1",
            "email": "victim@example.com"
        }))
        .expect_err("Google email_verified must be present and true");

        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[test]
    fn google_user_info_uses_verified_email_as_identity() {
        let info = google_oauth_user_info(&json!({
            "sub": "google-1",
            "email": " user@example.com ",
            "email_verified": true,
            "picture": "https://cdn.example.com/avatar.png",
            "name": "Alice"
        }))
        .expect("verified Google email should be accepted");

        assert_eq!(info.provider, "google");
        assert_eq!(info.provider_id, "google-1");
        assert_eq!(info.email.as_deref(), Some("user@example.com"));
        assert_eq!(info.username.as_deref(), Some("user@example.com"));
    }
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
            urlencoding::encode(&self.redirect_uri),
        )
    }

    async fn exchange_token(&self, code: &str) -> Result<String, AppError> {
        let response = self
            .http_client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("code", code),
                ("grant_type", "authorization_code"),
                ("redirect_uri", self.redirect_uri.as_str()),
            ])
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Google OAuth 请求失败: {e}")))?;

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Google OAuth 响应解析失败: {e}")))?;

        body["access_token"]
            .as_str()
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(ToString::to_string)
            .ok_or_else(|| oauth_token_error("Google", &body))
    }

    async fn get_user_info(&self, access_token: &str) -> Result<OAuth2UserInfo, AppError> {
        let response = self
            .http_client
            .get("https://openidconnect.googleapis.com/v1/userinfo")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Google 用户信息请求失败: {e}")))?;

        let user: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Google 用户信息解析失败: {e}")))?;

        google_oauth_user_info(&user)
    }
}

fn google_oauth_user_info(user: &serde_json::Value) -> Result<OAuth2UserInfo, AppError> {
    let email = verified_google_email(user)?;
    Ok(OAuth2UserInfo {
        provider_id: required_non_empty_string(user, "sub", "Google 用户信息")?.to_string(),
        provider: "google".to_string(),
        username: Some(email.clone()),
        email: Some(email),
        avatar_url: user["picture"].as_str().map(|s| s.to_string()),
        display_name: user["name"].as_str().map(|s| s.to_string()),
    })
}

fn verified_google_email(user: &serde_json::Value) -> Result<String, AppError> {
    let email = required_non_empty_string(user, "email", "Google 用户信息")?.to_string();
    if user["email_verified"].as_bool() != Some(true) {
        return Err(AppError::Forbidden("Google 邮箱未验证".into()));
    }
    Ok(email)
}

fn required_non_empty_string<'a>(
    value: &'a serde_json::Value,
    field: &str,
    context: &str,
) -> Result<&'a str, AppError> {
    value[field]
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}缺少字符串字段 {field}")))
}

/// URL 编码（简单实现，避免引入额外依赖）
mod urlencoding {
    pub(super) fn encode(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for byte in s.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    result.push(byte as char);
                }
                _ => {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
        result
    }
}
