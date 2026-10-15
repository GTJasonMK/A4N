use super::{ProjectAccessRole, ProjectAccessScope};
use a4n_domain::project::ProjectKind;
use a4n_domain::project::ProjectSummaryRecord;
use chrono::NaiveDateTime;
use sea_orm::FromQueryResult;

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
    pub items: Vec<AccessibleProjectRecord>,
    pub total: u64,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct AccessibleProjectRecord {
    pub id: i64,
    pub content_revision: i64,
    pub user_id: i64,
    pub title: String,
    pub description: String,
    pub project_type: String,
    pub project_kind: String,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub source_sync_status: String,
    pub status: String,
    pub lifecycle_gate: String,
    pub target_chapters: i32,
    pub current_chapter: i32,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, FromQueryResult)]
pub(in crate::collaboration) struct ProjectAccessRow {
    pub id: i64,
    pub user_id: i64,
    pub project_kind: String,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub(in crate::collaboration) struct UserDisplayRow {
    pub id: i64,
    pub username: String,
    pub email: String,
}

#[derive(Debug, Clone, FromQueryResult)]
pub(in crate::collaboration) struct UserAccessRow {
    pub role: String,
    pub is_active: bool,
}

impl AccessibleProjectRecord {
    pub fn into_summary_record(self) -> ProjectSummaryRecord {
        ProjectSummaryRecord {
            id: self.id,
            content_revision: self.content_revision,
            user_id: self.user_id,
            title: self.title,
            description: self.description,
            project_type: self.project_type,
            project_kind: self.project_kind,
            root_project_id: self.root_project_id,
            source_snapshot_id: self.source_snapshot_id,
            source_sync_status: self.source_sync_status,
            status: self.status,
            lifecycle_gate: self.lifecycle_gate,
            target_chapters: self.target_chapters,
            current_chapter: self.current_chapter,
            total_chapters: self.total_chapters,
            chapters_per_part: self.chapters_per_part,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
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
    pub fn from_project(project: &ProjectSummaryRecord) -> Self {
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
