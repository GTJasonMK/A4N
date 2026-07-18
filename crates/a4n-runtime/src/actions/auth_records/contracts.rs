use super::rules::deserialize_nullable_update_field;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSummary {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub oauth_provider: Option<String>,
    pub oauth_subject: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub last_login_at: Option<String>,
}

impl UserSummary {
    pub fn with_display_name(mut self) -> Self {
        self.display_name = Some(self.username.clone());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub invite_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateProfileInput {
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub email: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub avatar_url: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangePasswordInput {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Copy)]
pub struct BootstrapAdminInput<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub user_id: i64,
    pub user_agent: Option<String>,
    pub is_revoked: bool,
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    pub user: UserSummary,
    pub tokens: TokenPairSummary,
    pub session: SessionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshTokenInput {
    pub refresh_token: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPairSummary {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2LoginResult {
    pub user: UserSummary,
    pub tokens: TokenPairSummary,
    pub session: SessionSummary,
}
