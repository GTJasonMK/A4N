#[cfg(feature = "embedded-llm-provider")]
mod embedded_stream;

use std::pin::Pin;

use a4n_domain::{DEFAULT_LLM_MAX_CONTEXT_TOKENS, DEFAULT_LLM_MAX_OUTPUT_TOKENS};
use a4n_error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest, LlmProvider, ModelCapabilities};
use async_trait::async_trait;
use futures::stream::Stream;

use super::config::estimate_llm_tokens;
use super::contracts::DynamicLlmProvider;

#[async_trait]
impl LlmProvider for DynamicLlmProvider {
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        if let Some(client) = self.sidecar_client.clone() {
            return self.chat_stream_via_sidecar(client, request).await;
        }

        #[cfg(not(feature = "embedded-llm-provider"))]
        {
            let _ = request;
            return Err(AppError::ConfigError(
                "当前 runtime 未编译 embedded LLM provider，无法执行 LLM stream；请启用 sidecar stream endpoint 或以 --features embedded-capabilities 显式构建 full profile"
                    .to_string(),
            ));
        }

        #[cfg(feature = "embedded-llm-provider")]
        {
            self.chat_stream_via_embedded(request).await
        }
    }

    fn model_name(&self) -> &str {
        "dynamic-default-llm"
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            max_context_tokens: DEFAULT_LLM_MAX_CONTEXT_TOKENS,
            max_output_tokens: DEFAULT_LLM_MAX_OUTPUT_TOKENS,
        }
    }

    fn count_tokens(&self, text: &str) -> usize {
        estimate_llm_tokens(text)
    }
}
