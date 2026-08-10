use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use a4n_core::error::AppError;
use a4n_llm::gateway::LlmGatewayPayload;
use a4n_llm::provider::ChatChunk;
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, SIDECAR_STREAM_IDLE_TIMEOUT, ServiceRequest,
};
use a4n_sidecar_bootstrap::service_event_line;
use axum::body::Bytes;
use futures::{Stream, StreamExt, stream};
use tokio::time::{Instant, Interval, MissedTickBehavior};

use super::super::provider_from_config;
use super::projection::{chunk_event, completed_event, failed_event};
use crate::protocol::service_error;

pub(super) type EventLineStream = Pin<Box<dyn Stream<Item = Result<Bytes, Infallible>> + Send>>;
type ChatChunkStream = Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>;
type ProviderStreamFuture = Pin<Box<dyn Future<Output = Result<ChatChunkStream, AppError>> + Send>>;

pub(super) fn stream_with_provider(request: ServiceRequest<LlmGatewayPayload>) -> EventLineStream {
    let task_id = request.request_id.clone();
    let provider_stream = Box::pin(async move {
        let provider = provider_from_config(request.payload.provider)?;
        provider.chat_stream(request.payload.request).await
    });
    Box::pin(llm_event_stream_from_source(
        task_id,
        LlmEventSource::Connecting(provider_stream),
        stream_heartbeat_interval(),
    ))
}

#[cfg(test)]
pub(crate) fn llm_event_stream(
    task_id: String,
    chunks: ChatChunkStream,
) -> impl Stream<Item = Result<Bytes, Infallible>> + Send {
    llm_event_stream_from_source(
        task_id,
        LlmEventSource::Streaming(chunks),
        stream_heartbeat_interval(),
    )
}

fn llm_event_stream_from_source(
    task_id: String,
    source: LlmEventSource,
    heartbeat_interval: Duration,
) -> impl Stream<Item = Result<Bytes, Infallible>> + Send {
    stream::unfold(
        LlmEventStreamState {
            task_id,
            sequence: 0,
            completed_sent: false,
            final_usage: None,
            finish_reason: None,
            source,
            heartbeat: heartbeat_timer(heartbeat_interval),
        },
        |mut state| async move {
            if state.completed_sent {
                return None;
            }

            let event = loop {
                let heartbeat = &mut state.heartbeat;
                let step = match &mut state.source {
                    LlmEventSource::Connecting(provider_stream) => {
                        tokio::select! {
                            biased;
                            result = provider_stream.as_mut() => LlmEventStreamStep::Connected(result),
                            _ = heartbeat.tick() => LlmEventStreamStep::Heartbeat,
                        }
                    }
                    LlmEventSource::Streaming(chunks) => {
                        tokio::select! {
                            biased;
                            chunk = chunks.next() => LlmEventStreamStep::Chunk(chunk),
                            _ = heartbeat.tick() => LlmEventStreamStep::Heartbeat,
                        }
                    }
                };

                match step {
                    LlmEventStreamStep::Connected(Ok(chunks)) => {
                        state.source = LlmEventSource::Streaming(chunks);
                    }
                    LlmEventStreamStep::Connected(Err(error)) => {
                        state.sequence += 1;
                        state.completed_sent = true;
                        break failed_event(
                            state.task_id.clone(),
                            state.sequence,
                            service_error(error),
                        );
                    }
                    LlmEventStreamStep::Heartbeat => {
                        state.sequence += 1;
                        break CapabilityTaskEvent::new(
                            state.task_id.clone(),
                            CapabilityTaskEventKind::Progress,
                            state.sequence,
                        );
                    }
                    LlmEventStreamStep::Chunk(Some(Ok(chunk))) => {
                        state.sequence += 1;
                        match chunk_event(state.task_id.clone(), state.sequence, chunk) {
                            Ok(event) => {
                                if let Some(usage) = event.usage_delta.clone() {
                                    state.final_usage = Some(usage);
                                }
                                if let Some(finish_reason) = event.finish_reason.clone() {
                                    state.finish_reason = Some(finish_reason);
                                }
                                break event;
                            }
                            Err(error) => {
                                state.completed_sent = true;
                                break failed_event(state.task_id.clone(), state.sequence, error);
                            }
                        }
                    }
                    LlmEventStreamStep::Chunk(Some(Err(error))) => {
                        state.sequence += 1;
                        state.completed_sent = true;
                        break failed_event(
                            state.task_id.clone(),
                            state.sequence,
                            service_error(error),
                        );
                    }
                    LlmEventStreamStep::Chunk(None) => {
                        state.sequence += 1;
                        state.completed_sent = true;
                        break completed_event(
                            state.task_id.clone(),
                            state.sequence,
                            state.final_usage.clone(),
                            state.finish_reason.clone(),
                        );
                    }
                }
            };

            Some((Ok(service_event_line(event)), state))
        },
    )
}

struct LlmEventStreamState {
    task_id: String,
    sequence: u64,
    completed_sent: bool,
    final_usage: Option<serde_json::Value>,
    finish_reason: Option<String>,
    source: LlmEventSource,
    heartbeat: Interval,
}

enum LlmEventSource {
    Connecting(ProviderStreamFuture),
    Streaming(ChatChunkStream),
}

enum LlmEventStreamStep {
    Connected(Result<ChatChunkStream, AppError>),
    Heartbeat,
    Chunk(Option<Result<ChatChunk, AppError>>),
}

fn stream_heartbeat_interval() -> Duration {
    Duration::from_secs((SIDECAR_STREAM_IDLE_TIMEOUT.as_secs() / 3).max(1))
}

fn heartbeat_timer(interval: Duration) -> Interval {
    let mut timer = tokio::time::interval_at(Instant::now() + interval, interval);
    timer.set_missed_tick_behavior(MissedTickBehavior::Delay);
    timer
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[tokio::test]
    async fn llm_stream_heartbeats_before_first_and_between_chunks() {
        let chunks = stream::once(async {
            tokio::time::sleep(Duration::from_millis(25)).await;
            Ok(ChatChunk {
                content: Some("first".to_string()),
                reasoning_content: None,
                tool_calls: None,
                finish_reason: None,
                index: 0,
                usage: None,
            })
        })
        .chain(stream::once(async {
            tokio::time::sleep(Duration::from_millis(25)).await;
            Ok(ChatChunk {
                content: Some("second".to_string()),
                reasoning_content: None,
                tool_calls: None,
                finish_reason: Some("stop".to_string()),
                index: 1,
                usage: None,
            })
        }));
        let events = tokio::time::timeout(
            Duration::from_millis(200),
            llm_event_stream_from_source(
                "llm-task".to_string(),
                LlmEventSource::Streaming(Box::pin(chunks)),
                Duration::from_millis(10),
            )
            .map(|line| {
                serde_json::from_slice::<CapabilityTaskEvent>(
                    &line.expect("event stream should be infallible"),
                )
                .expect("event should deserialize")
            })
            .collect::<Vec<_>>(),
        )
        .await
        .expect("heartbeat stream should finish");

        let first_chunk = events
            .iter()
            .position(|event| event.chunk_delta.as_deref() == Some("first"))
            .expect("first chunk event");
        let second_chunk = events
            .iter()
            .position(|event| event.chunk_delta.as_deref() == Some("second"))
            .expect("second chunk event");
        assert!(
            events[..first_chunk]
                .iter()
                .any(|event| event.event_kind == CapabilityTaskEventKind::Progress)
        );
        assert!(
            events[first_chunk + 1..second_chunk]
                .iter()
                .any(|event| event.event_kind == CapabilityTaskEventKind::Progress)
        );
        assert_eq!(
            events.last().map(|event| event.event_kind),
            Some(CapabilityTaskEventKind::Completed)
        );
    }
}
