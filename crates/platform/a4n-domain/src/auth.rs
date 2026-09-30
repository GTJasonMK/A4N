use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const ROLE_ADMIN: &str = "admin";
pub const ROLE_USER: &str = "user";
pub const ROLE_READ_ONLY: &str = "read_only";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
    Admin,
    User,
    ReadOnly,
}

impl UserRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => ROLE_ADMIN,
            Self::User => ROLE_USER,
            Self::ReadOnly => ROLE_READ_ONLY,
        }
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Admin => "管理员",
            Self::User => "用户",
            Self::ReadOnly => "只读",
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("未知用户角色: {0}")]
pub struct UserRoleParseError(pub String);

pub fn parse_user_role(value: &str) -> Result<UserRole, UserRoleParseError> {
    match value {
        ROLE_ADMIN => Ok(UserRole::Admin),
        ROLE_USER => Ok(UserRole::User),
        ROLE_READ_ONLY => Ok(UserRole::ReadOnly),
        other => Err(UserRoleParseError(other.to_string())),
    }
}

pub fn validate_user_role(value: &str) -> Result<(), UserRoleParseError> {
    parse_user_role(value).map(|_| ())
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct UserSummary {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub oauth_provider: Option<String>,
    pub oauth_subject: Option<String>,
    pub avatar_url: Option<String>,
    pub page_cli_enabled: bool,
    pub is_active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub last_login_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAuthorizationRecord {
    pub id: i64,
    pub role: String,
    pub is_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_roles_and_rejects_unknown_values() {
        assert_eq!(parse_user_role(ROLE_ADMIN), Ok(UserRole::Admin));
        assert_eq!(parse_user_role(ROLE_USER), Ok(UserRole::User));
        assert_eq!(parse_user_role(ROLE_READ_ONLY), Ok(UserRole::ReadOnly));
        assert_eq!(
            parse_user_role("owner"),
            Err(UserRoleParseError("owner".into()))
        );
    }
}
