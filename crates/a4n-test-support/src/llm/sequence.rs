use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use a4n_error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest, LlmProvider, MessageRole, ModelCapabilities};
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
    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        Ok(reply_stream(pop_reply(&self.replies)?))
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
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        self.user_messages
            .lock()
            .expect("captured LLM user-message lock should not be poisoned")
            .extend(
                request
                    .messages
                    .iter()
                    .filter(|message| matches!(&message.role, MessageRole::User))
                    .map(|message| message.content.clone()),
            );
        Ok(reply_stream(pop_reply(&self.replies)?))
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

fn reply_stream(
    content: String,
) -> Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>> {
    Box::pin(stream::iter([Ok(ChatChunk {
        content: Some(content),
        reasoning_content: None,
        tool_calls: None,
        finish_reason: Some("stop".to_string()),
        index: 0,
        usage: None,
    })]))
}

fn sequence_capabilities() -> ModelCapabilities {
    ModelCapabilities {
        max_context_tokens: 8192,
        max_output_tokens: 4096,
    }
}
