use a4n_db_models::models::character::foreshadowing_index;
use a4n_domain::character::ForeshadowingIndexRecord;
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ForeshadowingAnalysisDraft {
    pub existing_id: Option<i64>,
    pub evidence: String,
    pub description: String,
    pub related_characters: Option<Value>,
    pub importance: i32,
}

#[derive(Debug, Clone)]
pub struct ForeshadowingFactUpdate {
    pub id: i64,
    pub status: a4n_domain::character::ForeshadowingStatus,
    pub reason: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ForeshadowingAnalysisSyncResult {
    pub current: Vec<ForeshadowingIndexRecord>,
    pub created: Vec<ForeshadowingIndexRecord>,
    pub updated: Vec<ForeshadowingIndexRecord>,
    pub existing: Vec<ForeshadowingIndexRecord>,
    pub resolved: Vec<ForeshadowingIndexRecord>,
}

pub fn foreshadowing_index_record(row: foreshadowing_index::Model) -> ForeshadowingIndexRecord {
    ForeshadowingIndexRecord {
        id: row.id,
        project_id: row.project_id,
        description: row.description,
        setup_chapter: row.setup_chapter,
        payoff_chapter: row.payoff_chapter,
        status: row.status,
        related_characters: row.related_characters,
        importance: row.importance,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}
