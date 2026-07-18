use std::future;
use std::pin::Pin;

use a4n_core::error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest, ChatResponse, LlmProvider, ModelCapabilities};
use async_trait::async_trait;
use futures::{Stream, stream};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct CancellingLlmProvider {
    cancel_token: CancellationToken,
}

impl CancellingLlmProvider {
    pub fn new(cancel_token: CancellationToken) -> Self {
        Self { cancel_token }
    }
}

#[async_trait]
impl LlmProvider for CancellingLlmProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, AppError> {
        self.cancel_token.cancel();
        future::pending::<Result<ChatResponse, AppError>>().await
    }

    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        Ok(Box::pin(stream::empty()))
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        Ok(true)
    }

    fn model_name(&self) -> &str {
        "cancelling-test-llm"
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            function_calling: false,
            vision: false,
            max_context_tokens: 8192,
            max_output_tokens: 4096,
        }
    }

    fn count_tokens(&self, text: &str) -> usize {
        text.chars().count()
    }
}
