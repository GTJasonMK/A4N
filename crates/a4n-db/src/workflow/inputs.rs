use a4n_domain::progress::ProgressMeta;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ListWorkflowRunsInput {
    pub project_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub chapter_ids: Option<Vec<i64>>,
    pub workflow_kind: Option<String>,
    pub status: Option<String>,
    pub resumable_only: Option<bool>,
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetWorkflowRunInput {
    pub workflow_run_id: i64,
    pub include_generation_runs: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertWorkflowRunInput {
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
    pub active_task_id: Option<String>,
    pub can_resume: bool,
    pub resume_block_reason: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertWorkflowCheckpointInput {
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
    pub completed: bool,
}

#[derive(Debug, Clone)]
pub struct WorkflowProgressEventInput<'a> {
    pub workflow_run_id: i64,
    pub task_id: Option<&'a str>,
    pub current: u32,
    pub total: u32,
    pub message: &'a str,
    pub meta: Option<ProgressMeta>,
}

/// 视频生成等路径的独占 workflow claim 输入。
#[derive(Debug, Clone)]
pub struct ClaimExclusiveWorkflowRunInput {
    pub run: UpsertWorkflowRunInput,
    pub resume_from_task_id: Option<String>,
}
