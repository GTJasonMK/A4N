use super::scope::ProjectAccessScope;
use a4n_core::models::project::novel_project;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibleProjectListInput {
    pub user_id: i64,
    pub offset: u64,
    pub limit: u64,
    pub project_kind: Option<String>,
    pub root_project_id: Option<i64>,
    pub access_scope: ProjectAccessScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibleProjectsPage {
    pub items: Vec<novel_project::Model>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollaborationRootProject {
    pub id: i64,
    pub user_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectAccessInfo {
    pub collaboration_root_project_id: i64,
    pub owner_user_id: i64,
    pub owner_username: Option<String>,
    pub my_role: Option<String>,
    pub member_count: u64,
    pub pending_invite_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMemberSummary {
    pub project_id: i64,
    pub user_id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub is_owner: bool,
    pub invited_by_user_id: Option<i64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInviteSummary {
    pub id: i64,
    pub project_id: i64,
    pub project_title: Option<String>,
    pub inviter_user_id: i64,
    pub inviter_username: Option<String>,
    pub invitee_user_id: i64,
    pub invitee_username: Option<String>,
    pub invitee_email: Option<String>,
    pub role: String,
    pub status: String,
    pub expires_at: String,
    pub accepted_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
