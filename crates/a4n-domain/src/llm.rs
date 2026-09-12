use serde::{Deserialize, Serialize};

pub const DEFAULT_LLM_MAX_CONTEXT_TOKENS: usize = 128_000;
pub const DEFAULT_LLM_MAX_OUTPUT_TOKENS: usize = 8_192;

pub const LLM_RESERVED_REQUEST_OPTION_KEYS: &[&str] = &[
    "model",
    "messages",
    "stream",
    "temperature",
    "max_tokens",
    "response_format",
    "prompt_cache_key",
    "tools",
    "tool_choice",
    "stop",
    "stop_sequences",
    "system",
    "anthropic_version",
    "cache_control",
];

#[derive(Debug, Clone)]
pub struct RecordOrReuseContextPackInput {
    pub project_id: i64,
    pub root_project_id: i64,
    pub scope_kind: String,
    pub workflow: String,
    pub pack_kind: String,
    pub source_refs_json: Option<serde_json::Value>,
    pub config_hash: String,
    pub content_sha256: String,
    pub canonical_content_json: serde_json::Value,
    pub canonical_content_text: String,
    pub compression_level: String,
    pub token_estimate: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPackReference {
    pub id: i64,
    pub pack_kind: String,
    pub content_sha256: String,
    pub compression_level: String,
    pub token_estimate: i32,
}

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
