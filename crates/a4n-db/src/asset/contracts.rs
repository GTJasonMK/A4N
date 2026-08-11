use a4n_core::models::asset::object_profile;
use sea_orm::DbErr;
use serde_json::Value;

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

#[derive(Debug, Clone)]
pub struct ObjectProfileEvidenceRecordInput {
    pub normalized_name: String,
    pub text_anchor_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct ObjectProfileRecordInput {
    pub name: String,
    pub normalized_name: String,
    pub aliases_json: Value,
    pub object_type: String,
    pub importance: String,
    pub visual_description: String,
    pub material_and_state: String,
    pub story_usage: String,
    pub visual_usage: String,
    pub generation_usage: String,
    pub location_context: Option<String>,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    pub evidence_json: Value,
    pub image_prompt_seed: String,
}

#[derive(Debug, Clone, Default)]
pub struct ObjectProfileUpsertRecordResult {
    pub created: Vec<object_profile::Model>,
    pub updated: Vec<object_profile::Model>,
}
