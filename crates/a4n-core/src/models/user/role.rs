use crate::error::AppError;

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

pub fn parse_user_role(value: &str) -> Result<UserRole, AppError> {
    match value {
        ROLE_ADMIN => Ok(UserRole::Admin),
        ROLE_USER => Ok(UserRole::User),
        ROLE_READ_ONLY => Ok(UserRole::ReadOnly),
        other => Err(AppError::InvalidArgument(format!("未知用户角色: {other}"))),
    }
}

pub fn validate_user_role(value: &str) -> Result<(), AppError> {
    parse_user_role(value).map(|_| ())
}
