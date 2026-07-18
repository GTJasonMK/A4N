use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_llm::provider::{LlmTraceContext, UsageInfo};
use a4n_orchestrator::{OrchestratorContext, task_queue::PersistentTaskQueue};

use crate::dynamic_llm::trace::record_dynamic_llm_trace;

use super::StreamKind;

pub(super) struct TerminalSyncContext {
    pub(super) kind: StreamKind,
    pub(super) task_id: String,
    pub(super) task_queue: Arc<PersistentTaskQueue>,
    pub(super) orchestrator: OrchestratorContext,
    pub(super) trace_context: Option<LlmTraceContext>,
    pub(super) model_name: String,
    pub(super) duration_ms: Option<i64>,
}

pub(super) fn start_completed_sync(
    context: TerminalSyncContext,
    usage: Option<UsageInfo>,
    finish_reason: Option<String>,
) -> tokio::task::JoinHandle<Result<(), AppError>> {
    tokio::spawn(async move {
        let status_result = context.task_queue.mark_done(&context.task_id).await;
        record_dynamic_llm_trace(
            &context.orchestrator,
            context.trace_context.as_ref(),
            Some(context.model_name.as_str()),
            usage,
            context.duration_ms,
            true,
            "completed",
            None,
            None,
            finish_reason.as_deref(),
        )
        .await;
        status_result.map_err(|error| context.kind.completed_queue_error(error))
    })
}

pub(super) fn start_failed_sync(
    context: TerminalSyncContext,
    error: AppError,
    message: String,
) -> tokio::task::JoinHandle<Result<(), AppError>> {
    tokio::spawn(async move {
        let status_result = context
            .task_queue
            .mark_failed(&context.task_id, &message)
            .await;
        record_dynamic_llm_trace(
            &context.orchestrator,
            context.trace_context.as_ref(),
            Some(context.model_name.as_str()),
            None,
            context.duration_ms,
            true,
            "failed",
            Some(context.kind.failed_trace_code()),
            Some(message.as_str()),
            None,
        )
        .await;
        match status_result {
            Ok(()) => Err(error),
            Err(status_error) => Err(context.kind.failed_queue_error(error, status_error)),
        }
    })
}
