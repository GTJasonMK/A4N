use super::errors::CollaborationDbError;
use serde::{Deserialize, Serialize};

pub(in crate::collaboration) const MEMBER_ROLE_MANAGER: &str = "manager";
pub(in crate::collaboration) const MEMBER_ROLE_EDITOR: &str = "editor";
pub(in crate::collaboration) const MEMBER_ROLE_VIEWER: &str = "viewer";
pub(in crate::collaboration) const ACCESS_ROLE_OWNER: &str = "owner";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectAccessRole {
    Owner,
    Manager,
    Editor,
    Viewer,
}

impl ProjectAccessRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => ACCESS_ROLE_OWNER,
            Self::Manager => MEMBER_ROLE_MANAGER,
            Self::Editor => MEMBER_ROLE_EDITOR,
            Self::Viewer => MEMBER_ROLE_VIEWER,
        }
    }

    pub(in crate::collaboration) fn rank(self) -> u8 {
        match self {
            Self::Viewer => 1,
            Self::Editor => 2,
            Self::Manager => 3,
            Self::Owner => 4,
        }
    }

    pub(in crate::collaboration) fn satisfies(self, required: ProjectAccessRole) -> bool {
        self.rank() >= required.rank()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectMemberRole {
    Manager,
    Editor,
    Viewer,
}

impl ProjectMemberRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manager => MEMBER_ROLE_MANAGER,
            Self::Editor => MEMBER_ROLE_EDITOR,
            Self::Viewer => MEMBER_ROLE_VIEWER,
        }
    }
}

impl std::fmt::Display for ProjectMemberRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub fn parse_project_member_role(value: &str) -> Result<ProjectMemberRole, CollaborationDbError> {
    match value.trim() {
        MEMBER_ROLE_MANAGER => Ok(ProjectMemberRole::Manager),
        MEMBER_ROLE_EDITOR => Ok(ProjectMemberRole::Editor),
        MEMBER_ROLE_VIEWER => Ok(ProjectMemberRole::Viewer),
        other => Err(CollaborationDbError::InvalidArgument(format!(
            "未知项目协作角色: {other}"
        ))),
    }
}
