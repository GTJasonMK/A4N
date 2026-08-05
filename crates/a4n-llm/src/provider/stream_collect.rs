//! 最小 chat_stream 收集器。
//!
//! 仅用于连通性探测与测试桩；生产生成必须走 orchestrator 的
//! `execute_llm_generation`（带 publisher / timeout / batch）。

use a4n_core::error::AppError;
use futures::StreamExt;

use super::{ChatRequest, ChatResponse, LlmProvider, UsageInfo};

/// 通过真实 `chat_stream` 收集一次完整响应，不调用 provider `chat()`。
pub async fn collect_chat_stream(
    llm: &dyn LlmProvider,
    request: ChatRequest,
) -> Result<ChatResponse, AppError> {
    let mut stream = llm.chat_stream(request).await?;
    let mut content = String::new();
    let mut usage: Option<UsageInfo> = None;
    let mut finish_reason: Option<String> = None;

    while let Some(item) = stream.next().await {
        let chunk = item?;
        if let Some(delta) = chunk.content.as_deref().filter(|text| !text.is_empty()) {
            content.push_str(delta);
        }
        if chunk.usage.is_some() {
            usage = chunk.usage;
        }
        if let Some(reason) = chunk.finish_reason {
            finish_reason = Some(reason);
            break;
        }
    }

    if finish_reason.is_none() {
        return Err(AppError::LlmError(
            "模型流结束但缺少 finish_reason".to_string(),
        ));
    }

    Ok(ChatResponse {
        message: super::ChatMessage::assistant(content),
        usage,
        finish_reason,
    })
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;

    use async_trait::async_trait;
    use futures::{Stream, stream};

    use super::*;
    use crate::provider::{
        ChatChunk, ChatMessage, LlmProvider, MessageRole, ModelCapabilities, UsageInfo,
    };

    struct StreamOnlyProvider;

    #[async_trait]
    impl LlmProvider for StreamOnlyProvider {
        async fn chat_stream(
            &self,
            _request: ChatRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError>
        {
            Ok(Box::pin(stream::iter([Ok(ChatChunk {
                content: Some("ok".to_string()),
                reasoning_content: None,
                tool_calls: None,
                finish_reason: Some("stop".to_string()),
                index: 0,
                usage: Some(UsageInfo {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                    total_tokens: 2,
                    prompt_cache_hit_tokens: None,
                    prompt_cache_miss_tokens: None,
                }),
            })])))
        }

        async fn test_connection(&self) -> Result<bool, AppError> {
            Ok(true)
        }

        fn model_name(&self) -> &str {
            "stream-only"
        }

        fn capabilities(&self) -> ModelCapabilities {
            ModelCapabilities {
                max_context_tokens: 128,
                max_output_tokens: 32,
            }
        }

        fn count_tokens(&self, text: &str) -> usize {
            text.chars().count()
        }
    }

    #[tokio::test]
    async fn collect_chat_stream_uses_stream_not_chat() {
        let response = collect_chat_stream(
            &StreamOnlyProvider,
            ChatRequest::new(
                vec![ChatMessage {
                    role: MessageRole::User,
                    content: "ping".into(),
                    tool_calls: None,
                    tool_call_id: None,
                }],
                None,
                None,
                None,
            ),
        )
        .await
        .expect("stream collect should succeed");
        assert_eq!(response.message.content, "ok");
        assert_eq!(response.finish_reason.as_deref(), Some("stop"));
        assert_eq!(response.usage.map(|u| u.total_tokens), Some(2));
    }
}
