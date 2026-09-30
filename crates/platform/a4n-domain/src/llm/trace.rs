use serde::{Deserialize, Serialize};

use super::usage::LlmCallUsageTraceInput;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordLlmCallTraceInput {
    pub generation_run_id: Option<i64>,
    pub workflow_run_id: Option<i64>,
    pub call_key: Option<String>,
    pub input_fingerprint_sha256: Option<String>,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub workflow: String,
    pub step_key: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub context_plan_json: Option<serde_json::Value>,
    pub context_pack_refs_json: Option<serde_json::Value>,
    pub usage: Option<LlmCallUsageTraceInput>,
    pub duration_ms: Option<i64>,
    pub streamed: bool,
    pub status: String,
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
    pub finish_reason: Option<String>,
}
