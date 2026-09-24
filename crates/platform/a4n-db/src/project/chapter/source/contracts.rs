use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum ResolveChapterSourceError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    InvalidState(String),
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}
