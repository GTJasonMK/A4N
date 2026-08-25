use a4n_core::models::project::novel_project;
use sea_orm::{DbErr, FromQueryResult};

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectFamilyMetadataQuery {
    pub project_id: i64,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
}

impl ProjectFamilyMetadataQuery {
    pub fn from_project(project: &novel_project::Model) -> Self {
        Self {
            project_id: project.id,
            root_project_id: project.root_project_id,
            source_snapshot_id: project.source_snapshot_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectFamilyMetadata {
    pub root_project_title: Option<String>,
    pub derived_chapter_scope: Option<serde_json::Value>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub(in crate::project::family_metadata) struct ProjectFamilyRootRow {
    pub id: i64,
    pub title: String,
}

#[derive(Debug, Clone, FromQueryResult)]
pub(in crate::project::family_metadata) struct ProjectFamilySnapshotRow {
    pub id: i64,
    pub chapter_scope_json: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectFamilyMetadataReadError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("派生项目引用的根文本项目不存在: {root_project_id}")]
    RootProjectMissing { root_project_id: i64 },
    #[error("派生项目引用的源文本快照不存在: {source_snapshot_id}")]
    SourceSnapshotMissing { source_snapshot_id: i64 },
    #[error("{operation}: {source}")]
    Database {
        operation: &'static str,
        #[source]
        source: DbErr,
    },
}

impl ProjectFamilyMetadataReadError {
    pub(in crate::project::family_metadata) fn database(
        operation: &'static str,
        source: DbErr,
    ) -> Self {
        Self::Database { operation, source }
    }
}
