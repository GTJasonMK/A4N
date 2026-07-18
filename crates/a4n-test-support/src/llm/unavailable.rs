use std::pin::Pin;

use a4n_core::error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest, ChatResponse, LlmProvider, ModelCapabilities};
use async_trait::async_trait;
use futures::Stream;

pub struct UnavailableLlmProvider {
    chat_error: String,
    stream_error: String,
}

impl UnavailableLlmProvider {
    pub fn new(chat_error: impl Into<String>, stream_error: impl Into<String>) -> Self {
        Self {
            chat_error: chat_error.into(),
            stream_error: stream_error.into(),
        }
    }
}

#[async_trait]
impl LlmProvider for UnavailableLlmProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, AppError> {
        Err(AppError::LlmError(self.chat_error.clone()))
    }

    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        Err(AppError::LlmError(self.stream_error.clone()))
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        Ok(false)
    }

    fn model_name(&self) -> &str {
        "test-unavailable"
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
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
