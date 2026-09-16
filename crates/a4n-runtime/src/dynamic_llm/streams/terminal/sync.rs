use a4n_domain::cost::{ModelUsageMeter, ModelUsageRecordCompletion};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{LlmTraceContext, UsageInfo};
use a4n_provider_runtime_contract::ModelUsageRecordHandle;
use a4n_provider_runtime_contract::TaskLease;

use crate::dynamic_llm::trace::{
    llm_error_kind, llm_trace_status_for_error, record_dynamic_llm_trace,
};
use crate::provider_runtime::DynamicProviderFacade;

use super::StreamKind;

pub(super) struct TerminalSyncContext {
    pub(super) kind: StreamKind,
    pub(super) running_task: Box<dyn TaskLease>,
    pub(super) cost_record: ModelUsageRecordHandle,
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
        let cost_result = context
            .facade
            .finish_model_usage(
                context.cost_record,
                ModelUsageRecordCompletion {
                    status: "completed".to_string(),
                    meter: usage_meter(context.usage.as_ref()),
                    error_kind: None,
                    error_message: None,
                },
            )
            .await;
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
        match (status_result, cost_result) {
            (_, Err(error)) => Err(error),
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) => Err(context.kind.completed_queue_error(error)),
        }
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
        let cost_result = context
            .facade
            .finish_model_usage(
                context.cost_record,
                ModelUsageRecordCompletion {
                    status: if matches!(error, AppError::Cancelled) {
                        "cancelled".to_string()
                    } else {
                        "failed".to_string()
                    },
                    meter: usage_meter(context.usage.as_ref()),
                    error_kind: Some(llm_error_kind(&error).to_string()),
                    error_message: Some(message.clone()),
                },
            )
            .await;
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
        if let Err(cost_error) = cost_result {
            return Err(cost_error);
        }
        match status_result {
            Ok(()) => Err(error),
            Err(status_error) => Err(context.kind.failed_queue_error(error, status_error)),
        }
    })
}

fn usage_meter(usage: Option<&UsageInfo>) -> ModelUsageMeter {
    usage
        .map(|usage| ModelUsageMeter {
            input_tokens: Some(usage.prompt_tokens as u64),
            output_tokens: Some(usage.completion_tokens as u64),
            cache_read_tokens: usage.prompt_cache_hit_tokens.map(|value| value as u64),
            cache_write_tokens: usage.prompt_cache_write_tokens.map(|value| value as u64),
            ..Default::default()
        })
        .unwrap_or_default()
}
