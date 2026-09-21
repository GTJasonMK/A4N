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

/// Authentication read boundary used by Hosts and Runtime.
///
/// The port exposes only host-safe projections. ORM users/sessions and their
/// password or refresh-token fields stay inside the authentication owner.
#[async_trait::async_trait]
pub trait AuthReadApplicationPort: Send + Sync {
    async fn current_user(&self, user_id: i64) -> Result<UserSummary, a4n_error::AppError>;

    async fn list_sessions(&self, user_id: i64)
    -> Result<Vec<SessionSummary>, a4n_error::AppError>;

    async fn verify_session_refresh_token(
        &self,
        user_id: i64,
        session_id: &str,
        refresh_token: &str,
    ) -> Result<(), a4n_error::AppError>;

    async fn verify_active_session(
        &self,
        user_id: i64,
        session_id: &str,
    ) -> Result<(), a4n_error::AppError>;
}

/// Authentication mutation boundary used by Hosts and Runtime.
#[async_trait::async_trait]
pub trait AuthMutationApplicationPort: Send + Sync {
    async fn require_existing_user_or_bootstrap_admin(
        &self,
        bootstrap: Option<(&str, &str, &str)>,
    ) -> Result<(), a4n_error::AppError>;

    async fn register_user(&self, input: RegisterInput)
    -> Result<UserSummary, a4n_error::AppError>;

    async fn login_user(&self, input: LoginInput) -> Result<UserSummary, a4n_error::AppError>;

    async fn create_session(
        &self,
        user_id: i64,
        refresh_token: &str,
        user_agent: Option<&str>,
    ) -> Result<SessionSummary, a4n_error::AppError>;

    async fn revoke_session(
        &self,
        user_id: i64,
        session_id: &str,
    ) -> Result<(), a4n_error::AppError>;

    async fn update_profile(
        &self,
        user_id: i64,
        input: UpdateProfileInput,
    ) -> Result<UserSummary, a4n_error::AppError>;

    async fn change_password(
        &self,
        user_id: i64,
        input: ChangePasswordInput,
    ) -> Result<(), a4n_error::AppError>;

    async fn rotate_session_refresh_token(
        &self,
        user_id: i64,
        session_id: &str,
        old_refresh_token: &str,
        new_refresh_token: &str,
    ) -> Result<(), a4n_error::AppError>;

    async fn login_with_oauth2(
        &self,
        provider: &str,
        provider_id: &str,
        email: Option<&str>,
        username_hint: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<UserSummary, a4n_error::AppError>;
}

/// Session identifiers are validated at the contract boundary so every Host
/// uses the same failure semantics without importing the orchestrator.
pub fn required_session_user_id(user_id: i64, operation: &str) -> Result<i64, a4n_error::AppError> {
    if user_id > 0 {
        Ok(user_id)
    } else {
        Err(a4n_error::AppError::InvalidArgument(format!(
            "{operation}缺少有效用户 ID"
        )))
    }
}

pub fn required_session_id(
    session_id: &str,
    operation: &str,
) -> Result<String, a4n_error::AppError> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        Err(a4n_error::AppError::InvalidArgument(format!(
            "{operation}缺少会话 ID"
        )))
    } else {
        Ok(session_id.to_string())
    }
}

pub fn required_authorization_code(code: &str) -> Result<&str, a4n_error::AppError> {
    let code = code.trim();
    if code.is_empty() {
        Err(a4n_error::AppError::InvalidArgument(
            "OAuth2 授权码不能为空".into(),
        ))
    } else {
        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LoginInput, RevokeSessionInput, UpdateProfileInput, required_authorization_code,
        required_session_id, required_session_user_id,
    };
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

    #[test]
    fn session_rules_reject_missing_identity() {
        assert!(required_session_user_id(0, "列出会话").is_err());
        assert_eq!(required_session_user_id(2, "列出会话").unwrap(), 2);
        assert!(required_session_id("  ", "撤销会话").is_err());
        assert_eq!(required_session_id(" sid ", "撤销会话").unwrap(), "sid");
    }

    #[test]
    fn authorization_code_rule_trims_and_rejects_blank() {
        assert!(required_authorization_code(" \n\t ").is_err());
        assert_eq!(required_authorization_code("  code  ").unwrap(), "code");
    }
}
