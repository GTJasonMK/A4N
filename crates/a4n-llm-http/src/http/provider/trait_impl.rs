use std::pin::Pin;

use a4n_core::error::AppError;
use a4n_llm::provider::{
    ChatChunk, ChatMessage, ChatRequest, LlmProvider, MessageRole, ModelCapabilities,
    collect_chat_stream,
};
use async_trait::async_trait;
use futures::Stream;

use super::{CONNECTION_TEST_MAX_TOKENS, HttpLlmProvider, streaming};

#[async_trait]
impl LlmProvider for HttpLlmProvider {
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        streaming::execute_chat_stream(self, request).await
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        let test_request = ChatRequest {
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "ping".into(),
                tool_calls: None,
                tool_call_id: None,
            }],
            temperature: Some(0.0),
            max_tokens: Some(CONNECTION_TEST_MAX_TOKENS),
            max_tokens_profile: None,
            response_format: None,
            stream_include_usage: false,
            trace_context: None,
            context_manifest_json: None,
            tools: None,
            tool_choice: None,
            stop_sequences: None,
        };

        // 连通性探测只走真实 chat_stream，禁止完整响应 chat()。
        collect_chat_stream(self, test_request)
            .await
            .map(|response| !response.message.content.trim().is_empty())
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
