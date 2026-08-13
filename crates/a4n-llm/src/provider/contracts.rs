use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProtocolKind {
    OpenAiChatCompletions,
    AnthropicMessages,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LlmProviderCapabilities {
    pub id: &'static str,
    pub display_name: &'static str,
    pub default_base_url: &'static str,
    pub protocol: LlmProtocolKind,
}

pub const LLM_PROVIDER_CAPABILITIES: &[LlmProviderCapabilities] = &[
    LlmProviderCapabilities {
        id: "openai",
        display_name: "OpenAI / 兼容协议",
        default_base_url: "https://api.openai.com/v1",
        protocol: LlmProtocolKind::OpenAiChatCompletions,
    },
    LlmProviderCapabilities {
        id: "anthropic",
        display_name: "Anthropic Claude",
        default_base_url: "https://api.anthropic.com",
        protocol: LlmProtocolKind::AnthropicMessages,
    },
    LlmProviderCapabilities {
        id: "deepseek",
        display_name: "DeepSeek",
        default_base_url: "https://api.deepseek.com/v1",
        protocol: LlmProtocolKind::OpenAiChatCompletions,
    },
    LlmProviderCapabilities {
        id: "qwen",
        display_name: "通义千问",
        default_base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        protocol: LlmProtocolKind::OpenAiChatCompletions,
    },
    LlmProviderCapabilities {
        id: "ollama",
        display_name: "Ollama 本地",
        default_base_url: "http://127.0.0.1:11434/v1",
        protocol: LlmProtocolKind::OpenAiChatCompletions,
    },
];

pub fn llm_provider_capabilities(provider: &str) -> Option<LlmProviderCapabilities> {
    let provider = provider.trim();
    LLM_PROVIDER_CAPABILITIES
        .iter()
        .copied()
        .find(|capabilities| capabilities.id.eq_ignore_ascii_case(provider))
}

impl LlmProtocolKind {
    pub fn for_provider(provider: &str) -> Option<Self> {
        llm_provider_capabilities(provider).map(|capabilities| capabilities.protocol)
    }
}

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
