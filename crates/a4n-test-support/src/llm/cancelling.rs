use std::future;
use std::pin::Pin;

use a4n_error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest, LlmProvider, ModelCapabilities};
use async_trait::async_trait;
use futures::Stream;
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
    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        self.cancel_token.cancel();
        future::pending().await
    }

    fn model_name(&self) -> &str {
        "cancelling-test-llm"
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            max_context_tokens: 8192,
            max_output_tokens: 4096,
        }
    }

    fn count_tokens(&self, text: &str) -> usize {
        text.chars().count()
    }
}
