use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{LlmTraceContext, UsageInfo};
use a4n_provider_runtime_contract::TaskLease;

use crate::dynamic_llm::trace::{
    llm_error_kind, llm_trace_status_for_error, record_dynamic_llm_trace,
};
use crate::provider_runtime::DynamicProviderFacade;

use super::StreamKind;

pub(super) struct TerminalSyncContext {
    pub(super) kind: StreamKind,
    pub(super) running_task: Box<dyn TaskLease>,
    pub(super) facade: DynamicProviderFacade,
    pub(super) trace_context: Option<LlmTraceContext>,
    pub(super) model_name: String,
    pub(super) duration_ms: Option<i64>,
    pub(super) usage: Option<UsageInfo>,
    pub(super) finish_reason: Option<String>,
}

pub(super) fn start_completed_sync(
    context: TerminalSyncContext,
) -> tokio::task::JoinHandle<Result<(), AppError>> {
    tokio::spawn(async move {
        let status_result = context.running_task.complete().await;
        record_dynamic_llm_trace(
            &context.facade,
            context.trace_context.as_ref(),
            Some(context.model_name.as_str()),
            context.usage,
            context.duration_ms,
            true,
            "completed",
            None,
            None,
            context.finish_reason.as_deref(),
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
        let status_result = if matches!(error, AppError::Cancelled) {
            context.running_task.cancel().await
        } else {
            context.running_task.fail(message.clone()).await
        };
        record_dynamic_llm_trace(
            &context.facade,
            context.trace_context.as_ref(),
            Some(context.model_name.as_str()),
            context.usage,
            context.duration_ms,
            true,
            llm_trace_status_for_error(&error),
            Some(llm_error_kind(&error)),
            Some(message.as_str()),
            context.finish_reason.as_deref(),
        )
        .await;
        match status_result {
            Ok(()) => Err(error),
            Err(status_error) => Err(context.kind.failed_queue_error(error, status_error)),
        }
    })
}
