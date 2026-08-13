use sea_orm::DbErr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultProviderConcurrencyLimits {
    pub llm: usize,
    pub image: usize,
    pub video: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistTaskQueueRecordInput {
    pub task_id: String,
    pub task_type: String,
    pub priority: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub workflow_run_id: Option<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum QueueDbError {
    #[error("{0}")]
    InvalidConfig(String),
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}
