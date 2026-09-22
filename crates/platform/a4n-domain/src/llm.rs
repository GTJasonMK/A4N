use serde::{Deserialize, Serialize};

mod providers;
mod usage;
pub use providers::{LLM_PROVIDER_CAPABILITIES, LlmProtocolKind, LlmProviderCapabilities};
pub use usage::LlmCallUsageTraceInput;

pub const DEFAULT_LLM_MAX_CONTEXT_TOKENS: usize = 128_000;
pub const DEFAULT_LLM_MAX_OUTPUT_TOKENS: usize = 8_192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmFinishReason {
    Stop,
    ToolCalls,
    Truncated,
}

pub fn classify_finish_reason(finish_reason: Option<&str>) -> Result<LlmFinishReason, String> {
    match finish_reason {
        Some("stop") => Ok(LlmFinishReason::Stop),
        Some("tool_calls") => Ok(LlmFinishReason::ToolCalls),
        Some("length" | "max_tokens") => Ok(LlmFinishReason::Truncated),
        None | Some("") => Err("模型流结束但缺少 finish_reason".to_string()),
        Some(reason) => Err(format!("模型未正常完成：finish_reason={reason}")),
    }
}

/// 传输结束不等于业务完成；调用断点与 Provider 共用同一成功条件。
pub fn validate_finish_reason(finish_reason: Option<&str>) -> Result<(), String> {
    match classify_finish_reason(finish_reason)? {
        LlmFinishReason::Stop | LlmFinishReason::ToolCalls => Ok(()),
        LlmFinishReason::Truncated => Err("模型输出被 max_tokens 截断，任务未完成".to_string()),
    }
}

pub const LLM_RESERVED_REQUEST_OPTION_KEYS: &[&str] = &[
    "model",
    "messages",
    "stream",
    "temperature",
    "max_tokens",
    "max_completion_tokens",
    "response_format",
    "prompt_cache_key",
    "tools",
    "tool_choice",
    "stop",
    "stop_sequences",
    "system",
    "anthropic_version",
    "cache_control",
    "input",
    "max_output_tokens",
    "text",
    "include",
    "store",
    "contents",
    "systeminstruction",
    "toolconfig",
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
