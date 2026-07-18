use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum CollaborationDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    InvalidState(String),
    #[error("{0}")]
    Internal(String),
    #[error("协作记录数据库操作失败: {0}")]
    Database(#[source] DbErr),
}
