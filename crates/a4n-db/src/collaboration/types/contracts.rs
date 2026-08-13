use super::{ProjectAccessRole, ProjectAccessScope};
use a4n_core::models::project::novel_project;
use a4n_domain::project::ProjectKind;

#[derive(Debug, Clone)]
pub struct AccessibleProjectListInput {
    pub user_id: i64,
    pub offset: u64,
    pub limit: u64,
    pub project_kind: Option<ProjectKind>,
    pub root_project_id: Option<i64>,
    pub access_scope: ProjectAccessScope,
}

#[derive(Debug, Clone)]
pub struct AccessibleProjectsPage {
    pub items: Vec<novel_project::Model>,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollaborationRootProject {
    pub id: i64,
    pub user_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectAccessInfoQuery {
    pub project_id: i64,
    pub root_project_id: Option<i64>,
}

impl ProjectAccessInfoQuery {
    pub fn from_project(project: &novel_project::Model) -> Self {
        Self {
            project_id: project.id,
            root_project_id: project.root_project_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectAccessInfo {
    pub collaboration_root_project_id: i64,
    pub owner_username: String,
    pub my_role: ProjectAccessRole,
    pub member_count: u64,
    pub pending_invite_count: Option<u64>,
}
