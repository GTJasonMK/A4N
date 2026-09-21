use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatChunk, LlmTraceContext};
use a4n_provider_runtime_contract::ModelUsageRecordHandle;
use a4n_provider_runtime_contract::TaskLease;
use a4n_service_protocol::CapabilityTaskEvent;
use futures::Stream;

use super::events::{SidecarLlmEvent, parse_sidecar_llm_event};
use super::terminal::{QueueTrackedStreamTerminal, StreamKind};
use crate::provider_runtime::DynamicProviderFacade;

pub(in crate::dynamic_llm) struct QueueTrackedSidecarLlmStream {
    events: tokio::sync::mpsc::Receiver<CapabilityTaskEvent>,
    terminal: QueueTrackedStreamTerminal,
}

impl QueueTrackedSidecarLlmStream {
    pub(in crate::dynamic_llm) fn new(
        events: tokio::sync::mpsc::Receiver<CapabilityTaskEvent>,
        facade: DynamicProviderFacade,
        running_task: Box<dyn TaskLease>,
        cost_record: ModelUsageRecordHandle,
        trace_context: Option<LlmTraceContext>,
        model_name: String,
        started: Instant,
    ) -> Self {
        Self {
            events,
            terminal: QueueTrackedStreamTerminal::new(
                StreamKind::Sidecar,
                facade,
                running_task,
                cost_record,
                trace_context,
                model_name,
                started,
            ),
        }
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
        if self.terminal.poll_cancelled(cx) {
            self.events.close();
            self.terminal.start_failed(AppError::Cancelled);
            return self.terminal.poll(cx);
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
            match parse_sidecar_llm_event(event) {
                Ok(SidecarLlmEvent::Chunk(chunk)) => {
                    self.terminal.observe_chunk(&chunk);
                    return Poll::Ready(Some(Ok(chunk)));
                }
                Ok(SidecarLlmEvent::Completed) => {
                    self.terminal.start_success();
                    return self.terminal.poll(cx);
                }
                Ok(SidecarLlmEvent::Progress) => {}
                Err(error) => {
                    self.terminal.start_failed(error);
                    return self.terminal.poll(cx);
                }
            }
        }
    }
}

impl Unpin for QueueTrackedSidecarLlmStream {}

impl Drop for QueueTrackedSidecarLlmStream {
    fn drop(&mut self) {
        if self.terminal.is_pending() {
            self.terminal.start_dropped(AppError::LlmError(
                "LLM sidecar stream 在完成前被释放".to_string(),
            ));
        }
    }
}
