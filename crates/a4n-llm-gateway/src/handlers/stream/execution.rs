use std::convert::Infallible;
use std::pin::Pin;

use a4n_core::error::AppError;
use a4n_llm::provider::{ChatChunk, LlmProvider};
use a4n_service_protocol::ServiceRequest;
use a4n_sidecar_bootstrap::service_event_line;
use axum::body::Bytes;
use futures::{Stream, StreamExt, stream};

use super::super::provider_from_config;
use super::projection::{chunk_event, completed_event, failed_event};
use crate::contracts::LlmChatPayload;
use crate::protocol::service_error;

pub(super) type EventLineStream = Pin<Box<dyn Stream<Item = Result<Bytes, Infallible>> + Send>>;

pub(super) async fn stream_with_provider(
    request: ServiceRequest<LlmChatPayload>,
) -> Result<EventLineStream, AppError> {
    let provider = provider_from_config(request.payload.provider)?;
    let chunks = provider.chat_stream(request.payload.request).await?;
    Ok(Box::pin(llm_event_stream(request.request_id, chunks)))
}

pub(crate) fn llm_event_stream(
    task_id: String,
    chunks: Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>,
) -> impl Stream<Item = Result<Bytes, Infallible>> + Send {
    stream::unfold(
        LlmEventStreamState {
            task_id,
            sequence: 0,
            completed_sent: false,
            final_usage: None,
            finish_reason: None,
            chunks,
        },
        |mut state| async move {
            if state.completed_sent {
                return None;
            }

            let event = match state.chunks.next().await {
                Some(Ok(chunk)) => {
                    state.sequence += 1;
                    match chunk_event(state.task_id.clone(), state.sequence, chunk) {
                        Ok(event) => {
                            if let Some(usage) = event.usage_delta.clone() {
                                state.final_usage = Some(usage);
                            }
                            if let Some(finish_reason) = event.finish_reason.clone() {
                                state.finish_reason = Some(finish_reason);
                            }
                            event
                        }
                        Err(error) => {
                            state.completed_sent = true;
                            failed_event(state.task_id.clone(), state.sequence, error)
                        }
                    }
                }
                Some(Err(error)) => {
                    state.sequence += 1;
                    state.completed_sent = true;
                    failed_event(state.task_id.clone(), state.sequence, service_error(error))
                }
                None => {
                    state.sequence += 1;
                    state.completed_sent = true;
                    completed_event(
                        state.task_id.clone(),
                        state.sequence,
                        state.final_usage.clone(),
                        state.finish_reason.clone(),
                    )
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
    chunks: Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>,
}
