mod completion;
mod context_pack;
mod providers;
mod request_options;
mod trace;
mod usage;
pub use completion::{LlmFinishReason, classify_finish_reason, validate_finish_reason};
pub use context_pack::{ContextPackReference, RecordOrReuseContextPackInput};
pub use providers::{LLM_PROVIDER_CAPABILITIES, LlmProtocolKind, LlmProviderCapabilities};
pub use request_options::LLM_RESERVED_REQUEST_OPTION_KEYS;
pub use trace::RecordLlmCallTraceInput;
pub use usage::LlmCallUsageTraceInput;

pub const DEFAULT_LLM_MAX_CONTEXT_TOKENS: usize = 128_000;
pub const DEFAULT_LLM_MAX_OUTPUT_TOKENS: usize = 8_192;
