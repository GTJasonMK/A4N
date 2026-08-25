use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum ProjectCleanupDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{source}")]
    Video {
        #[source]
        source: crate::video::VideoDbError,
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

impl From<DbErr> for ProjectCleanupDbError {
    fn from(source: DbErr) -> Self {
        Self::Database { source }
    }
}
