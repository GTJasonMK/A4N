//! LLM request/response DTOs and provider traits.

mod contracts;
mod interface;
mod options;
mod request;

#[cfg(test)]
mod tests;

pub use contracts::{
    ChatChunk, ChatMessage, ChatResponse, LlmTraceContext, MessageRole, ModelCapabilities,
    ToolCall, UsageInfo, is_truncated_finish_reason,
};
pub use interface::LlmProvider;
pub use options::{normalize_request_options, normalize_request_options_json};
pub use request::{ChatRequest, ResponseFormat};
