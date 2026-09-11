use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum MediaDbError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}
