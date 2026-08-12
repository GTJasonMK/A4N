use a4n_core::models::character::foreshadowing_index;
pub use a4n_domain::character::ForeshadowingRecordInput;
use sea_orm::DbErr;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum ForeshadowingDbError {
    #[error("项目不存在，不能同步伏笔: project_id={project_id}")]
    ProjectNotFound { project_id: i64 },
    #[error("伏笔不存在: project_id={project_id}, foreshadowing_id={foreshadowing_id}")]
    RecordNotFound {
        project_id: i64,
        foreshadowing_id: i64,
    },
    #[error("{0}")]
    InvalidArgument(String),
    #[error("状态无效：当前状态 {current}，不允许操作 {operation}")]
    InvalidState { current: String, operation: String },
    #[error("第 {chapter_number} 章不能回收伏笔 {foreshadowing_id}：{reason}")]
    InvalidResolutionTarget {
        foreshadowing_id: i64,
        chapter_number: i32,
        reason: String,
    },
    #[error("第 {chapter_number} 章重复回收伏笔 {foreshadowing_id}")]
    DuplicateResolutionTarget {
        foreshadowing_id: i64,
        chapter_number: i32,
    },
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ForeshadowingCleanupResult {
    pub stale_ids: Vec<i64>,
    pub changed_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForeshadowingAnalysisDraft {
    pub description: String,
    pub related_characters: Option<Value>,
    pub importance: i32,
}

#[derive(Debug, Clone, Default)]
pub struct ForeshadowingAnalysisSyncResult {
    pub current: Vec<foreshadowing_index::Model>,
    pub created: Vec<foreshadowing_index::Model>,
    pub updated: Vec<foreshadowing_index::Model>,
    pub existing: Vec<foreshadowing_index::Model>,
    pub resolved: Vec<foreshadowing_index::Model>,
    pub stale_ids: Vec<i64>,
}
