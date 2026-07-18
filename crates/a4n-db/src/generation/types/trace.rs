use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCallUsageTraceInput {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    pub prompt_cache_hit_tokens: Option<usize>,
    pub prompt_cache_miss_tokens: Option<usize>,
}

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
    pub context_plan_json: Option<Value>,
    pub context_pack_refs_json: Option<Value>,
    pub usage: Option<LlmCallUsageTraceInput>,
    pub duration_ms: Option<i64>,
    pub streamed: bool,
    pub status: String,
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
    pub finish_reason: Option<String>,
}
