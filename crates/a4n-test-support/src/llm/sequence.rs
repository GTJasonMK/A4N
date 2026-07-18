use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use a4n_core::error::AppError;
use a4n_llm::provider::{
    ChatChunk, ChatMessage, ChatRequest, ChatResponse, LlmProvider, MessageRole, ModelCapabilities,
};
use async_trait::async_trait;
use futures::{Stream, stream};

#[derive(Clone)]
pub struct SequenceLlmProvider {
    replies: Arc<Mutex<VecDeque<String>>>,
}

impl SequenceLlmProvider {
    pub fn new(replies: Vec<&str>) -> Self {
        Self::new_owned(replies.into_iter().map(ToString::to_string).collect())
    }

    pub fn new_owned(replies: Vec<String>) -> Self {
        Self {
            replies: Arc::new(Mutex::new(replies.into_iter().collect())),
        }
    }
}

#[async_trait]
impl LlmProvider for SequenceLlmProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, AppError> {
        Ok(sequence_response(pop_reply(&self.replies)?))
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
        "sequence-test-llm"
    }

    fn capabilities(&self) -> ModelCapabilities {
        sequence_capabilities()
    }

    fn count_tokens(&self, text: &str) -> usize {
        text.chars().count()
    }
}

#[derive(Clone)]
pub struct CapturingSequenceLlmProvider {
    replies: Arc<Mutex<VecDeque<String>>>,
    user_messages: Arc<Mutex<Vec<String>>>,
}

impl CapturingSequenceLlmProvider {
    pub fn new_owned(replies: Vec<String>) -> Self {
        Self {
            replies: Arc::new(Mutex::new(replies.into_iter().collect())),
            user_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn captured_user_messages(&self) -> Vec<String> {
        self.user_messages
            .lock()
            .expect("captured LLM user-message lock should not be poisoned")
            .clone()
    }
}

#[async_trait]
impl LlmProvider for CapturingSequenceLlmProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
        self.user_messages
            .lock()
            .expect("captured LLM user-message lock should not be poisoned")
            .extend(request.messages.iter().filter_map(|message| {
                matches!(&message.role, MessageRole::User).then(|| message.content.clone())
            }));
        Ok(sequence_response(pop_reply(&self.replies)?))
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
        "capturing-sequence-test-llm"
    }

    fn capabilities(&self) -> ModelCapabilities {
        sequence_capabilities()
    }

    fn count_tokens(&self, text: &str) -> usize {
        text.chars().count()
    }
}

fn pop_reply(replies: &Mutex<VecDeque<String>>) -> Result<String, AppError> {
    replies
        .lock()
        .expect("sequence LLM reply lock should not be poisoned")
        .pop_front()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("测试 LLM 响应已耗尽")))
}

fn sequence_response(content: String) -> ChatResponse {
    ChatResponse {
        message: ChatMessage::assistant(content),
        usage: None,
        finish_reason: Some("stop".to_string()),
    }
}

fn sequence_capabilities() -> ModelCapabilities {
    ModelCapabilities {
        function_calling: false,
        vision: false,
        max_context_tokens: 8192,
        max_output_tokens: 4096,
    }
}
