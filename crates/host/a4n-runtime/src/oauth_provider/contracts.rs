mod helpers;
pub(super) mod urlencoding;

use a4n_error::AppError;
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

pub(super) use helpers::{oauth_token_error, required_non_empty_string};
