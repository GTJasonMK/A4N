use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum SceneProfileDbError {
    #[error("数据库错误: {0}")]
    Database(#[from] DbErr),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error("状态无效：当前状态 {current}，不允许操作 {operation}")]
    InvalidState { current: String, operation: String },
}

impl From<crate::generation::GenerationDbError> for SceneProfileDbError {
    fn from(error: crate::generation::GenerationDbError) -> Self {
        match error {
            crate::generation::GenerationDbError::InvalidArgument(message) => {
                Self::InvalidArgument(message)
            }
            crate::generation::GenerationDbError::NotFound(message) => Self::NotFound(message),
            crate::generation::GenerationDbError::Database(error) => Self::Database(error),
        }
    }
}

impl From<crate::generation::GenerationLifecycleGateError> for SceneProfileDbError {
    fn from(error: crate::generation::GenerationLifecycleGateError) -> Self {
        use crate::generation::GenerationLifecycleGateError as Error;
        match error {
            Error::InvalidArgument(message) => Self::InvalidArgument(message),
            Error::NotFound(message) => Self::NotFound(message),
            Error::Blocked(message) => Self::InvalidState {
                current: "project_lifecycle_gate".to_string(),
                operation: message,
            },
            Error::Corrupt(message) => Self::Database(DbErr::Custom(message)),
            Error::Database(error) => Self::Database(error),
        }
    }
}

impl From<crate::text_anchor::TextAnchorDbError> for SceneProfileDbError {
    fn from(error: crate::text_anchor::TextAnchorDbError) -> Self {
        match error {
            crate::text_anchor::TextAnchorDbError::Database(error) => Self::Database(error),
            crate::text_anchor::TextAnchorDbError::NotFound(message) => Self::NotFound(message),
            crate::text_anchor::TextAnchorDbError::InvalidArgument(message) => {
                Self::InvalidArgument(message)
            }
        }
    }
}

pub type SceneProfileDbResult<T> = Result<T, SceneProfileDbError>;
