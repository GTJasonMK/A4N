#[cfg(feature = "embedded-llm-provider")]
mod embedded_stream;

use std::pin::Pin;

use a4n_domain::{DEFAULT_LLM_MAX_CONTEXT_TOKENS, DEFAULT_LLM_MAX_OUTPUT_TOKENS};
use a4n_error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest, LlmProvider, ModelCapabilities};
use async_trait::async_trait;
use futures::stream::Stream;

use super::contracts::{DynamicLlmProvider, ResolvedLlmModel};

impl DynamicLlmProvider {
    pub(crate) async fn chat_stream_with_config_id(
        &self,
        config_id: Option<i64>,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        a4n_llm::provider::open_chat_stream(
            self.raw_chat_stream_by_id(config_id, request),
            Default::default(),
        )
        .await
    }

    async fn raw_chat_stream_by_id(
        &self,
        config_id: Option<i64>,
        request: ChatRequest,
    ) -> Result<a4n_llm::provider::ChatChunkStream, AppError> {
        let model = self.resolve_runtime_model_by_id(config_id).await?;
        self.chat_stream_with_resolved_model(model, request).await
    }

    pub(in crate::dynamic_llm) async fn chat_stream_with_resolved_model(
        &self,
        model: ResolvedLlmModel,
        request: ChatRequest,
    ) -> Result<a4n_llm::provider::ChatChunkStream, AppError> {
        if let Some(client) = self.sidecar_client.clone() {
            let prepared = self.prepare_call(model, request).await?;
            return self.chat_stream_via_sidecar(client, prepared).await;
        }

        #[cfg(not(feature = "embedded-llm-provider"))]
        {
            let _ = (model, request);
            return Err(AppError::ConfigError(
                "当前 runtime 未编译 embedded LLM provider，无法执行 LLM stream；请启用 sidecar stream endpoint 或以 --features embedded-capabilities 显式构建 full profile"
                    .to_string(),
            ));
        }

        #[cfg(feature = "embedded-llm-provider")]
        {
            let prepared = self.prepare_call(model, request).await?;
            self.chat_stream_via_embedded(prepared).await
        }
    }
}

#[async_trait]
impl LlmProvider for DynamicLlmProvider {
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        self.raw_chat_stream_by_id(None, request).await
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
}
