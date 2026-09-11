mod kind;
mod sync;

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use a4n_error::AppError;
use a4n_llm::provider::{ChatChunk, LlmTraceContext, UsageInfo};
use a4n_provider_runtime_contract::TaskLease;

use crate::provider_runtime::DynamicProviderFacade;

pub(in crate::dynamic_llm::streams) use kind::StreamKind;
use sync::{TerminalSyncContext, start_completed_sync, start_failed_sync};

pub(super) struct QueueTrackedStreamTerminal {
    kind: StreamKind,
    facade: DynamicProviderFacade,
    running_task: Option<Box<dyn TaskLease>>,
    trace_context: Option<LlmTraceContext>,
    model_name: String,
    started: Instant,
    usage: Option<UsageInfo>,
    finish_reason: Option<String>,
    terminal_task: Option<tokio::task::JoinHandle<Result<(), AppError>>>,
    finished: bool,
}

impl QueueTrackedStreamTerminal {
    pub(super) fn new(
        kind: StreamKind,
        facade: DynamicProviderFacade,
        running_task: Box<dyn TaskLease>,
        trace_context: Option<LlmTraceContext>,
        model_name: String,
        started: Instant,
    ) -> Self {
        Self {
            kind,
            facade,
            running_task: Some(running_task),
            trace_context,
            model_name,
            started,
            usage: None,
            finish_reason: None,
            terminal_task: None,
            finished: false,
        }
    }

    pub(super) fn is_syncing(&self) -> bool {
        self.terminal_task.is_some()
    }

    pub(super) fn is_finished(&self) -> bool {
        self.finished
    }

    pub(super) fn is_pending(&self) -> bool {
        self.running_task.is_some() && !self.finished
    }

    pub(super) fn observe_chunk(&mut self, chunk: &ChatChunk) {
        if let Some(usage) = chunk.usage.clone() {
            self.usage = Some(usage);
        }
        if let Some(finish_reason) = chunk.finish_reason.clone() {
            self.finish_reason = Some(finish_reason);
        }
    }

    pub(super) fn start_success(&mut self) {
        if self.finished || self.terminal_task.is_some() {
            return;
        }
        let Some(running_task) = self.running_task.take() else {
            return;
        };
        let kind = self.kind;
        let facade = self.facade.clone();
        let trace_context = self.trace_context.clone();
        let model_name = self.model_name.clone();
        let duration_ms = Some(self.started.elapsed().as_millis() as i64);
        let usage = self.usage.clone();
        let finish_reason = self.finish_reason.clone();
        self.terminal_task = Some(start_completed_sync(
            TerminalSyncContext {
                kind,
                running_task,
                facade,
                trace_context,
                model_name,
                duration_ms,
            },
            usage,
            finish_reason,
        ));
    }

    pub(super) fn start_failed(&mut self, error: AppError) {
        if self.finished || self.terminal_task.is_some() {
            return;
        }
        let Some(running_task) = self.running_task.take() else {
            return;
        };
        let kind = self.kind;
        let message = error.to_string();
        let facade = self.facade.clone();
        let trace_context = self.trace_context.clone();
        let model_name = self.model_name.clone();
        let duration_ms = Some(self.started.elapsed().as_millis() as i64);
        self.terminal_task = Some(start_failed_sync(
            TerminalSyncContext {
                kind,
                running_task,
                facade,
                trace_context,
                model_name,
                duration_ms,
            },
            error,
            message,
        ));
    }

    pub(super) fn start_dropped(&mut self, error: AppError) {
        if self.finished || self.terminal_task.is_some() {
            return;
        }
        let Some(running_task) = self.running_task.take() else {
            return;
        };
        let kind = self.kind;
        let message = error.to_string();
        let facade = self.facade.clone();
        let trace_context = self.trace_context.clone();
        let model_name = self.model_name.clone();
        let duration_ms = Some(self.started.elapsed().as_millis() as i64);
        drop(start_failed_sync(
            TerminalSyncContext {
                kind,
                running_task,
                facade,
                trace_context,
                model_name,
                duration_ms,
            },
            error,
            message,
        ));
    }

    pub(super) fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<ChatChunk, AppError>>> {
        let Some(task) = self.terminal_task.as_mut() else {
            return Poll::Ready(None);
        };
        match Pin::new(task).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                self.terminal_task = None;
                self.finished = true;
                match result {
                    Ok(Ok(())) => Poll::Ready(None),
                    Ok(Err(error)) => Poll::Ready(Some(Err(error))),
                    Err(error) => Poll::Ready(Some(Err(self.kind.terminal_task_error(error)))),
                }
            }
        }
    }
}
