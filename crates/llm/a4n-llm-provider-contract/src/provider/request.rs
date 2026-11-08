use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::contracts::{ChatImage, ChatMessage, LlmTraceContext};
use super::{PromptCacheBreakpoint, PromptCacheMode};

/// 聊天请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vision_images: Vec<ChatImage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub stream_include_usage: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_scope_key: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cache_breakpoints: Vec<PromptCacheBreakpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_mode: Option<PromptCacheMode>,
    #[serde(skip)]
    pub trace_context: Option<LlmTraceContext>,
    #[serde(skip)]
    pub context_manifest_json: Option<Value>,
    #[serde(skip)]
    pub prompt_assembly_key: Option<String>,
    pub tools: Option<Vec<Value>>,
    pub tool_choice: Option<String>,
    pub stop_sequences: Option<Vec<String>>,
}

impl ChatRequest {
    pub fn new(
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<usize>,
    ) -> Self {
        Self {
            messages,
            vision_images: Vec::new(),
            temperature,
            max_tokens,
            response_format: None,
            stream_include_usage: false,
            cache_scope_key: None,
            cache_breakpoints: Vec::new(),
            cache_mode: None,
            trace_context: None,
            context_manifest_json: None,
            prompt_assembly_key: None,
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
    ) -> Self {
        Self::new(
            vec![
                ChatMessage::system(system_prompt),
                ChatMessage::user(user_prompt),
            ],
            temperature,
            max_tokens,
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

    pub fn with_prompt_assembly_key(mut self, call_key: impl Into<String>) -> Self {
        self.prompt_assembly_key = Some(call_key.into());
        self
    }

    pub fn with_json_response_format(mut self) -> Self {
        self.response_format = Some(ResponseFormat::json_object());
        self
    }

    pub fn output_schema_tokens(&self, count: impl FnOnce(&str) -> usize) -> usize {
        self.response_format
            .as_ref()
            .and_then(|format| format.json_schema.as_ref())
            .map(|schema| count(&schema.schema.to_string()))
            .unwrap_or(0)
    }

    pub fn with_json_schema_response_format(
        mut self,
        name: impl Into<String>,
        schema: Value,
        strict: bool,
    ) -> Result<Self, a4n_error::AppError> {
        self.response_format = Some(
            ResponseFormat::json_schema(name, schema, strict)
                .map_err(a4n_error::AppError::InvalidArgument)?,
        );
        Ok(self)
    }

    /// Preserve internal context that the provider wire serializer omits.
    pub fn snapshot_value(&self) -> Result<Value, a4n_error::AppError> {
        let mut value = serde_json::to_value(self)?;
        value["trace_context"] = serde_json::to_value(&self.trace_context)?;
        value["context_manifest_json"] = self.context_manifest_json.clone().unwrap_or(Value::Null);
        value["prompt_assembly_key"] = self
            .prompt_assembly_key
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null);
        Ok(value)
    }

    /// Restore a durable snapshot, including internal context omitted from the provider wire.
    pub fn from_snapshot_value(mut value: Value) -> Result<Self, a4n_error::AppError> {
        let object = value
            .as_object_mut()
            .ok_or_else(|| a4n_error::AppError::InvalidArgument("模型请求快照必须是对象".into()))?;
        let legacy_user_prefix_len = object
            .remove("cache_user_prefix_len")
            .filter(|value| !value.is_null())
            .map(serde_json::from_value::<usize>)
            .transpose()?;
        let _legacy_system_prefix = object
            .remove("cache_system_prefix")
            .map(serde_json::from_value::<bool>)
            .transpose()?;
        let trace = object.remove("trace_context");
        let manifest = object
            .remove("context_manifest_json")
            .filter(|value| !value.is_null());
        let prompt_assembly_key = object
            .remove("prompt_assembly_key")
            .filter(|value| !value.is_null())
            .map(serde_json::from_value::<String>)
            .transpose()?;
        let mut request: Self = serde_json::from_value(value)?;
        if let Some(byte_offset) = legacy_user_prefix_len {
            let message_index = request
                .messages
                .iter()
                .rposition(|message| matches!(message.role, super::MessageRole::User))
                .ok_or_else(|| {
                    a4n_error::AppError::InvalidArgument("旧缓存断点缺少 user 消息".into())
                })?;
            request.add_cache_breakpoint(message_index, byte_offset)?;
        }
        request.cache_breakpoints = request.normalized_cache_breakpoints()?;
        request.trace_context = trace.map(serde_json::from_value).transpose()?.flatten();
        request.context_manifest_json = manifest;
        request.prompt_assembly_key = prompt_assembly_key;
        Ok(request)
    }
}

pub use a4n_llm_sidecar_contract::LlmResponseFormat as ResponseFormat;

fn is_false(value: &bool) -> bool {
    !*value
}
