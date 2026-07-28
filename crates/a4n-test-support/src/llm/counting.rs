use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use a4n_core::error::AppError;
use a4n_llm::provider::{
    ChatChunk, ChatMessage, ChatRequest, ChatResponse, LlmProvider, ModelCapabilities, UsageInfo,
};
use async_trait::async_trait;
use futures::{Stream, stream};

#[derive(Clone)]
pub struct CountingLlmProvider {
    stream_calls: Arc<AtomicUsize>,
    test_connection_calls: Arc<AtomicUsize>,
    response: ChatResponse,
}

impl CountingLlmProvider {
    pub fn new(response: ChatResponse) -> Self {
        Self {
            stream_calls: Arc::new(AtomicUsize::new(0)),
            test_connection_calls: Arc::new(AtomicUsize::new(0)),
            response,
        }
    }

    pub fn stream_call_count(&self) -> usize {
        self.stream_calls.load(Ordering::SeqCst)
    }

    pub fn test_connection_call_count(&self) -> usize {
        self.test_connection_calls.load(Ordering::SeqCst)
    }
}

impl Default for CountingLlmProvider {
    fn default() -> Self {
        Self::new(ChatResponse {
            message: ChatMessage::assistant("probe ok"),
            usage: Some(UsageInfo {
                prompt_tokens: 2,
                completion_tokens: 3,
                total_tokens: 5,
                prompt_cache_hit_tokens: None,
                prompt_cache_miss_tokens: None,
            }),
            finish_reason: Some("stop".to_string()),
        })
    }
}

#[async_trait]
impl LlmProvider for CountingLlmProvider {
    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        self.stream_calls.fetch_add(1, Ordering::SeqCst);
        let response = self.response.clone();
        Ok(Box::pin(stream::iter([Ok(ChatChunk {
            content: Some(response.message.content),
            reasoning_content: None,
            tool_calls: None,
            finish_reason: response.finish_reason,
            index: 0,
            usage: response.usage,
        })])))
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        self.test_connection_calls.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    }

    fn model_name(&self) -> &str {
        "counting"
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            function_calling: false,
            vision: false,
            max_context_tokens: 1024,
            max_output_tokens: 256,
        }
    }

    fn count_tokens(&self, text: &str) -> usize {
        text.chars().count()
    }
}
