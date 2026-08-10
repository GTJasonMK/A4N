use crate::workflow::ProgressMeta;
use schemars::JsonSchema;
use serde::Serialize;

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
