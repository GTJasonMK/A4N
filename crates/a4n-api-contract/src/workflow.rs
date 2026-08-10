use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::task::TaskProgressSummary;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProgressPhaseKind {
    Prepare,
    Context,
    ModelConnect,
    ModelWait,
    ModelStream,
    StructuredOutput,
    Parse,
    Validate,
    Save,
    AssetGeneration,
    Transfer,
    Queue,
    Cleanup,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProgressSeverity {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProgressMeta {
    pub task_kind: String,
    pub stage_code: String,
    pub stage_label: String,
    pub phase_kind: ProgressPhaseKind,
    pub detail: Option<String>,
    pub unit_label: Option<String>,
    pub item_index: Option<u64>,
    pub item_total: Option<u64>,
    pub parent_stage_code: Option<String>,
    pub severity: ProgressSeverity,
    pub estimated: bool,
    pub persistent: bool,
}

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

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CancelWorkflowInput {
    pub task_id: Option<String>,
    pub workflow_run_id: Option<i64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowProgressEventsInput {
    pub task_id: Option<String>,
    pub workflow_run_id: Option<i64>,
    #[schemars(range(min = 1, max = 200))]
    pub limit: usize,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResumeWorkflowRunInput {
    pub workflow_run_id: i64,
}
