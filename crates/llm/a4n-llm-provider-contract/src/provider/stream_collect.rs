//! 最小 chat_stream 收集器。
//!
//! 仅用于连通性探测与测试桩；生产生成必须走 orchestrator 的
//! `execute_llm_generation`（带 publisher / timeout / batch）。

use std::collections::BTreeMap;

use a4n_error::AppError;
use futures::{Stream, StreamExt};
use serde_json::Value;

use super::{ChatChunk, ChatRequest, ChatResponse, LlmProvider, ToolCall, UsageInfo};

/// 通过真实 `chat_stream` 收集一次完整响应，不调用 provider `chat()`。
pub async fn collect_chat_stream(
    llm: &dyn LlmProvider,
    request: ChatRequest,
) -> Result<ChatResponse, AppError> {
    let stream = super::open_chat_stream(llm.chat_stream(request), Default::default()).await?;
    collect_chat_chunks(stream).await
}

/// 收集一个已经建立的 chat stream，并保留原生 tool call。
///
/// sidecar transport 与 embedded provider 都经过这个收集器，避免某一条
/// transport 路径在把增量转换成完整响应时丢失 tool call。
pub async fn collect_chat_chunks<S>(stream: S) -> Result<ChatResponse, AppError>
where
    S: Stream<Item = Result<ChatChunk, AppError>>,
{
    let mut stream = Box::pin(stream);
    let mut accumulator = ChatStreamAccumulator::default();

    while let Some(item) = stream.next().await {
        accumulator.push(item?)?;
    }
    accumulator.finish()
}

/// Accumulates one provider stream without losing reasoning or tool-call
/// deltas. The worker can inspect `content()` between chunks and call
/// `finish()` only after the transport has reached its terminal event.
#[derive(Debug, Clone, Default)]
pub struct ChatStreamAccumulator {
    content: String,
    reasoning_content: Option<String>,
    usage: Option<UsageInfo>,
    finish_reason: Option<String>,
    tool_calls: ToolCallCollection,
}

impl ChatStreamAccumulator {
    pub fn push(&mut self, chunk: ChatChunk) -> Result<(), AppError> {
        if self.finish_reason.is_some()
            && (chunk
                .content
                .as_deref()
                .is_some_and(|text| !text.is_empty())
                || chunk
                    .reasoning_content
                    .as_deref()
                    .is_some_and(|text| !text.is_empty())
                || chunk
                    .tool_calls
                    .as_ref()
                    .is_some_and(|calls| !calls.is_empty()))
        {
            return Err(AppError::LlmError(
                "模型在结束标记之后继续返回正文、推理或工具调用".into(),
            ));
        }
        if let Some(delta) = chunk.content.filter(|text| !text.is_empty()) {
            self.content.push_str(&delta);
        }
        if let Some(delta) = chunk.reasoning_content {
            self.reasoning_content
                .get_or_insert_with(String::new)
                .push_str(&delta);
        }
        if let Some(calls) = chunk.tool_calls {
            for call in calls {
                self.tool_calls.append(call)?;
            }
        }
        if chunk.usage.is_some() {
            self.usage = chunk.usage;
        }
        if let Some(reason) = chunk.finish_reason {
            if self
                .finish_reason
                .as_ref()
                .is_some_and(|previous| previous != &reason)
            {
                return Err(AppError::LlmError("模型返回冲突的 finish_reason".into()));
            }
            self.finish_reason = Some(reason);
        }
        Ok(())
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn reasoning_content(&self) -> Option<&str> {
        self.reasoning_content.as_deref()
    }

    pub fn finish_reason(&self) -> Option<&str> {
        self.finish_reason.as_deref()
    }

    pub fn snapshot(&self) -> Result<ChatResponse, AppError> {
        let mut message = super::ChatMessage::assistant(self.content.clone());
        message.tool_calls = self.tool_calls.clone().finish()?;
        message.preserve_tool_reasoning(self.reasoning_content())?;
        Ok(ChatResponse {
            message,
            usage: self.usage.clone(),
            finish_reason: self.finish_reason.clone(),
            reasoning_content: self.reasoning_content().map(str::to_string),
        })
    }

    pub fn finish(self) -> Result<ChatResponse, AppError> {
        super::validate_finish_reason(self.finish_reason.as_deref())?;
        let mut message = super::ChatMessage::assistant(self.content);
        message.tool_calls = self.tool_calls.finish()?;
        if message
            .tool_calls
            .as_ref()
            .is_none_or(|calls| calls.is_empty())
        {
            super::validate_text_content(&message.content)?;
        }
        message.preserve_tool_reasoning(self.reasoning_content.as_deref())?;
        Ok(ChatResponse {
            message,
            usage: self.usage,
            finish_reason: self.finish_reason,
            reasoning_content: self.reasoning_content,
        })
    }
}

#[derive(Debug, Clone, Default)]
struct ToolCallCollection {
    calls: BTreeMap<ToolCallKey, ToolCallAccumulator>,
}

impl ToolCallCollection {
    fn append(&mut self, call: ToolCall) -> Result<(), AppError> {
        let key = match call.stream_index {
            Some(index) => ToolCallKey::Index(index),
            None if !call.id.trim().is_empty() => ToolCallKey::Id(call.id.clone()),
            None => return Err(AppError::LlmError("模型 tool call 缺少 id 和 index".into())),
        };
        self.calls.entry(key).or_default().append(call)
    }

    fn finish(self) -> Result<Option<Vec<ToolCall>>, AppError> {
        if self.calls.is_empty() {
            return Ok(None);
        }
        self.calls
            .into_values()
            .map(ToolCallAccumulator::finish)
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ToolCallKey {
    Index(u32),
    Id(String),
}

#[derive(Debug, Clone, Default)]
struct ToolCallAccumulator {
    id: String,
    name: String,
    arguments: String,
    provider_metadata: Option<Value>,
}

impl ToolCallAccumulator {
    fn append(&mut self, call: ToolCall) -> Result<(), AppError> {
        if let Some(metadata) = call.provider_metadata {
            if self
                .provider_metadata
                .as_ref()
                .is_some_and(|current| current != &metadata)
            {
                return Err(AppError::LlmError(
                    "同一 tool call 收到冲突协议上下文".into(),
                ));
            }
            self.provider_metadata = Some(metadata);
        }
        if !call.id.trim().is_empty() {
            if !self.id.is_empty() && self.id != call.id {
                return Err(AppError::LlmError(format!(
                    "同一 Agent tool call 收到冲突 id: {} / {}",
                    self.id, call.id
                )));
            }
            self.id = call.id;
        }
        if !call.name.trim().is_empty() {
            if !self.name.is_empty() && self.name != call.name {
                return Err(AppError::LlmError("同一 tool call 收到冲突工具名称".into()));
            }
            self.name = call.name;
        }
        match call.arguments {
            Value::String(fragment) => self.arguments.push_str(&fragment),
            value if value.is_null() => {}
            value => {
                // Non-streaming providers (and Anthropic's adapter) may emit
                // a complete JSON value in one chunk.
                if !self.arguments.trim().is_empty() {
                    return Err(AppError::LlmError(
                        "同一 Agent tool call 同时包含增量和完整参数".to_string(),
                    ));
                }
                self.arguments = serde_json::to_string(&value).map_err(AppError::Serialization)?;
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<ToolCall, AppError> {
        if self.id.trim().is_empty() {
            return Err(AppError::LlmError("模型 tool call 缺少 id".to_string()));
        }
        let arguments = if self.arguments.trim().is_empty() {
            serde_json::json!({})
        } else {
            a4n_domain::json_contract::parse_strict_json_value(&self.arguments).map_err(
                |error| AppError::LlmError(format!("模型 tool call 参数不是合法 JSON: {error}")),
            )?
        };
        if self.name.trim().is_empty() {
            return Err(AppError::LlmError(format!(
                "模型 tool call 缺少工具名称: {}",
                self.id
            )));
        }
        Ok(ToolCall {
            id: self.id,
            name: self.name,
            arguments,
            stream_index: None,
            provider_metadata: self.provider_metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;

    use async_trait::async_trait;
    use futures::{Stream, stream};

    use super::*;

    fn text_chunk(reason: Option<&str>) -> ChatChunk {
        ChatChunk {
            content: None,
            reasoning_content: None,
            tool_calls: None,
            finish_reason: reason.map(str::to_string),
            index: 0,
            usage: None,
        }
    }

    #[test]
    fn a_later_terminal_cannot_replace_a_truncated_or_failed_terminal() {
        for reason in ["length", "content_filter", "stop"] {
            let mut accumulator = ChatStreamAccumulator::default();
            let mut first = text_chunk(Some(reason));
            first.content = Some("answer".into());
            accumulator.push(first).unwrap();
            let replacement = if reason == "stop" {
                "tool_calls"
            } else {
                "stop"
            };
            assert!(accumulator.push(text_chunk(Some(replacement))).is_err());
            assert_eq!(accumulator.finish_reason(), Some(reason));
        }
        let mut accumulator = ChatStreamAccumulator::default();
        accumulator.push(text_chunk(Some("stop"))).unwrap();
        accumulator.push(text_chunk(Some("stop"))).unwrap();
        let mut late_reasoning = text_chunk(None);
        late_reasoning.reasoning_content = Some("late output".into());
        assert!(accumulator.push(late_reasoning).is_err());
    }

    #[test]
    fn tool_ids_are_never_synthesized_or_treated_as_reserved_strings() {
        for id in ["__stream_tool_0", "call-real-id", ""] {
            let mut accumulator = ChatStreamAccumulator::default();
            let mut chunk = text_chunk(Some("tool_calls"));
            chunk.tool_calls = Some(vec![ToolCall {
                id: id.into(),
                name: "read".into(),
                arguments: serde_json::json!({}),
                stream_index: Some(0),
                provider_metadata: None,
            }]);
            accumulator.push(chunk).unwrap();
            let result = accumulator.finish();
            if id.is_empty() {
                assert!(result.is_err());
            } else {
                assert_eq!(result.unwrap().message.tool_calls.unwrap()[0].id, id);
            }
        }
    }

    #[test]
    fn a_tool_name_cannot_change_during_the_same_call() {
        let mut accumulator = ToolCallAccumulator::default();
        let call = ToolCall {
            id: "call".into(),
            name: "read".into(),
            arguments: Value::Null,
            stream_index: Some(0),
            provider_metadata: None,
        };
        accumulator.append(call.clone()).unwrap();
        accumulator.append(call.clone()).unwrap();
        assert!(
            accumulator
                .append(ToolCall {
                    name: "delete".into(),
                    ..call
                })
                .is_err()
        );
    }

    #[test]
    fn completed_tool_stream_preserves_reasoning_through_message_snapshot() {
        let mut stream = ChatStreamAccumulator::default();
        stream
            .push(ChatChunk {
                content: None,
                reasoning_content: Some("opaque provider replay state".into()),
                tool_calls: Some(vec![ToolCall {
                    id: "call".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({}),
                    stream_index: None,
                    provider_metadata: Some(serde_json::json!({"native_signature":"keep"})),
                }]),
                finish_reason: Some("tool_calls".into()),
                index: 0,
                usage: None,
            })
            .unwrap();
        let response = stream.finish().unwrap();
        let saved = serde_json::to_value(response.message).unwrap();
        let restored: super::super::ChatMessage = serde_json::from_value(saved).unwrap();
        let metadata = restored
            .tool_calls
            .unwrap()
            .remove(0)
            .provider_metadata
            .unwrap();
        assert_eq!(
            metadata["reasoning_content"],
            "opaque provider replay state"
        );
        assert_eq!(metadata["native_signature"], "keep");
    }
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
                    prompt_cache_write_tokens: None,
                }),
            })])))
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
            ),
        )
        .await
        .expect("stream collect should succeed");
        assert_eq!(response.message.content, "ok");
        assert_eq!(response.finish_reason.as_deref(), Some("stop"));
        assert_eq!(response.usage.map(|u| u.total_tokens), Some(2));
    }

    #[test]
    fn message_budget_includes_tool_arguments_and_message_framing() {
        let mut message = ChatMessage::assistant("");
        message.tool_calls = Some(vec![ToolCall {
            id: "call-1".to_string(),
            name: "edit".to_string(),
            arguments: serde_json::json!({"text": "x".repeat(10_000)}),
            stream_index: None,
            provider_metadata: None,
        }]);
        assert!(StreamOnlyProvider.count_messages_tokens(&[message]) > 10_000);
    }

    #[tokio::test]
    async fn collect_chat_stream_reassembles_incremental_tool_arguments() {
        struct ToolStreamProvider;

        #[async_trait]
        impl LlmProvider for ToolStreamProvider {
            async fn chat_stream(
                &self,
                _request: ChatRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError>
            {
                Ok(Box::pin(stream::iter([
                    Ok(ChatChunk {
                        content: None,
                        reasoning_content: None,
                        tool_calls: Some(vec![ToolCall {
                            id: "call-1".into(),
                            name: "locate".into(),
                            arguments: Value::String("{\"project_id\":".into()),
                            stream_index: Some(0),
                            provider_metadata: None,
                        }]),
                        finish_reason: None,
                        index: 0,
                        usage: None,
                    }),
                    Ok(ChatChunk {
                        content: None,
                        reasoning_content: None,
                        tool_calls: Some(vec![ToolCall {
                            id: String::new(),
                            name: String::new(),
                            arguments: Value::String("7}".into()),
                            stream_index: Some(0),
                            provider_metadata: None,
                        }]),
                        finish_reason: Some("tool_calls".into()),
                        index: 0,
                        usage: None,
                    }),
                ])))
            }

            fn model_name(&self) -> &str {
                "tool-stream"
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

        let response = collect_chat_stream(
            &ToolStreamProvider,
            ChatRequest::new(vec![ChatMessage::user("tool")], None, None),
        )
        .await
        .expect("tool stream should collect");
        let call = response
            .message
            .tool_calls
            .expect("tool call should be retained")
            .remove(0);
        assert_eq!(call.id, "call-1");
        assert_eq!(call.name, "locate");
        assert_eq!(call.arguments, serde_json::json!({"project_id": 7}));
    }

    #[tokio::test]
    async fn collect_chat_chunks_merges_by_index_and_keeps_usage_after_finish() {
        let chunks = stream::iter([
            Ok(ChatChunk {
                content: None,
                reasoning_content: None,
                tool_calls: Some(vec![ToolCall {
                    id: "call-a".into(),
                    name: "first".into(),
                    arguments: Value::String("{\"value\":".into()),
                    stream_index: Some(1),
                    provider_metadata: None,
                }]),
                finish_reason: None,
                index: 0,
                usage: None,
            }),
            Ok(ChatChunk {
                content: None,
                reasoning_content: None,
                tool_calls: Some(vec![ToolCall {
                    id: "call-b".into(),
                    name: "second".into(),
                    arguments: Value::String("{}".into()),
                    stream_index: Some(0),
                    provider_metadata: None,
                }]),
                finish_reason: None,
                index: 0,
                usage: None,
            }),
            Ok(ChatChunk {
                content: None,
                reasoning_content: None,
                tool_calls: Some(vec![ToolCall {
                    id: String::new(),
                    name: String::new(),
                    arguments: Value::String("1}".into()),
                    stream_index: Some(1),
                    provider_metadata: None,
                }]),
                finish_reason: Some("tool_calls".into()),
                index: 0,
                usage: None,
            }),
            Ok(ChatChunk {
                content: None,
                reasoning_content: None,
                tool_calls: None,
                finish_reason: None,
                index: 0,
                usage: Some(UsageInfo {
                    prompt_tokens: 3,
                    completion_tokens: 4,
                    total_tokens: 7,
                    prompt_cache_hit_tokens: None,
                    prompt_cache_miss_tokens: None,
                    prompt_cache_write_tokens: None,
                }),
            }),
        ]);

        let response = collect_chat_chunks(chunks)
            .await
            .expect("interleaved tool stream should collect");
        let calls = response.message.tool_calls.expect("tool calls");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].id, "call-b");
        assert_eq!(calls[1].id, "call-a");
        assert_eq!(calls[1].arguments, serde_json::json!({"value": 1}));
        assert_eq!(response.usage.map(|usage| usage.total_tokens), Some(7));
    }
}
