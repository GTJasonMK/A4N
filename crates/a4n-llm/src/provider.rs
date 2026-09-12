//! LLM request/response DTOs and provider traits.

mod contracts;
mod interface;
mod options;
mod request;
mod stream_collect;
mod stream_control;

#[cfg(test)]
mod tests;

pub use contracts::{
    ChatChunk, ChatMessage, ChatResponse, LLM_PROVIDER_CAPABILITIES, LlmFinishReason,
    LlmProtocolKind, LlmProviderCapabilities, LlmTraceContext, MessageRole, ModelCapabilities,
    ToolCall, UsageInfo, classify_finish_reason, is_truncated_finish_reason,
    validate_finish_reason,
};
pub use interface::LlmProvider;
pub use options::{normalize_extra_body, normalize_extra_body_json};
pub use request::{ChatRequest, ResponseFormat};
pub use stream_collect::{ChatStreamAccumulator, collect_chat_chunks, collect_chat_stream};
pub use stream_control::{ChatChunkStream, LLM_CALL_TOTAL_TIMEOUT, open_chat_stream};
