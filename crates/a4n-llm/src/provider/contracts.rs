use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use a4n_domain::llm::{LLM_PROVIDER_CAPABILITIES, LlmProtocolKind, LlmProviderCapabilities};

/// 模型能力标记
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelCapabilities {
    /// 最大上下文窗口（Token 数）
    pub max_context_tokens: usize,
    /// 最大输出 Token 数
    pub max_output_tokens: usize,
}

/// 消息角色
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, content)
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, content)
    }

    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

/// 工具调用定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
    /// OpenAI-compatible streams identify a logical call by array index while
    /// the final call id may only arrive in a later delta.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_index: Option<u32>,
    /// Opaque protocol state needed when replaying this call (for example a
    /// Gemini thought signature or Responses encrypted reasoning items).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<Value>,
}

/// 应用层 LLM 调用追踪上下文。
///
/// 该字段只用于 A4N 内部审计和成本观测，不会透传到 OpenAI-compatible 请求体。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmTraceContext {
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub workflow: String,
    pub step_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_run_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_fingerprint_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_plan_json: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_pack_refs_json: Option<Value>,
}

/// 聊天流式块
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatChunk {
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub finish_reason: Option<String>,
    pub index: u32,
    pub usage: Option<UsageInfo>,
}

/// 聊天响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub usage: Option<UsageInfo>,
    pub finish_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
}

/// Token 用量统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageInfo {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    pub prompt_cache_hit_tokens: Option<usize>,
    pub prompt_cache_miss_tokens: Option<usize>,
}

/// 不同 OpenAI-compatible 服务对输出上限的 finish_reason 命名不完全一致。
pub fn is_truncated_finish_reason(finish_reason: Option<&str>) -> bool {
    matches!(finish_reason, Some("length" | "max_tokens"))
}
