use a4n_domain::object_profile::ObjectProfileRecord;
use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum AssetDbError {
    #[error("数据库错误: {0}")]
    Database(#[from] DbErr),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error("状态无效：当前状态 {current}，不允许操作 {operation}")]
    InvalidState { current: String, operation: String },
}

impl From<crate::generation::GenerationDbError> for AssetDbError {
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

impl From<crate::text_anchor::TextAnchorDbError> for AssetDbError {
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

#[derive(Debug, Clone, Default)]
pub struct ObjectProfileUpsertRecordResult {
    pub created: Vec<ObjectProfileRecord>,
    pub updated: Vec<ObjectProfileRecord>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct ObjectProfileUpsertModels {
    pub created: Vec<a4n_db_models::models::asset::object_profile::Model>,
    pub updated: Vec<a4n_db_models::models::asset::object_profile::Model>,
}
