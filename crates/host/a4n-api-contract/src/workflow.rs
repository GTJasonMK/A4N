use a4n_progress::ProgressMeta;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::task::TaskProgressSummary;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowProgressEvent {
    pub id: String,
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub meta: Option<ProgressMeta>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowProgressEventsResponse {
    pub events: Vec<WorkflowProgressEvent>,
    pub stream_text: Option<String>,
}

#[derive(Serialize, JsonSchema)]
pub struct WorkflowStateSummary {
    pub task_id: String,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub progress: Option<TaskProgressSummary>,
    pub progress_events: Vec<WorkflowProgressEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowRunProgressSummary {
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub percent: f32,
    pub meta: Option<ProgressMeta>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowRunSummary {
    pub id: i64,
    pub business_key: String,
    pub workflow_kind: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub scope_table: Option<String>,
    pub scope_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub mode: Option<String>,
    pub input_fingerprint_json: Option<Value>,
    pub status: String,
    pub current_step_key: Option<String>,
    pub progress: Option<WorkflowRunProgressSummary>,
    pub active_task_id: Option<String>,
    pub can_resume: bool,
    pub resume_block_reason: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowCheckpointSummary {
    pub id: i64,
    pub workflow_run_id: i64,
    pub step_key: String,
    pub step_order: i32,
    pub status: String,
    pub input_fingerprint_sha256: Option<String>,
    pub input_fingerprint_json: Option<Value>,
    pub checkpoint_json: Option<Value>,
    pub artifacts_json: Option<Value>,
    pub generation_run_ids_json: Option<Value>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerationRunSummary {
    pub id: i64,
    pub workflow_run_id: Option<i64>,
    pub workflow: String,
    pub step_key: String,
    pub call_key: Option<String>,
    pub title: String,
    pub model_kind: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub input_fingerprint_sha256: Option<String>,
    pub input_fingerprint_json: Option<Value>,
    pub raw_response_json: Option<Value>,
    pub parsed_output_json: Option<Value>,
    pub validation_json: Option<Value>,
    pub artifact_refs_json: Option<Value>,
    pub attempt: i32,
    pub response_summary_json: Option<Value>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowRunDetailSummary {
    pub run: WorkflowRunSummary,
    pub checkpoints: Vec<WorkflowCheckpointSummary>,
    pub generation_runs: Vec<GenerationRunSummary>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CancelWorkflowInput {
    pub task_id: Option<String>,
    pub workflow_run_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowProgressEventsInput {
    pub task_id: Option<String>,
    pub workflow_run_id: Option<i64>,
    #[schemars(range(min = 1, max = 200))]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListWorkflowRunsInput {
    pub project_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub workflow_kind: Option<String>,
    pub status: Option<String>,
    pub resumable_only: bool,
    #[schemars(range(min = 1, max = 200))]
    pub limit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetWorkflowRunInput {
    pub workflow_run_id: i64,
    pub include_generation_runs: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResumeWorkflowRunInput {
    pub workflow_run_id: i64,
}

/// Runtime-only task/workflow lookup record. Persistence and ORM models stay
/// inside the workflow owner; Runtime receives only the fields needed for
/// access checks, progress routing, and resume dispatch.
#[derive(Debug, Clone)]
pub struct TaskLookupRecord {
    pub task_id: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub workflow_run_id: Option<i64>,
    pub status: String,
}

pub struct ActiveTaskLookupRecord {
    pub task_id: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub status: String,
    pub workflow: WorkflowStateSummary,
}

#[derive(Debug, Clone)]
pub struct WorkflowTaskAccessRecord {
    pub workflow_run_id: i64,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct WorkflowResumeRecord {
    pub id: i64,
    pub workflow_kind: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub scope_table: Option<String>,
    pub scope_id: Option<i64>,
    pub input_fingerprint_json: Option<Value>,
    pub status: String,
    pub can_resume: bool,
    pub resume_block_reason: Option<String>,
}

/// Narrow lookup boundary used by Runtime task/workflow actions.
#[async_trait::async_trait]
pub trait WorkflowTaskLookupApplicationPort: Send + Sync {
    async fn task_by_id(
        &self,
        task_id: &str,
    ) -> Result<Option<TaskLookupRecord>, a4n_error::AppError>;

    async fn active_tasks(&self) -> Result<Vec<ActiveTaskLookupRecord>, a4n_error::AppError>;

    async fn workflow_access_by_active_task_id(
        &self,
        task_id: &str,
    ) -> Result<Option<WorkflowTaskAccessRecord>, a4n_error::AppError>;

    async fn workflow_run_for_resume(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowResumeRecord, a4n_error::AppError>;

    async fn workflow_run_summary(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowRunSummary, a4n_error::AppError>;

    async fn workflow_progress_events(
        &self,
        workflow_run_id: i64,
        limit: usize,
    ) -> Result<Vec<WorkflowProgressEvent>, a4n_error::AppError>;
}

/// Narrow workflow lifecycle boundary for Runtime workers.
///
/// The workflow owner retains persistence, queue cancellation, generated-video
/// interruption, and resume state transitions.
#[async_trait::async_trait]
pub trait WorkflowCancellationPort: Send + Sync {
    async fn mark_running(&self, workflow_run_id: i64) -> Result<(), a4n_error::AppError>;

    async fn cancel(&self, workflow_run_id: i64, reason: &str) -> Result<(), a4n_error::AppError>;

    async fn resume_with_active_task(
        &self,
        workflow_run_id: i64,
        active_task_id: &str,
    ) -> Result<(), a4n_error::AppError>;
}

/// Host-facing workflow read boundary.
///
/// Workflow visibility, persistence queries and API projection stay in the
/// workflow owner. Runtime retains only live task-handle and cancellation
/// coordination.
#[async_trait::async_trait]
pub trait WorkflowReadApplicationPort: Send + Sync {
    async fn list_for_user(
        &self,
        user_id: i64,
        input: ListWorkflowRunsInput,
    ) -> Result<Vec<WorkflowRunSummary>, a4n_error::AppError>;

    async fn detail_for_user(
        &self,
        user_id: i64,
        input: GetWorkflowRunInput,
    ) -> Result<WorkflowRunDetailSummary, a4n_error::AppError>;
}
