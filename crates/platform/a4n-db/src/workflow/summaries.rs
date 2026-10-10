pub use a4n_domain::workflow::{
    WorkflowCheckpointSummary, WorkflowRunProgressSummary, WorkflowRunResumeRecord,
    WorkflowRunSummary,
};
use a4n_progress::ProgressMeta;
use a4n_queue::priority::TaskInfo;
use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedTaskInfo {
    pub task: TaskInfo,
    pub workflow_run_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CancelWorkflowRunPersistenceResult {
    pub workflow_run_id: i64,
    pub active_task_id: Option<String>,
    pub interrupted_generated_video_ids: Vec<i64>,
    pub already_terminal: bool,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressEventSummary {
    pub id: String,
    pub task_id: Option<String>,
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub percent: f32,
    pub meta: Option<ProgressMeta>,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromQueryResult)]
pub struct WorkflowRunRecoveryRecord {
    pub id: i64,
    pub business_key: String,
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromQueryResult)]
pub struct WorkflowRunStatusRecord {
    pub id: i64,
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromQueryResult)]
pub struct WorkflowRunAccessRecord {
    pub id: i64,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub has_raw_response: bool,
    pub has_parsed_output: bool,
    pub has_validation: bool,
    pub attempt: i32,
    pub response_summary_json: Option<Value>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub(crate) struct GenerationRunSummaryRecord {
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
    pub has_raw_response: bool,
    pub has_parsed_output: bool,
    pub has_validation: bool,
    pub attempt: i32,
    pub response_summary_json: Option<Value>,
    pub error_message: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub completed_at: Option<chrono::NaiveDateTime>,
}

impl From<GenerationRunSummaryRecord> for GenerationRunSummary {
    fn from(run: GenerationRunSummaryRecord) -> Self {
        Self {
            id: run.id,
            workflow_run_id: run.workflow_run_id,
            workflow: run.workflow,
            step_key: run.step_key,
            call_key: run.call_key,
            title: run.title,
            model_kind: run.model_kind,
            provider: run.provider,
            model: run.model,
            status: run.status,
            input_fingerprint_sha256: run.input_fingerprint_sha256,
            input_fingerprint_json: run.input_fingerprint_json,
            raw_response_json: run.raw_response_json,
            parsed_output_json: run.parsed_output_json,
            validation_json: run.validation_json,
            artifact_refs_json: run.artifact_refs_json,
            has_raw_response: run.has_raw_response,
            has_parsed_output: run.has_parsed_output,
            has_validation: run.has_validation,
            attempt: run.attempt,
            response_summary_json: run.response_summary_json,
            error_message: run.error_message,
            created_at: run.created_at.and_utc().to_rfc3339(),
            completed_at: run.completed_at.map(|value| value.and_utc().to_rfc3339()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunDetailSummary {
    pub run: WorkflowRunSummary,
    pub checkpoints: Vec<WorkflowCheckpointSummary>,
    pub generation_runs: Vec<GenerationRunSummary>,
}
