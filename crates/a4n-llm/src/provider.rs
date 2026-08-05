//! LLM request/response DTOs and provider traits.

mod contracts;
mod interface;
mod options;
mod request;
mod stream_collect;

#[cfg(test)]
mod tests;

pub use contracts::{
    ChatChunk, ChatMessage, ChatResponse, LlmProtocolKind, LlmTraceContext, MessageRole,
    ModelCapabilities, ToolCall, UsageInfo, is_truncated_finish_reason,
};
pub use interface::LlmProvider;
pub use options::{normalize_extra_body, normalize_extra_body_json};
pub use request::{ChatRequest, ResponseFormat};
pub use stream_collect::collect_chat_stream;
