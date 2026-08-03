use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::contracts::{ChatMessage, LlmTraceContext};

/// 聊天请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub stream_include_usage: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub prompt_cache_system_prefix: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_user_prefix_len: Option<usize>,
    #[serde(skip)]
    pub trace_context: Option<LlmTraceContext>,
    #[serde(skip)]
    pub context_manifest_json: Option<Value>,
    pub tools: Option<Vec<Value>>,
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
            prompt_cache_key: None,
            prompt_cache_system_prefix: false,
            prompt_cache_user_prefix_len: None,
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

    pub fn with_context_manifest_json(mut self, manifest: Value) -> Self {
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
#[serde(deny_unknown_fields)]
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

fn is_false(value: &bool) -> bool {
    !*value
}
