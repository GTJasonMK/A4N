use std::pin::Pin;
use std::sync::{Arc, Mutex};

use a4n_core::error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest, LlmProvider, ModelCapabilities};
use async_trait::async_trait;
use futures::Stream;

use super::SequenceLlmProvider;

#[derive(Clone)]
pub struct SwitchableLlmProvider {
    delegate: Arc<Mutex<Arc<dyn LlmProvider>>>,
}

impl SwitchableLlmProvider {
    pub fn new() -> Self {
        Self {
            delegate: Arc::new(Mutex::new(Arc::new(SequenceLlmProvider::new(Vec::new())))),
        }
    }

    pub fn set(&self, delegate: Arc<dyn LlmProvider>) {
        *self
            .delegate
            .lock()
            .expect("switchable LLM provider mutex poisoned") = delegate;
    }

    fn delegate(&self) -> Arc<dyn LlmProvider> {
        Arc::clone(
            &self
                .delegate
                .lock()
                .expect("switchable LLM provider mutex poisoned"),
        )
    }
}

impl Default for SwitchableLlmProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for SwitchableLlmProvider {
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        self.delegate().chat_stream(request).await
    }

    fn model_name(&self) -> &str {
        "switchable-test-llm"
    }

    fn capabilities(&self) -> ModelCapabilities {
        self.delegate().capabilities()
    }

    fn count_tokens(&self, text: &str) -> usize {
        self.delegate().count_tokens(text)
    }
}
