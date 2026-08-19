use sea_orm::DbErr;
use serde_json::Value;

#[derive(Debug, Clone)]
pub(in crate::character) struct CharacterProfileCurrentStateUpdate {
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub background: Option<String>,
    pub personality: Option<String>,
    pub current_state: Value,
    pub last_synced_chapter: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterProfileCurrentStateFields {
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub background: Option<String>,
    pub personality: Option<String>,
    pub current_state: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterProfileCreateInput {
    pub project_id: i64,
    pub name: String,
    pub base_attributes: Value,
    pub current_state: Value,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub background: Option<String>,
    pub personality: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterProfileDeletionSummary {
    pub project_id: i64,
    pub profile_id: i64,
    pub portrait_paths: Vec<String>,
    pub attribute_event_ids: Vec<i64>,
    pub state_index_ids: Vec<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum CharacterProfileCreateError {
    #[error("角色档案已存在: {name}")]
    AlreadyExists { name: String },
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
    #[error(transparent)]
    Generation(#[from] crate::generation::GenerationDbError),
}

#[derive(Debug, thiserror::Error)]
pub enum CharacterProfileDeleteError {
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Database(#[from] DbErr),
    #[error(transparent)]
    Generation(#[from] crate::generation::GenerationDbError),
    #[error(transparent)]
    ImageCleanup(#[from] crate::video::VideoDbError),
}
