use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::serde_update::{deserialize_non_null_update_field, deserialize_nullable_update_field};

pub use a4n_domain::auth::UserSummary;

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
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_non_null_update_field"
    )]
    #[schemars(with = "String")]
    pub email: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_nullable_update_field"
    )]
    #[schemars(with = "Option<String>")]
    pub avatar_url: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_cli_enabled: Option<bool>,
}

impl UpdateProfileInput {
    pub fn is_empty(&self) -> bool {
        self.email.is_none() && self.avatar_url.is_none() && self.page_cli_enabled.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChangePasswordInput {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RevokeSessionInput {
    pub session_id: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OAuth2AuthorizeInput {
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OAuth2CallbackInput {
    pub provider: String,
    pub code: String,
}

#[cfg(test)]
mod tests {
    use super::{LoginInput, RevokeSessionInput, UpdateProfileInput};
    use serde_json::json;

    #[test]
    fn update_profile_has_one_strict_patch_contract() {
        let missing: UpdateProfileInput = serde_json::from_value(json!({})).unwrap();
        assert!(missing.is_empty());
        assert_eq!(missing.email, None);
        assert_eq!(missing.avatar_url, None);

        assert!(serde_json::from_value::<UpdateProfileInput>(json!({"email": null})).is_err());

        let cleared_avatar: UpdateProfileInput =
            serde_json::from_value(json!({"avatar_url": null})).unwrap();
        assert!(!cleared_avatar.is_empty());
        assert_eq!(cleared_avatar.avatar_url, Some(None));

        let value: UpdateProfileInput = serde_json::from_value(json!({
            "email": "alice@example.com",
            "avatar_url": "https://cdn/avatar.png"
        }))
        .unwrap();
        assert_eq!(value.email, Some("alice@example.com".to_string()));
        assert_eq!(
            value.avatar_url,
            Some(Some("https://cdn/avatar.png".to_string()))
        );

        assert!(
            serde_json::from_value::<UpdateProfileInput>(json!({"legacy_email": "x"})).is_err()
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

    #[test]
    fn revoke_session_input_is_strict() {
        let input: RevokeSessionInput =
            serde_json::from_value(json!({ "session_id": "session-1" })).unwrap();
        assert_eq!(input.session_id, "session-1");
        assert!(
            serde_json::from_value::<RevokeSessionInput>(json!({
                "session_id": "session-1",
                "user_id": 7
            }))
            .is_err()
        );
    }
}
