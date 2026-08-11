use super::*;

#[derive(Debug, Clone)]
pub struct BlueprintRestoreResult {
    pub blueprint: novel_blueprint::Model,
    pub version: String,
    pub status_change: Option<BlueprintRestoreStatusChange>,
    pub restored_at: NaiveDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintRestoreStatusChange {
    pub from: ProjectStatus,
    pub to: ProjectStatus,
    pub changed_at: NaiveDateTime,
}

#[derive(Debug, thiserror::Error)]
pub enum BlueprintRestoreError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    InvalidSnapshot(String),
    #[error("蓝图版本不存在: project_id={project_id}, version={version}")]
    VersionNotFound { project_id: i64, version: String },
    #[error("项目不存在: {project_id}")]
    ProjectNotFound { project_id: i64 },
    #[error("项目状态无效: project_id={project_id}, status={status}")]
    InvalidProjectStatus { project_id: i64, status: String },
    #[error("{source}")]
    CharacterSync {
        #[source]
        source: BlueprintSyncError,
    },
    #[error("{source}")]
    RelationshipSync {
        #[source]
        source: BlueprintSyncError,
    },
    #[error("{source}")]
    StatusWrite {
        #[source]
        source: ProjectStatusWriteError,
    },
    #[error("{source}")]
    Serialization {
        #[source]
        source: serde_json::Error,
    },
    #[error("{operation}: {source}")]
    Database {
        operation: &'static str,
        #[source]
        source: DbErr,
    },
}

impl BlueprintRestoreError {
    pub(super) fn invalid_snapshot(message: impl Into<String>) -> Self {
        Self::InvalidSnapshot(message.into())
    }

    pub(super) fn database(operation: &'static str, source: DbErr) -> Self {
        Self::Database { operation, source }
    }
}
