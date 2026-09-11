use std::pin::Pin;

use a4n_error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest, LlmProvider, ModelCapabilities};
use async_trait::async_trait;
use futures::Stream;

use super::{HttpLlmProvider, streaming};

#[async_trait]
impl LlmProvider for HttpLlmProvider {
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        streaming::execute_chat_stream(self, request).await
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn capabilities(&self) -> ModelCapabilities {
        self.caps.clone()
    }

    fn count_tokens(&self, text: &str) -> usize {
        let chinese_chars = text.chars().filter(|c| u32::from(*c) > 0x2E80).count();
        let other_words = if chinese_chars == text.len() {
            0
        } else {
            text.split_whitespace().count()
        };
        ((chinese_chars as f32 * 0.5) + (other_words as f32 * 1.3)).ceil() as usize
    }
}
