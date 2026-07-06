//! LLM Provider trait：大语言模型调用抽象
//!
//! 定义统一的 LLM 调用接口，支持同步聊天、流式聊天、故障转移。
//! 所有 LLM 后端（OpenAI、Ollama、Anthropic）均实现此 trait。

use async_trait::async_trait;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use a4n_core::error::AppError;

/// 模型能力标记
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// 是否支持 function calling
    pub function_calling: bool,
    /// 是否支持视觉输入
    pub vision: bool,
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
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// 聊天请求
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip)]
    pub stream_include_usage: bool,
    #[serde(skip)]
    pub trace_context: Option<LlmTraceContext>,
    #[serde(skip)]
    pub context_manifest_json: Option<serde_json::Value>,
    pub tools: Option<Vec<serde_json::Value>>,
    pub tool_choice: Option<String>,
    pub stop_sequences: Option<Vec<String>>,
}

impl ChatRequest {
    pub fn new(
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<usize>,
        max_tokens_profile: Option<String>,
    ) -> Self {
        Self {
            messages,
            temperature,
            max_tokens,
            max_tokens_profile,
            response_format: None,
            stream_include_usage: false,
            trace_context: None,
            context_manifest_json: None,
            tools: None,
            tool_choice: None,
            stop_sequences: None,
        }
    }

    pub fn system_user(
        system_prompt: impl Into<String>,
        user_prompt: impl Into<String>,
        temperature: Option<f32>,
        max_tokens: Option<usize>,
        max_tokens_profile: Option<String>,
    ) -> Self {
        Self::new(
            vec![
                ChatMessage::system(system_prompt),
                ChatMessage::user(user_prompt),
            ],
            temperature,
            max_tokens,
            max_tokens_profile,
        )
    }

    pub fn with_stream_usage(mut self) -> Self {
        self.stream_include_usage = true;
        self
    }

    pub fn with_trace_context(mut self, trace_context: LlmTraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn with_context_manifest_json(mut self, manifest: serde_json::Value) -> Self {
        self.context_manifest_json = Some(manifest);
        self
    }

    pub fn with_json_response_format(mut self) -> Self {
        self.response_format = Some(ResponseFormat::json_object());
        self
    }
}

/// OpenAI-compatible response format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResponseFormat {
    pub r#type: String,
}

impl ResponseFormat {
    pub fn json_object() -> Self {
        Self {
            r#type: "json_object".to_string(),
        }
    }
}

/// 应用层 LLM 调用追踪上下文。
///
/// 该字段只用于 A4N 内部审计和成本观测，不会透传到 OpenAI-compatible 请求体。
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub context_plan_json: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_pack_refs_json: Option<serde_json::Value>,
}

/// 聊天流式块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub finish_reason: Option<String>,
    pub index: u32,
    pub usage: Option<UsageInfo>,
}

/// 聊天响应
#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub usage: Option<UsageInfo>,
    pub finish_reason: Option<String>,
}

/// 不同 OpenAI-compatible 服务对输出上限的 finish_reason 命名不完全一致。
pub fn is_truncated_finish_reason(finish_reason: Option<&str>) -> bool {
    matches!(finish_reason, Some("length" | "max_tokens"))
}

#[cfg(test)]
mod tests {
    use super::{ChatMessage, ChatRequest, LlmTraceContext, is_truncated_finish_reason};

    #[test]
    fn detects_known_truncated_finish_reasons() {
        assert!(is_truncated_finish_reason(Some("length")));
        assert!(is_truncated_finish_reason(Some("max_tokens")));
        assert!(!is_truncated_finish_reason(Some("stop")));
        assert!(!is_truncated_finish_reason(None));
    }

    #[test]
    fn chat_request_serialization_skips_internal_trace_fields() {
        let request = ChatRequest::new(vec![ChatMessage::user("hello")], None, None, None)
            .with_stream_usage()
            .with_trace_context(LlmTraceContext {
                project_id: Some(1),
                root_project_id: Some(1),
                workflow: "test".to_string(),
                step_key: "step".to_string(),
                generation_run_id: None,
                workflow_run_id: None,
                call_key: None,
                input_fingerprint_sha256: None,
                context_plan_json: None,
                context_pack_refs_json: None,
            });

        let value = serde_json::to_value(&request).expect("request serializes");
        assert!(value.get("trace_context").is_none());
        assert!(value.get("stream_include_usage").is_none());
        assert!(value.get("context_manifest_json").is_none());
    }

    #[test]
    fn chat_request_serializes_json_response_format() {
        let request = ChatRequest::new(vec![ChatMessage::user("json")], None, None, None)
            .with_json_response_format();

        let value = serde_json::to_value(&request).expect("request serializes");
        assert_eq!(value["response_format"]["type"], "json_object");
    }
}

/// Token 用量统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    pub prompt_cache_hit_tokens: Option<usize>,
    pub prompt_cache_miss_tokens: Option<usize>,
}

/// LLM Provider trait
///
/// 所有 LLM 后端必须实现此 trait。
/// 提供同步聊天、流式聊天、连接测试和能力查询。
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// 同步聊天：发送完整消息列表，返回完整响应
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError>;

    /// 流式聊天：发送消息列表，返回流式块
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError>;

    /// 测试连接可用性
    async fn test_connection(&self) -> Result<bool, AppError>;

    /// 获取模型名称
    fn model_name(&self) -> &str;

    /// 获取模型能力
    fn capabilities(&self) -> ModelCapabilities;

    /// 计算文本的 Token 数量
    fn count_tokens(&self, text: &str) -> usize;

    /// 计算消息列表的总 Token 数
    fn count_messages_tokens(&self, messages: &[ChatMessage]) -> usize {
        messages.iter().map(|m| self.count_tokens(&m.content)).sum()
    }

    /// 从消息列表中截断以适应上下文窗口
    ///
    /// 保留最新的消息，直到超出 Token 限制。
    fn truncate_messages(&self, messages: Vec<ChatMessage>, max_tokens: usize) -> Vec<ChatMessage> {
        let mut kept = Vec::new();
        let mut token_count = 0;
        for msg in messages.into_iter().rev() {
            let msg_tokens = self.count_tokens(&msg.content);
            if token_count + msg_tokens <= max_tokens {
                token_count += msg_tokens;
                kept.push(msg);
            } else {
                break;
            }
        }
        kept.reverse();
        kept
    }
}
