use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use a4n_core::error::AppError;
use a4n_llm::provider::{ChatChunk, LlmTraceContext};
use a4n_orchestrator::DynamicProviderFacade;
use a4n_queue::priority::TaskPermit;
use futures::Stream;

use super::terminal::{QueueTrackedStreamTerminal, StreamKind};

pub(in crate::dynamic_llm) struct QueueTrackedLlmStream {
    inner: Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>,
    terminal: QueueTrackedStreamTerminal,
}

impl QueueTrackedLlmStream {
    pub(in crate::dynamic_llm) fn new(
        inner: Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>,
        facade: DynamicProviderFacade,
        permit: TaskPermit,
        trace_context: Option<LlmTraceContext>,
        model_name: String,
        started: Instant,
    ) -> Self {
        Self {
            inner,
            terminal: QueueTrackedStreamTerminal::new(
                StreamKind::Embedded,
                facade,
                permit,
                trace_context,
                model_name,
                started,
            ),
        }
    }
}

impl Stream for QueueTrackedLlmStream {
    type Item = Result<ChatChunk, AppError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.terminal.is_syncing() {
            return self.terminal.poll(cx);
        }
        if self.terminal.is_finished() {
            return Poll::Ready(None);
        }
        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                self.terminal.observe_chunk(&chunk);
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(err))) => {
                self.terminal.start_failed(err);
                self.terminal.poll(cx)
            }
            Poll::Ready(None) => {
                self.terminal.start_success();
                self.terminal.poll(cx)
            }
            other => other,
        }
    }
}

impl Unpin for QueueTrackedLlmStream {}

impl Drop for QueueTrackedLlmStream {
    fn drop(&mut self) {
        if self.terminal.is_pending() {
            self.terminal
                .start_failed(AppError::LlmError("LLM stream 在完成前被释放".to_string()));
        }
    }
}
