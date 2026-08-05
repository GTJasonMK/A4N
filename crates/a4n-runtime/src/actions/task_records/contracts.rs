use a4n_core::event::progress::ProgressMeta;
use serde::{Deserialize, Serialize};

pub use a4n_orchestrator::workflow::{
    GenerationRunSummary, GetWorkflowRunInput, ListWorkflowRunsInput, TaskProgressEventSummary,
    WorkflowCheckpointSummary, WorkflowRunDetailSummary, WorkflowRunProgressSummary,
    WorkflowRunSummary,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressSummary {
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub percent: f32,
    pub meta: Option<ProgressMeta>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfoSummary {
    pub id: String,
    pub task_type: String,
    pub priority: String,
    pub status: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub progress: Option<TaskProgressSummary>,
    pub progress_events: Vec<TaskProgressEventSummary>,
    pub created_at: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSlotStatsSummary {
    pub max_concurrent: usize,
    pub running: usize,
    pub waiting: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// `get_workflow_progress_events` 的响应。
///
/// `events` 是去重后的历史里程碑；`stream_text` 是当前快照里的活体信号
/// （模型输出尾巴），随每次轮询覆盖，不进事件历史——delta 频率是每 token 一次，
/// 推成事件会冲爆环形缓冲并挤掉字段级里程碑。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowProgressEventsResponse {
    pub events: Vec<TaskProgressEventSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowProgressSummary {
    pub task_id: String,
    pub stage: String,
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub percent: f32,
    pub meta: Option<ProgressMeta>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStateSummary {
    pub task_id: String,
    pub status: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub stage: Option<String>,
    pub progress: Option<WorkflowProgressSummary>,
    pub progress_events: Vec<TaskProgressEventSummary>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelWorkflowInput {
    pub task_id: Option<String>,
    pub workflow_run_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowProgressEventsInput {
    pub task_id: Option<String>,
    pub workflow_run_id: Option<i64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResumeWorkflowRunInput {
    pub workflow_run_id: i64,
}
