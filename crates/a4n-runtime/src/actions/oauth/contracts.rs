use serde::Deserialize;

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
