use super::*;

#[derive(Debug, thiserror::Error)]
pub enum ProjectResetDbError {
    #[error("项目不存在: {project_id}")]
    ProjectNotFound { project_id: i64 },
    #[error("重置生成态不支持项目类型: {project_kind}")]
    UnsupportedProjectKind { project_kind: String },
    #[error("{0}")]
    InvalidArgument(String),
    #[error("状态无效：当前状态 {current}，不允许操作 {operation}")]
    InvalidState { current: String, operation: String },
    #[error("项目状态已变化，请刷新后重试: expected={expected}, actual={actual}")]
    StatusChanged { expected: String, actual: String },
    #[error("{0}")]
    Internal(String),
    #[error("{source}")]
    CharacterProfileDelete {
        #[source]
        source: character::CharacterProfileDeleteError,
    },
    #[error("{source}")]
    Generation {
        #[source]
        source: generation::GenerationDbError,
    },
    #[error("{source}")]
    ProjectCleanup {
        #[source]
        source: crate::project_delete::ProjectDeleteDbError,
    },
    #[error("{source}")]
    ProjectRecordCleanup {
        #[source]
        source: project_db::ProjectCleanupDbError,
    },
    #[error("{source}")]
    ProjectStatusWrite {
        #[source]
        source: project_db::ProjectStatusWriteError,
    },
    #[error("{source}")]
    ProjectMutation {
        #[source]
        source: project_db::ProjectMutationError,
    },
    #[error("{source}")]
    Database {
        #[source]
        source: DbErr,
    },
}

impl From<DbErr> for ProjectResetDbError {
    fn from(source: DbErr) -> Self {
        Self::Database { source }
    }
}

impl From<character::CharacterProfileDeleteError> for ProjectResetDbError {
    fn from(source: character::CharacterProfileDeleteError) -> Self {
        Self::CharacterProfileDelete { source }
    }
}

impl From<generation::GenerationDbError> for ProjectResetDbError {
    fn from(source: generation::GenerationDbError) -> Self {
        Self::Generation { source }
    }
}

impl From<crate::project_delete::ProjectDeleteDbError> for ProjectResetDbError {
    fn from(source: crate::project_delete::ProjectDeleteDbError) -> Self {
        Self::ProjectCleanup { source }
    }
}

impl From<project_db::ProjectCleanupDbError> for ProjectResetDbError {
    fn from(source: project_db::ProjectCleanupDbError) -> Self {
        Self::ProjectRecordCleanup { source }
    }
}

impl From<project_db::ProjectStatusWriteError> for ProjectResetDbError {
    fn from(source: project_db::ProjectStatusWriteError) -> Self {
        Self::ProjectStatusWrite { source }
    }
}

impl From<project_db::ProjectMutationError> for ProjectResetDbError {
    fn from(source: project_db::ProjectMutationError) -> Self {
        Self::ProjectMutation { source }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectStatusRollbackOutcome {
    pub project: novel_project::Model,
    pub needs_rag_clear: bool,
}
