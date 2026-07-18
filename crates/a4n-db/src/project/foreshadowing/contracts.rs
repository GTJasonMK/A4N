use a4n_core::models::character::foreshadowing_index;
use sea_orm::DbErr;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum ForeshadowingDbError {
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
    pub status: String,
    pub related_characters: Option<Value>,
    pub importance: i32,
}

#[derive(Debug, Clone, Default)]
pub struct ForeshadowingAnalysisSyncResult {
    pub current: Vec<foreshadowing_index::Model>,
    pub stale_ids: Vec<i64>,
}
