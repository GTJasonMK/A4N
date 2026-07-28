use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use a4n_core::error::AppError;
use a4n_llm::provider::{ChatChunk, LlmTraceContext};
use a4n_orchestrator::DynamicProviderFacade;
use a4n_queue::priority::TaskPermit;
use a4n_service_protocol::{CapabilityTaskEvent, CapabilityTaskEventKind};
use futures::Stream;

use super::events::{chat_chunk_from_sidecar_event, sidecar_event_error, usage_from_sidecar_value};
use super::terminal::{QueueTrackedStreamTerminal, StreamKind};

pub(in crate::dynamic_llm) struct QueueTrackedSidecarLlmStream {
    events: tokio::sync::mpsc::Receiver<CapabilityTaskEvent>,
    terminal: QueueTrackedStreamTerminal,
}

impl QueueTrackedSidecarLlmStream {
    pub(in crate::dynamic_llm) fn new(
        events: tokio::sync::mpsc::Receiver<CapabilityTaskEvent>,
        facade: DynamicProviderFacade,
        permit: TaskPermit,
        trace_context: Option<LlmTraceContext>,
        model_name: String,
        started: Instant,
    ) -> Self {
        Self {
            events,
            terminal: QueueTrackedStreamTerminal::new(
                StreamKind::Sidecar,
                facade,
                permit,
                trace_context,
                model_name,
                started,
            ),
        }
    }

    fn apply_usage_value(&mut self, value: Option<serde_json::Value>) -> Result<(), AppError> {
        if let Some(value) = value {
            self.terminal
                .replace_usage(usage_from_sidecar_value(value)?);
        }
        Ok(())
    }
}

impl Stream for QueueTrackedSidecarLlmStream {
    type Item = Result<ChatChunk, AppError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.terminal.is_syncing() {
            return self.terminal.poll(cx);
        }
        if self.terminal.is_finished() {
            return Poll::Ready(None);
        }
        loop {
            let event = match self.events.poll_recv(cx) {
                Poll::Ready(Some(event)) => event,
                Poll::Ready(None) => {
                    let error =
                        AppError::LlmError("LLM sidecar stream 在 Completed 前关闭".to_string());
                    self.terminal.start_failed(error);
                    return self.terminal.poll(cx);
                }
                Poll::Pending => return Poll::Pending,
            };
            match event.event_kind {
                CapabilityTaskEventKind::Chunk => {
                    let chunk = match chat_chunk_from_sidecar_event(event) {
                        Ok(chunk) => chunk,
                        Err(error) => {
                            self.terminal.start_failed(error);
                            return self.terminal.poll(cx);
                        }
                    };
                    self.terminal.observe_chunk(&chunk);
                    return Poll::Ready(Some(Ok(chunk)));
                }
                CapabilityTaskEventKind::Usage => {
                    if let Err(error) = self.apply_usage_value(event.usage_delta) {
                        self.terminal.start_failed(error);
                        return self.terminal.poll(cx);
                    }
                }
                CapabilityTaskEventKind::Completed => {
                    if let Err(error) = self.apply_usage_value(event.final_usage) {
                        self.terminal.start_failed(error);
                        return self.terminal.poll(cx);
                    }
                    if let Some(finish_reason) = event.finish_reason {
                        self.terminal.replace_finish_reason(finish_reason);
                    }
                    self.terminal.start_success();
                    return self.terminal.poll(cx);
                }
                CapabilityTaskEventKind::Failed => {
                    let error = sidecar_event_error(event);
                    self.terminal.start_failed(error);
                    return self.terminal.poll(cx);
                }
                CapabilityTaskEventKind::Progress
                | CapabilityTaskEventKind::StructuredProgress
                | CapabilityTaskEventKind::Checkpoint
                | CapabilityTaskEventKind::Artifact
                | CapabilityTaskEventKind::CancelAck => {}
            }
        }
    }
}

impl Unpin for QueueTrackedSidecarLlmStream {}

impl Drop for QueueTrackedSidecarLlmStream {
    fn drop(&mut self) {
        if self.terminal.is_pending() {
            self.terminal.start_failed(AppError::LlmError(
                "LLM sidecar stream 在完成前被释放".to_string(),
            ));
        }
    }
}
