use sea_orm::DbErr;

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
