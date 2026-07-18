use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_llm::provider::{ChatMessage, ChatRequest, LlmProvider};
use futures::StreamExt;
use tokio_util::sync::CancellationToken;

use super::{
    CancellingLlmProvider, CapturingSequenceLlmProvider, CountingLlmProvider, SequenceLlmProvider,
    StaticLlmProvider, SwitchableLlmProvider, UnavailableLlmProvider,
};

fn request() -> ChatRequest {
    ChatRequest::new(vec![ChatMessage::user("test")], None, None, None)
}

#[tokio::test]
async fn static_provider_returns_the_same_sync_and_stream_reply() {
    let provider = StaticLlmProvider::new("fixed reply");

    let response = provider
        .chat(request())
        .await
        .expect("static chat response");
    let mut chunks = provider
        .chat_stream(request())
        .await
        .expect("static stream response");
    let chunk = chunks
        .next()
        .await
        .expect("one stream chunk")
        .expect("successful stream chunk");

    assert_eq!(response.message.content, "fixed reply");
    assert_eq!(response.finish_reason.as_deref(), Some("stop"));
    assert_eq!(chunk.content.as_deref(), Some("fixed reply"));
    assert_eq!(chunk.finish_reason.as_deref(), Some("stop"));
    assert!(chunks.next().await.is_none());
}

#[tokio::test]
async fn sequence_provider_consumes_fifo_and_fails_when_exhausted() {
    let provider = SequenceLlmProvider::new(vec!["first", "second"]);

    let first = provider.chat(request()).await.expect("first response");
    let second = provider.chat(request()).await.expect("second response");
    let error = provider
        .chat(request())
        .await
        .expect_err("exhausted sequence should fail");

    assert_eq!(first.message.content, "first");
    assert_eq!(second.message.content, "second");
    assert!(error.to_string().contains("测试 LLM 响应已耗尽"));
}

#[tokio::test]
async fn capturing_sequence_provider_records_only_user_messages() {
    let provider = CapturingSequenceLlmProvider::new_owned(vec![
        "first reply".to_string(),
        "second reply".to_string(),
    ]);
    let first_request = ChatRequest::new(
        vec![
            ChatMessage::system("system"),
            ChatMessage::user("first user"),
            ChatMessage::assistant("history"),
        ],
        None,
        None,
        None,
    );

    let first = provider
        .chat(first_request)
        .await
        .expect("first captured response");
    let second = provider
        .chat(ChatRequest::new(
            vec![ChatMessage::user("second user")],
            None,
            None,
            None,
        ))
        .await
        .expect("second captured response");

    assert_eq!(first.message.content, "first reply");
    assert_eq!(second.message.content, "second reply");
    assert_eq!(
        provider.captured_user_messages(),
        vec!["first user".to_string(), "second user".to_string()]
    );
}

#[tokio::test]
async fn cancelling_provider_cancels_then_keeps_the_chat_pending() {
    let cancel_token = CancellationToken::new();
    let provider = CancellingLlmProvider::new(cancel_token.clone());

    tokio::select! {
        _ = cancel_token.cancelled() => {}
        result = provider.chat(request()) => {
            panic!("cancelling provider chat must remain pending, got {result:?}");
        }
    }

    assert!(cancel_token.is_cancelled());
}

#[tokio::test]
async fn counting_provider_tracks_chat_and_connection_calls_independently() {
    let provider = CountingLlmProvider::default();

    let response = provider
        .chat(request())
        .await
        .expect("counted chat response");
    assert!(provider.test_connection().await.expect("connection result"));

    assert_eq!(response.message.content, "probe ok");
    assert_eq!(response.usage.map(|usage| usage.total_tokens), Some(5));
    assert_eq!(provider.chat_call_count(), 1);
    assert_eq!(provider.test_connection_call_count(), 1);
}

#[tokio::test]
async fn switchable_provider_replaces_its_delegate() {
    let provider = SwitchableLlmProvider::new();
    let initial_error = provider
        .chat(request())
        .await
        .expect_err("initial empty sequence must reject calls");
    assert!(initial_error.to_string().contains("测试 LLM 响应已耗尽"));

    provider.set(Arc::new(StaticLlmProvider::new("switched reply")));
    let response = provider
        .chat(request())
        .await
        .expect("switched delegate response");

    assert_eq!(response.message.content, "switched reply");
    assert_eq!(provider.model_name(), "switchable-test-llm");
}

#[tokio::test]
async fn unavailable_llm_preserves_sync_stream_and_capability_contracts() {
    let provider = UnavailableLlmProvider::new("chat unavailable", "stream unavailable");
    let chat_error = provider
        .chat(request())
        .await
        .expect_err("unavailable chat must fail");
    let stream_error = match provider.chat_stream(request()).await {
        Ok(_) => panic!("unavailable stream must fail"),
        Err(error) => error,
    };

    assert!(matches!(
        chat_error,
        AppError::LlmError(message) if message == "chat unavailable"
    ));
    assert!(matches!(
        stream_error,
        AppError::LlmError(message) if message == "stream unavailable"
    ));
    assert!(!provider.test_connection().await.expect("connection result"));
    assert_eq!(provider.model_name(), "test-unavailable");
    assert_eq!(provider.count_tokens("text"), 0);

    let capabilities = provider.capabilities();
    assert!(!capabilities.function_calling);
    assert!(!capabilities.vision);
    assert_eq!(capabilities.max_context_tokens, 0);
    assert_eq!(capabilities.max_output_tokens, 0);
}
