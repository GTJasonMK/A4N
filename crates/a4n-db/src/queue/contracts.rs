use sea_orm::DbErr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultProviderConcurrencyLimits {
    pub llm: usize,
    pub image: usize,
    pub video: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistTaskFamilyContext {
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistTaskQueueRecordInput {
    pub task_id: String,
    pub task_type: String,
    pub priority: String,
    pub project_id: Option<i64>,
    pub family: Option<PersistTaskFamilyContext>,
    pub workflow_run_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoverableQueuedTask {
    pub id: String,
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

impl PersistTaskQueueRecordInput {
    pub(in crate::queue) fn persisted_project_id(&self) -> Option<i64> {
        self.family
            .as_ref()
            .map_or(self.project_id, |ctx| Some(ctx.project_id))
    }

    pub(in crate::queue) fn root_project_id(&self) -> Option<i64> {
        self.family.as_ref().map(|ctx| ctx.root_project_id)
    }

    pub(in crate::queue) fn source_snapshot_id(&self) -> Option<i64> {
        self.family.as_ref().and_then(|ctx| ctx.source_snapshot_id)
    }

    pub(in crate::queue) fn task_scope(&self) -> Option<String> {
        self.family.as_ref().map(|ctx| ctx.task_scope.clone())
    }
}
