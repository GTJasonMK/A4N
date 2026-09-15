use crate::provider::ChatRequest;
use crate::provider::{
    ChatMessage, MessageRole, PromptCacheBreakpoint, PromptCacheMode, ResponseFormat, ToolCall,
};

pub use a4n_llm_sidecar_contract::LlmGatewayProviderConfig;
use a4n_llm_sidecar_contract::{
    LlmChatMessage, LlmChatRequest, LlmMessageRole, LlmPromptCacheBreakpoint, LlmPromptCacheMode,
    LlmResponseFormat, LlmToolCall,
};

pub type LlmGatewayPayload = a4n_llm_sidecar_contract::LlmGatewayPayload<LlmChatRequest>;
pub type LlmProbePayload = a4n_llm_sidecar_contract::LlmProbePayload<LlmChatRequest>;

impl From<ChatRequest> for LlmChatRequest {
    fn from(request: ChatRequest) -> Self {
        Self {
            messages: request.messages.into_iter().map(Into::into).collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            response_format: request.response_format.map(Into::into),
            stream_include_usage: request.stream_include_usage,
            cache_scope_key: request.cache_scope_key,
            cache_breakpoints: request
                .cache_breakpoints
                .into_iter()
                .map(Into::into)
                .collect(),
            cache_mode: request.cache_mode.map(Into::into),
            cache_system_prefix: request.cache_system_prefix,
            cache_user_prefix_len: request.cache_user_prefix_len,
            tools: request.tools,
            tool_choice: request.tool_choice,
            stop_sequences: request.stop_sequences,
        }
    }
}

impl From<LlmChatRequest> for ChatRequest {
    fn from(request: LlmChatRequest) -> Self {
        Self {
            messages: request.messages.into_iter().map(Into::into).collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            response_format: request.response_format.map(Into::into),
            stream_include_usage: request.stream_include_usage,
            cache_scope_key: request.cache_scope_key,
            cache_breakpoints: request
                .cache_breakpoints
                .into_iter()
                .map(Into::into)
                .collect(),
            cache_mode: request.cache_mode.map(Into::into),
            cache_system_prefix: request.cache_system_prefix,
            cache_user_prefix_len: request.cache_user_prefix_len,
            trace_context: None,
            context_manifest_json: None,
            tools: request.tools,
            tool_choice: request.tool_choice,
            stop_sequences: request.stop_sequences,
        }
    }
}

impl From<ChatMessage> for LlmChatMessage {
    fn from(message: ChatMessage) -> Self {
        Self {
            role: message.role.into(),
            content: message.content,
            tool_calls: message
                .tool_calls
                .map(|calls| calls.into_iter().map(Into::into).collect()),
            tool_call_id: message.tool_call_id,
        }
    }
}

impl From<LlmChatMessage> for ChatMessage {
    fn from(message: LlmChatMessage) -> Self {
        Self {
            role: message.role.into(),
            content: message.content,
            tool_calls: message
                .tool_calls
                .map(|calls| calls.into_iter().map(Into::into).collect()),
            tool_call_id: message.tool_call_id,
        }
    }
}

impl From<MessageRole> for LlmMessageRole {
    fn from(role: MessageRole) -> Self {
        match role {
            MessageRole::System => Self::System,
            MessageRole::User => Self::User,
            MessageRole::Assistant => Self::Assistant,
            MessageRole::Tool => Self::Tool,
        }
    }
}

impl From<LlmMessageRole> for MessageRole {
    fn from(role: LlmMessageRole) -> Self {
        match role {
            LlmMessageRole::System => Self::System,
            LlmMessageRole::User => Self::User,
            LlmMessageRole::Assistant => Self::Assistant,
            LlmMessageRole::Tool => Self::Tool,
        }
    }
}

impl From<ToolCall> for LlmToolCall {
    fn from(call: ToolCall) -> Self {
        Self {
            id: call.id,
            name: call.name,
            arguments: call.arguments,
            stream_index: call.stream_index,
            provider_metadata: call.provider_metadata,
        }
    }
}

impl From<LlmToolCall> for ToolCall {
    fn from(call: LlmToolCall) -> Self {
        Self {
            id: call.id,
            name: call.name,
            arguments: call.arguments,
            stream_index: call.stream_index,
            provider_metadata: call.provider_metadata,
        }
    }
}

impl From<PromptCacheBreakpoint> for LlmPromptCacheBreakpoint {
    fn from(point: PromptCacheBreakpoint) -> Self {
        Self {
            message_index: point.message_index,
            byte_offset: point.byte_offset,
        }
    }
}

impl From<LlmPromptCacheBreakpoint> for PromptCacheBreakpoint {
    fn from(point: LlmPromptCacheBreakpoint) -> Self {
        Self {
            message_index: point.message_index,
            byte_offset: point.byte_offset,
        }
    }
}

impl From<PromptCacheMode> for LlmPromptCacheMode {
    fn from(mode: PromptCacheMode) -> Self {
        match mode {
            PromptCacheMode::Explicit => Self::Explicit,
            PromptCacheMode::Implicit => Self::Implicit,
        }
    }
}

impl From<LlmPromptCacheMode> for PromptCacheMode {
    fn from(mode: LlmPromptCacheMode) -> Self {
        match mode {
            LlmPromptCacheMode::Explicit => Self::Explicit,
            LlmPromptCacheMode::Implicit => Self::Implicit,
        }
    }
}

impl From<ResponseFormat> for LlmResponseFormat {
    fn from(format: ResponseFormat) -> Self {
        Self {
            r#type: format.r#type,
        }
    }
}

impl From<LlmResponseFormat> for ResponseFormat {
    fn from(format: LlmResponseFormat) -> Self {
        Self {
            r#type: format.r#type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn chat_request_conversion_preserves_wire_fields_and_drops_internal_context() {
        let request = ChatRequest {
            messages: vec![ChatMessage {
                role: MessageRole::Assistant,
                content: "answer".to_string(),
                tool_calls: Some(vec![ToolCall {
                    id: "call-1".to_string(),
                    name: "lookup".to_string(),
                    arguments: json!({"key": "value"}),
                    stream_index: Some(0),
                    provider_metadata: Some(json!({"signature": "opaque"})),
                }]),
                tool_call_id: None,
            }],
            temperature: Some(0.2),
            max_tokens: Some(256),
            response_format: Some(ResponseFormat::json_object()),
            stream_include_usage: true,
            cache_scope_key: Some("scope".to_string()),
            cache_breakpoints: vec![PromptCacheBreakpoint {
                message_index: 0,
                byte_offset: 6,
            }],
            cache_mode: Some(PromptCacheMode::Explicit),
            cache_system_prefix: true,
            cache_user_prefix_len: Some(4),
            trace_context: Some(crate::provider::LlmTraceContext {
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
            }),
            context_manifest_json: Some(json!({"internal": true})),
            tools: Some(vec![json!({"type": "function"})]),
            tool_choice: Some("auto".to_string()),
            stop_sequences: Some(vec!["DONE".to_string()]),
        };

        let expected_wire = serde_json::to_value(&request).expect("request should serialize");
        let wire = LlmChatRequest::from(request);
        assert_eq!(serde_json::to_value(&wire).unwrap(), expected_wire);

        let restored = ChatRequest::from(wire);
        assert_eq!(serde_json::to_value(restored).unwrap(), expected_wire);
    }
}
