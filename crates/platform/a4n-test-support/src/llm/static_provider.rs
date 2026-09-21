use std::pin::Pin;

use a4n_error::AppError;
use a4n_llm_provider_contract::{ChatChunk, ChatRequest, LlmProvider, ModelCapabilities};
use async_trait::async_trait;
use futures::{Stream, stream};

#[derive(Clone)]
pub struct StaticLlmProvider {
    reply: String,
}

impl StaticLlmProvider {
    pub fn new(reply: impl Into<String>) -> Self {
        Self {
            reply: reply.into(),
        }
    }
}

#[async_trait]
impl LlmProvider for StaticLlmProvider {
    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        Ok(Box::pin(stream::iter([Ok(ChatChunk {
            content: Some(self.reply.clone()),
            reasoning_content: None,
            tool_calls: None,
            finish_reason: Some("stop".to_string()),
            index: 0,
            usage: None,
        })])))
    }

    fn model_name(&self) -> &str {
        "static-test-llm"
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            max_context_tokens: 8192,
            max_output_tokens: 1024,
        }
    }

    fn count_tokens(&self, text: &str) -> usize {
        text.chars().count()
    }
}
