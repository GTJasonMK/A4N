use a4n_error::AppError;
use a4n_llm::provider::{ChatRequest, ChatResponse, collect_chat_chunks, open_chat_stream};
use a4n_provider_contract::LlmProviderConfig;

use super::super::contracts::{DynamicLlmProvider, ResolvedLlmModel};

impl DynamicLlmProvider {
    pub(crate) async fn chat_with_config_id(
        &self,
        config_id: Option<i64>,
        request: ChatRequest,
    ) -> Result<ChatResponse, AppError> {
        collect_chat_chunks(self.chat_stream_with_config_id(config_id, request).await?).await
    }

    pub(crate) async fn chat_with_config(
        &self,
        config: LlmProviderConfig,
        request: ChatRequest,
    ) -> Result<ChatResponse, AppError> {
        let stream = open_chat_stream(
            self.chat_stream_with_resolved_model(ResolvedLlmModel::new(config)?, request),
            Default::default(),
        )
        .await?;
        collect_chat_chunks(stream).await
    }
}
