//! OpenAI-compatible HTTP LLM provider implementation.

// OpenAI-compatible HTTP LLM provider.

mod anthropic;
mod cache;
#[cfg(test)]
mod contract_matrix;
mod errors;
mod native;
mod output_format;
mod provider;
mod response;
mod sse;
mod stream;
#[cfg(test)]
mod tests;

use anthropic::AnthropicLlmProvider;
use provider::HttpLlmProvider;

use a4n_error::AppError;
use a4n_llm_provider_contract::{
    ChatRequest, LlmProtocolKind, LlmProvider, ModelCapabilities, validate_prompt_cache_options,
};
pub use output_format::prepare_output_format;
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
    validate_prompt_cache_options(provider, extra_body_json.as_ref())?;
    let result: Result<Box<dyn LlmProvider>, AppError> =
        match LlmProtocolKind::for_provider(provider) {
            Some(
                protocol @ (LlmProtocolKind::OpenAiChatCompletions
                | LlmProtocolKind::OpenAiResponses
                | LlmProtocolKind::GeminiGenerateContent),
            ) => Ok(Box::new(
                HttpLlmProvider::new_with_timeout_options_and_capabilities(
                    base_url,
                    model,
                    api_key,
                    timeout_secs,
                    extra_body_json,
                    capabilities,
                )?
                .with_protocol(protocol)
                .with_reasoning_replay(provider == "deepseek"),
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
        };
    Ok(Box::new(output_format::OutputFormatProvider {
        provider: provider.to_string(),
        inner: result?,
    }))
}

pub fn request_body_preview(
    provider_kind: &str,
    model: &str,
    extra_body_json: Option<Value>,
    capabilities: ModelCapabilities,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    validate_prompt_cache_options(provider_kind, extra_body_json.as_ref())?;
    let request = output_format::prepare_output_format(provider_kind, request.clone())?;
    match LlmProtocolKind::for_provider(provider_kind) {
        Some(
            protocol @ (LlmProtocolKind::OpenAiChatCompletions
            | LlmProtocolKind::OpenAiResponses
            | LlmProtocolKind::GeminiGenerateContent),
        ) => {
            if protocol == LlmProtocolKind::OpenAiChatCompletions {
                HttpLlmProvider::request_body_preview(
                    model,
                    extra_body_json,
                    capabilities,
                    provider_kind == "deepseek",
                    &request,
                )
            } else {
                HttpLlmProvider::native_request_body_preview(
                    protocol,
                    model,
                    extra_body_json,
                    &request,
                )
            }
        }
        Some(LlmProtocolKind::AnthropicMessages) => AnthropicLlmProvider::request_body_preview(
            model,
            extra_body_json,
            capabilities,
            &request,
        ),
        None => Err(AppError::ConfigError(format!(
            "不支持的 LLM provider: {provider_kind}"
        ))),
    }
}
