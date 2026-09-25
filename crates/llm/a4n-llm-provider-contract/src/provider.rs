//! LLM request/response DTOs and provider traits.

mod cache;
mod contracts;
mod interface;
mod options;
mod request;
mod stream_collect;
mod stream_control;
mod wire;

#[cfg(test)]
mod tests;

pub use cache::{
    PROMPT_CACHE_MODE_FIELD, PROMPT_CACHE_OPTIONS_FIELD, PromptCacheBreakpoint, PromptCacheMode,
    validate_prompt_cache_options,
};
pub use contracts::{
    ChatChunk, ChatImage, ChatMessage, ChatResponse, LLM_PROVIDER_CAPABILITIES, LlmFinishReason,
    LlmProtocolKind, LlmProviderCapabilities, LlmTraceContext, MessageRole, ModelCapabilities,
    ToolCall, UsageInfo, classify_finish_reason, is_truncated_finish_reason,
    validate_finish_reason, validated_model_output_tokens,
};
pub use interface::LlmProvider;
pub use options::{normalize_extra_body, normalize_extra_body_json};
pub use request::{ChatRequest, ResponseFormat};
pub use stream_collect::{ChatStreamAccumulator, collect_chat_chunks, collect_chat_stream};
pub use stream_control::{ChatChunkStream, LLM_CALL_TOTAL_TIMEOUT, open_chat_stream};
