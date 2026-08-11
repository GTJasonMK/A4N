use a4n_domain::project::ProjectStatus;
use chrono::NaiveDateTime;
use sea_orm::DbErr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStatusHistoryInput {
    pub project_id: i64,
    pub from_status: ProjectStatus,
    pub to_status: ProjectStatus,
    pub reason: Option<String>,
    pub changed_by: Option<String>,
    pub changed_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateProjectStatusRecordInput {
    pub to_status: ProjectStatus,
    pub history_from_status: ProjectStatus,
    pub history_reason: String,
    pub changed_by: Option<String>,
    pub changed_at: NaiveDateTime,
    pub clear_draft_fields: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectStatusWriteError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("项目不存在: {project_id}")]
    ProjectNotFound { project_id: i64 },
    #[error("项目状态已变化，请刷新后重试: expected={expected}, actual={actual}")]
    StatusChanged { expected: String, actual: String },
    #[error("{source}")]
    Database {
        #[source]
        source: DbErr,
    },
}

impl From<DbErr> for ProjectStatusWriteError {
    fn from(source: DbErr) -> Self {
        Self::Database { source }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectStatusArtifacts {
    pub has_blueprint: bool,
    pub has_part_outline: bool,
    pub has_chapter_outline: bool,
    pub has_chapter_content: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectStatusReadError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{operation}失败: {source}")]
    Database {
        operation: &'static str,
        #[source]
        source: DbErr,
    },
}

impl ProjectStatusReadError {
    pub(super) fn database(operation: &'static str, source: DbErr) -> Self {
        Self::Database { operation, source }
    }
}
