pub(super) struct BootstrapUnavailableLlm;

#[async_trait::async_trait]
impl a4n_llm::provider::LlmProvider for BootstrapUnavailableLlm {
    async fn chat_stream(
        &self,
        _request: a4n_llm::provider::ChatRequest,
    ) -> Result<
        std::pin::Pin<
            Box<
                dyn futures::Stream<
                        Item = Result<a4n_llm::provider::ChatChunk, a4n_core::error::AppError>,
                    > + Send,
            >,
        >,
        a4n_core::error::AppError,
    > {
        Err(a4n_core::error::AppError::Internal(anyhow::anyhow!(
            "OrchestratorContext bootstrap 占位 LLM，尚未完成 with_providers"
        )))
    }

    async fn test_connection(&self) -> Result<bool, a4n_core::error::AppError> {
        Err(a4n_core::error::AppError::Internal(anyhow::anyhow!(
            "OrchestratorContext bootstrap 占位 LLM，尚未完成 with_providers"
        )))
    }

    fn model_name(&self) -> &str {
        "bootstrap-unavailable"
    }

    fn capabilities(&self) -> a4n_llm::provider::ModelCapabilities {
        a4n_llm::provider::ModelCapabilities {
            function_calling: false,
            vision: false,
            max_context_tokens: 0,
            max_output_tokens: 0,
        }
    }

    fn count_tokens(&self, _text: &str) -> usize {
        0
    }
}

pub(super) struct BootstrapUnavailableEmbedding;

#[async_trait::async_trait]
impl a4n_embedding::provider::EmbeddingProvider for BootstrapUnavailableEmbedding {
    async fn embed(&self, _texts: Vec<String>) -> Result<Vec<Vec<f32>>, a4n_core::error::AppError> {
        Err(a4n_core::error::AppError::Internal(anyhow::anyhow!(
            "OrchestratorContext bootstrap 占位 Embedding，尚未完成 with_providers"
        )))
    }

    fn dimension(&self) -> usize {
        0
    }

    fn model_name(&self) -> &str {
        "bootstrap-unavailable"
    }

    fn backend(&self) -> a4n_embedding::provider::EmbeddingBackend {
        a4n_embedding::provider::EmbeddingBackend::LocalHash
    }
}
