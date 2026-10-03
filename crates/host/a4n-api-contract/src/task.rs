use a4n_progress::ProgressMeta;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelTaskInput {
    #[schemars(length(min = 1, max = 160))]
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTaskHistoryInput {
    pub project_id: Option<i64>,
    pub offset: u64,
    #[schemars(range(min = 1))]
    pub limit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskProgressSummary {
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub meta: Option<ProgressMeta>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TaskInfoSummary {
    pub id: String,
    pub task_type: String,
    pub priority: String,
    pub status: String,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub progress: Option<TaskProgressSummary>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

/// Host-facing task history read boundary.
///
/// Runtime performs the existing task access check before calling this port;
/// persistence reads and the API projection stay with the task owner.
#[async_trait::async_trait]
pub trait TaskReadApplicationPort: Send + Sync {
    async fn history(
        &self,
        project_id: Option<i64>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<TaskInfoSummary>, a4n_error::AppError>;
}

/// Host-facing queue statistics boundary.
///
/// The queue owner projects its in-memory scheduler facts to the API summary;
/// Runtime only performs the surrounding command routing and authorization.
#[async_trait::async_trait]
pub trait TaskQueueStatsApplicationPort: Send + Sync {
    async fn stats(&self) -> Result<QueueStatsSummary, a4n_error::AppError>;
}

/// Narrow task-cancellation boundary used by Runtime actions.
///
/// Queue state and persistence remain owned by the task queue application;
/// callers submit only the durable task identity.
#[async_trait::async_trait]
pub trait TaskCancellationPort: Send + Sync {
    /// Returns true only when a persisted task transitioned to cancelled.
    async fn cancel(&self, task_id: &str) -> Result<bool, a4n_error::AppError>;
}

#[derive(Serialize, JsonSchema)]
pub struct QueueSlotStatsSummary {
    pub running: usize,
    pub waiting: usize,
}

#[derive(Serialize, JsonSchema)]
pub struct QueueStatsSummary {
    pub llm_queue: QueueSlotStatsSummary,
    pub image_queue: QueueSlotStatsSummary,
    pub video_queue: QueueSlotStatsSummary,
    pub embedding_queue: QueueSlotStatsSummary,
    pub background_queue: QueueSlotStatsSummary,
    pub high_priority_waiting: usize,
    pub medium_priority_waiting: usize,
    pub low_priority_waiting: usize,
}
