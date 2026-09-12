use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};

use a4n_error::AppError;
use a4n_progress::{FieldMilestone, ProgressSpan, StructuredJsonProgressSpec};
use async_trait::async_trait;
use futures::{Stream, stream};
use tokio_util::sync::CancellationToken;

use crate::provider::{
    ChatChunk, ChatMessage, ChatRequest, LlmProvider, ModelCapabilities, UsageInfo,
};

use super::{
    ChatJsonStreamInput, StructuredLlmProgress, chat_json_stream_result_with_progress,
    chat_json_stream_result_with_sink, chat_json_stream_with_progress,
};

const ITEM_MILESTONES: &[FieldMilestone] = &[FieldMilestone::new("name", "名称", 1)];

struct StreamProvider {
    chunks: Vec<ChatChunk>,
    stream_calls: AtomicUsize,
}

#[tokio::test]
async fn delta_sink_and_structured_progress_share_one_stream() {
    // 结构化里程碑与原始 delta 是同一个 token 流的两种投影，
    // 不得为了同时获得二者而重复请求模型。
    let provider = StreamProvider::new(vec![
        chunk(Some(r#"{"items":[{"name":"#), None, 0),
        chunk(Some(r#""甲"}]}"#), Some("stop"), 0),
    ]);
    let progress = StructuredLlmProgress {
        spec: StructuredJsonProgressSpec {
            root_array_key: "items",
            item_label: "条目",
            expected_items: None,
            milestones: ITEM_MILESTONES,
        },
        span: ProgressSpan::new(0, 100),
    };
    let mut deltas = Vec::new();
    let mut messages = Vec::new();
    let mut delta_sink = |delta: &str| deltas.push(delta.to_string());

    let result = chat_json_stream_result_with_sink(
        input(&provider, Some(progress), None),
        |_, message| {
            messages.push(message.to_string());
            Ok(())
        },
        Some(&mut delta_sink),
    )
    .await
    .expect("stream with sink should succeed");

    // 只建立一次流。
    assert_eq!(provider.stream_calls.load(Ordering::SeqCst), 1);
    // 活体信号拿到全部原始增量，拼接后等于完整响应。
    assert_eq!(deltas.concat(), result.response_content);
    assert_eq!(deltas.len(), 2);
    // 结构化里程碑仍照常产出。
    assert!(
        messages.iter().any(|message| message.contains("名称")),
        "应报出字段级里程碑，实际: {messages:?}"
    );
}

#[tokio::test]
async fn delta_sink_is_optional_and_progress_is_unchanged_without_it() {
    let provider = StreamProvider::new(vec![chunk(Some(r#"{"ok":true}"#), Some("stop"), 0)]);

    let result = chat_json_stream_result_with_progress(input(&provider, None, None), |_, _| Ok(()))
        .await
        .expect("stream without sink should succeed");

    assert_eq!(result.response_content, r#"{"ok":true}"#);
    assert_eq!(provider.stream_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn result_preserves_finish_reason_and_prompt_cache_usage() {
    let mut final_chunk = chunk(Some(r#"{"ok":true}"#), Some("stop"), 0);
    final_chunk.usage = Some(UsageInfo {
        prompt_tokens: 2_000,
        completion_tokens: 20,
        total_tokens: 2_020,
        prompt_cache_hit_tokens: Some(1_800),
        prompt_cache_miss_tokens: Some(200),
    });
    let provider = StreamProvider::new(vec![final_chunk]);

    let result = chat_json_stream_result_with_progress(input(&provider, None, None), |_, _| Ok(()))
        .await
        .expect("stream result should succeed");

    assert_eq!(result.finish_reason.as_deref(), Some("stop"));
    assert_eq!(
        result.usage.and_then(|usage| usage.prompt_cache_hit_tokens),
        Some(1_800)
    );
}

impl StreamProvider {
    fn new(chunks: Vec<ChatChunk>) -> Self {
        Self {
            chunks,
            stream_calls: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl LlmProvider for StreamProvider {
    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        self.stream_calls.fetch_add(1, Ordering::SeqCst);
        Ok(Box::pin(stream::iter(
            self.chunks.clone().into_iter().map(Ok),
        )))
    }

    fn model_name(&self) -> &str {
        "stream-test"
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            max_context_tokens: 4096,
            max_output_tokens: 1024,
        }
    }

    fn count_tokens(&self, text: &str) -> usize {
        text.len()
    }
}

fn request() -> ChatRequest {
    ChatRequest::new(vec![ChatMessage::user("test")], None, None)
}

fn chunk(content: Option<&str>, finish_reason: Option<&str>, index: u32) -> ChatChunk {
    ChatChunk {
        content: content.map(str::to_string),
        reasoning_content: None,
        tool_calls: None,
        finish_reason: finish_reason.map(str::to_string),
        index,
        usage: None,
    }
}

fn input<'a>(
    provider: &'a StreamProvider,
    structured_progress: Option<StructuredLlmProgress>,
    cancel_token: Option<&'a CancellationToken>,
) -> ChatJsonStreamInput<'a> {
    ChatJsonStreamInput {
        llm: provider,
        request: request(),
        prompt_name: "结构化测试",
        truncated_message: "输出被截断",
        structured_progress,
        cancel_token,
        cancelled_message: "请求已取消",
    }
}

#[tokio::test]
async fn concatenates_chunks_and_reports_stream_and_structured_progress() {
    let provider = StreamProvider::new(vec![
        chunk(Some(r#"{"items":[{"name":"一"},"#), None, 0),
        chunk(Some(r#"{"name":"二"}]}"#), Some("stop"), 1),
    ]);
    let progress = StructuredLlmProgress {
        spec: StructuredJsonProgressSpec {
            root_array_key: "items",
            item_label: "条目",
            expected_items: Some(2),
            milestones: ITEM_MILESTONES,
        },
        span: ProgressSpan::new(20, 80),
    };
    let mut events = Vec::new();

    let content = chat_json_stream_with_progress(input(&provider, Some(progress), None), |n, m| {
        events.push((n, m.to_string()));
        Ok(())
    })
    .await
    .expect("stream should succeed");

    assert_eq!(content, r#"{"items":[{"name":"一"},{"name":"二"}]}"#);
    assert_eq!(provider.stream_calls.load(Ordering::SeqCst), 1);
    assert!(
        events
            .iter()
            .any(|(_, message)| message == "正在连接模型：结构化测试")
    );
    assert!(events.iter().any(|(_, message)| {
        message == "模型流已连接：结构化测试，正在等待输出"
    }));
    assert!(
        events
            .iter()
            .any(|(_, message)| message == "模型已开始输出：结构化测试")
    );
    assert!(
        events
            .iter()
            .any(|(_, message)| message.contains("名称已完成"))
    );
    assert!(
        events
            .iter()
            .any(|(_, message)| message.contains("条目已完成"))
    );
    assert!(events.iter().any(|(current, message)| {
        *current == 80 && message == "结构化测试 输出完成，正在解析 JSON"
    }));
    assert!(events.windows(2).all(|pair| pair[0].0 <= pair[1].0));
}

#[tokio::test]
async fn advances_progress_while_a_long_field_is_still_streaming() {
    const PAYLOAD_MILESTONES: &[FieldMilestone] = &[FieldMilestone::new("payload", "长文本", 1)];
    let first = format!(r#"{{"payload":"{}"#, "字".repeat(1_100));
    let provider = StreamProvider::new(vec![
        chunk(Some(&first), None, 0),
        chunk(Some(r#""}"#), Some("stop"), 1),
    ]);
    let progress = StructuredLlmProgress {
        spec: StructuredJsonProgressSpec {
            root_array_key: "",
            item_label: "对象",
            expected_items: Some(1),
            milestones: PAYLOAD_MILESTONES,
        },
        span: ProgressSpan::new(20, 80),
    };
    let mut events = Vec::new();

    chat_json_stream_with_progress(
        input(&provider, Some(progress), None),
        |current, message| {
            events.push((current, message.to_string()));
            Ok(())
        },
    )
    .await
    .expect("long streamed field should succeed");

    assert!(events.iter().any(|(current, message)| {
        *current > 20 && *current < 80 && message.contains("已接收约")
    }));
    assert!(events.windows(2).all(|pair| pair[0].0 <= pair[1].0));
}

#[tokio::test]
async fn returns_the_supplied_error_for_truncated_output() {
    let provider = StreamProvider::new(vec![chunk(Some("{}"), Some("length"), 0)]);

    let error = chat_json_stream_with_progress(input(&provider, None, None), |_, _| Ok(()))
        .await
        .expect_err("length finish reason should fail");

    assert_eq!(error, "输出被截断");
}

#[tokio::test]
async fn pre_cancelled_request_does_not_open_the_provider_stream() {
    let provider = StreamProvider::new(Vec::new());
    let cancel_token = CancellationToken::new();
    cancel_token.cancel();

    let error =
        chat_json_stream_with_progress(input(&provider, None, Some(&cancel_token)), |_, _| Ok(()))
            .await
            .expect_err("pre-cancelled request should fail");

    assert_eq!(error, "请求已取消");
    assert_eq!(provider.stream_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn complete_json_without_a_model_terminal_is_not_success() {
    let provider = StreamProvider::new(vec![chunk(Some("{\"ok\":true}"), None, 0)]);
    let error = chat_json_stream_with_progress(input(&provider, None, None), |_, _| Ok(()))
        .await
        .expect_err("transport EOF is not a model terminal");
    assert!(error.contains("finish_reason"));
}
