use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum ProjectCleanupDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("状态无效：当前状态 {current}，不允许操作 {operation}")]
    InvalidState { current: String, operation: String },
    #[error("{source}")]
    Video {
        #[source]
        source: crate::video::VideoDbError,
    },
    #[error("{source}")]
    Generation {
        #[source]
        source: crate::generation::GenerationDbError,
    },
    #[error("{source}")]
    Database {
        #[source]
        source: DbErr,
    },
}

impl From<crate::video::VideoDbError> for ProjectCleanupDbError {
    fn from(source: crate::video::VideoDbError) -> Self {
        Self::Video { source }
    }
}

impl From<crate::generation::GenerationDbError> for ProjectCleanupDbError {
    fn from(source: crate::generation::GenerationDbError) -> Self {
        Self::Generation { source }
    }
}

impl From<DbErr> for ProjectCleanupDbError {
    fn from(source: DbErr) -> Self {
        Self::Database { source }
    }
}
