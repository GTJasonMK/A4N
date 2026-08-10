use std::str::FromStr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::DomainError;

const MEMBER_ROLE_MANAGER: &str = "manager";
const MEMBER_ROLE_EDITOR: &str = "editor";
const MEMBER_ROLE_VIEWER: &str = "viewer";
const ACCESS_ROLE_OWNER: &str = "owner";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectAccessScope {
    All,
    Owned,
    Collaborating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectAccessRole {
    Owner,
    Manager,
    Editor,
    Viewer,
}

impl ProjectAccessRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Owner => ACCESS_ROLE_OWNER,
            Self::Manager => MEMBER_ROLE_MANAGER,
            Self::Editor => MEMBER_ROLE_EDITOR,
            Self::Viewer => MEMBER_ROLE_VIEWER,
        }
    }

    const fn rank(self) -> u8 {
        match self {
            Self::Viewer => 1,
            Self::Editor => 2,
            Self::Manager => 3,
            Self::Owner => 4,
        }
    }

    pub const fn satisfies(self, required: Self) -> bool {
        self.rank() >= required.rank()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectMemberRole {
    Manager,
    Editor,
    Viewer,
}

impl ProjectMemberRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manager => MEMBER_ROLE_MANAGER,
            Self::Editor => MEMBER_ROLE_EDITOR,
            Self::Viewer => MEMBER_ROLE_VIEWER,
        }
    }
}

impl FromStr for ProjectMemberRole {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            MEMBER_ROLE_MANAGER => Ok(Self::Manager),
            MEMBER_ROLE_EDITOR => Ok(Self::Editor),
            MEMBER_ROLE_VIEWER => Ok(Self::Viewer),
            _ => Err(DomainError::InvalidProjectMemberRole {
                value: value.to_string(),
            }),
        }
    }
}

impl From<ProjectMemberRole> for ProjectAccessRole {
    fn from(role: ProjectMemberRole) -> Self {
        match role {
            ProjectMemberRole::Manager => Self::Manager,
            ProjectMemberRole::Editor => Self::Editor,
            ProjectMemberRole::Viewer => Self::Viewer,
        }
    }
}

#[derive(Serialize, JsonSchema)]
pub struct ProjectMemberSummary {
    pub user_id: i64,
    pub username: String,
    pub email: String,
    pub role: ProjectAccessRole,
}

#[derive(Serialize, JsonSchema)]
pub struct ProjectInviteSummary {
    pub id: i64,
    pub project_title: String,
    pub inviter_username: String,
    pub invitee_username: String,
    pub role: ProjectMemberRole,
}
