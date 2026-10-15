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

pub use a4n_domain::character::CharacterProfileCurrentStateFields;

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
    #[error("状态无效：当前状态 {current}，不允许操作 {operation}")]
    InvalidState { current: String, operation: String },
    #[error(transparent)]
    Database(#[from] DbErr),
    #[error(transparent)]
    Generation(#[from] crate::generation::GenerationDbError),
}

#[derive(Debug, thiserror::Error)]
pub enum CharacterProfileDeleteError {
    #[error("角色档案 {0} 仍被来源快照或游戏剧本引用，不能删除")]
    Referenced(i64),
    #[error("{0}")]
    NotFound(String),
    #[error("状态无效：当前状态 {current}，不允许操作 {operation}")]
    InvalidState { current: String, operation: String },
    #[error(transparent)]
    Database(#[from] DbErr),
    #[error(transparent)]
    Generation(#[from] crate::generation::GenerationDbError),
    #[error(transparent)]
    ImageCleanup(#[from] crate::video::VideoDbError),
}
