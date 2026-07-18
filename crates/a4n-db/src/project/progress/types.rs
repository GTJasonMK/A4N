use a4n_core::models::project::novel_project;
use sea_orm::DbErr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectProgressQuery {
    pub project_id: i64,
    pub project_kind: String,
    pub source_mode: String,
    pub status: String,
    pub current_chapter: i32,
    pub target_chapters: i32,
}

impl ProjectProgressQuery {
    pub fn from_project(project: &novel_project::Model) -> Self {
        Self {
            project_id: project.id,
            project_kind: project.project_kind.clone(),
            source_mode: project.source_mode.clone(),
            status: project.status.clone(),
            current_chapter: project.current_chapter,
            target_chapters: project.target_chapters,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectProgressStatus {
    Idle,
    Running,
    Blocked,
    Failed,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectProgressSummary {
    pub kind: String,
    pub stage: String,
    pub label: String,
    pub current: Option<i64>,
    pub total: Option<i64>,
    pub status: ProjectProgressStatus,
    pub detail: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectProgressReadError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("项目缺少进度摘要: {project_id}")]
    SummaryMissing { project_id: i64 },
    #[error("{operation}: {source}")]
    Database {
        operation: &'static str,
        #[source]
        source: DbErr,
    },
}

impl ProjectProgressReadError {
    pub(super) fn database(operation: &'static str, source: DbErr) -> Self {
        Self::Database { operation, source }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn project_progress_read_error_preserves_operation_and_invariant_fields() {
        let missing = ProjectProgressReadError::SummaryMissing { project_id: 17 };
        assert_eq!(missing.to_string(), "项目缺少进度摘要: 17");
        assert!(missing.source().is_none());

        let database = ProjectProgressReadError::database(
            "查询视频提示词失败",
            DbErr::Custom("query failed".to_string()),
        );
        assert!(matches!(
            database,
            ProjectProgressReadError::Database {
                operation: "查询视频提示词失败",
                source: DbErr::Custom(ref message),
            } if message == "query failed"
        ));
        assert!(database.source().is_some());
    }
}
