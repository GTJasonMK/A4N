use a4n_domain::scene_profile::{
    SceneEntityStateRecordInput, SceneOccurrenceRecordInput, SceneProfileRecordInput,
    SceneTransitionRecordInput,
};
use sea_orm::DbErr;
use std::collections::BTreeMap;

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

#[derive(Debug, Clone)]
pub struct SceneProfileRecordDraft {
    pub scene_ref: String,
    pub scene_profile_id: Option<i64>,
    pub input: SceneProfileRecordInput,
}

#[derive(Debug, Clone, Default)]
pub struct SceneSyncRecordInput {
    pub profiles: Vec<SceneProfileRecordDraft>,
    pub occurrences: Vec<SceneOccurrenceRecordInput>,
    pub transitions: Vec<SceneTransitionRecordInput>,
    pub entity_states: Vec<SceneEntityStateRecordInput>,
}

#[derive(Debug, Clone, Default)]
pub struct SceneSyncRecordResult {
    pub profile_ids: Vec<i64>,
    pub profile_ref_ids: BTreeMap<String, i64>,
    pub occurrence_count: usize,
    pub transition_count: usize,
    pub entity_state_count: usize,
}
