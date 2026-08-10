use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::serde_update::deserialize_nullable_update_field;

/// Web/Tauri 用户摘要。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RegisterInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub invite_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateProfileInput {
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub email: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub avatar_url: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChangePasswordInput {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SessionSummary {
    pub id: String,
    pub user_agent: Option<String>,
    pub created_at: String,
    pub expires_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TokenPairSummary {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AuthResult {
    pub user: UserSummary,
    pub tokens: TokenPairSummary,
    pub session: SessionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RefreshTokenInput {
    pub refresh_token: String,
    pub session_id: String,
}

#[cfg(test)]
mod tests {
    use super::{LoginInput, UpdateProfileInput};
    use serde_json::json;

    #[test]
    fn update_profile_distinguishes_missing_and_null() {
        let missing: UpdateProfileInput = serde_json::from_value(json!({})).unwrap();
        assert_eq!(missing.email, None);
        assert_eq!(missing.avatar_url, None);

        let null: UpdateProfileInput =
            serde_json::from_value(json!({"email": null, "avatar_url": null})).unwrap();
        assert_eq!(null.email, Some(None));
        assert_eq!(null.avatar_url, Some(None));

        let value: UpdateProfileInput = serde_json::from_value(json!({
            "email": "alice@example.com",
            "avatar_url": "https://cdn/avatar.png"
        }))
        .unwrap();
        assert_eq!(value.email, Some(Some("alice@example.com".to_string())));
        assert_eq!(
            value.avatar_url,
            Some(Some("https://cdn/avatar.png".to_string()))
        );
    }

    #[test]
    fn login_rejects_unknown_fields() {
        assert!(
            serde_json::from_value::<LoginInput>(json!({
                "username": "u",
                "password": "p",
                "extra": true
            }))
            .is_err()
        );
    }
}
