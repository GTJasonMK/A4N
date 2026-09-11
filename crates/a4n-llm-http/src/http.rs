//! OpenAI-compatible HTTP LLM provider.

mod anthropic;
mod errors;
mod provider;
mod response;
mod sse;
mod stream;
#[cfg(test)]
mod tests;

use anthropic::AnthropicLlmProvider;
use provider::HttpLlmProvider;

use a4n_error::AppError;
use a4n_llm::provider::{LlmProtocolKind, LlmProvider, ModelCapabilities};
use serde_json::Value;

pub fn pick_llm_provider(
    provider: &str,
    base_url: String,
    model: String,
    api_key: Option<String>,
    timeout_secs: u64,
    extra_body_json: Option<Value>,
    capabilities: ModelCapabilities,
) -> Result<Box<dyn LlmProvider>, AppError> {
    match LlmProtocolKind::for_provider(provider) {
        Some(LlmProtocolKind::OpenAiChatCompletions) => Ok(Box::new(
            HttpLlmProvider::new_with_timeout_options_and_capabilities(
                base_url,
                model,
                api_key,
                timeout_secs,
                extra_body_json,
                capabilities,
            )?,
        )),
        Some(LlmProtocolKind::AnthropicMessages) => Ok(Box::new(AnthropicLlmProvider::new(
            base_url,
            model,
            api_key,
            timeout_secs,
            extra_body_json,
            capabilities,
        )?)),
        None => Err(AppError::ConfigError(format!(
            "不支持的 LLM provider: {provider}"
        ))),
    }
}
