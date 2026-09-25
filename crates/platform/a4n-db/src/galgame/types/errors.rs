use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum GalgameAssetPlanSpecRecordError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameGenerationRecordError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameMediaRecordError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameProjectRecordError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameContextRecordError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameScriptRecordError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameLineReorderRecordError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameLineRecordError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum GalgameSceneRecordError {
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}
