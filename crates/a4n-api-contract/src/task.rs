use a4n_domain::progress::ProgressMeta;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelTaskInput {
    #[schemars(length(min = 1, max = 160))]
    pub task_id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTaskHistoryInput {
    pub project_id: Option<i64>,
    pub offset: u64,
    #[schemars(range(min = 1))]
    pub limit: u64,
}

#[derive(Serialize, JsonSchema)]
pub struct TaskProgressSummary {
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub meta: Option<ProgressMeta>,
}

#[derive(Serialize, JsonSchema)]
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
