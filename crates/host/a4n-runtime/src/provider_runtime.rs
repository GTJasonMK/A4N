//! Runtime-owned view of the dynamic provider port.

use std::{future::Future, sync::Arc};

use a4n_domain::cost::{ModelUsageRecordCompletion, ModelUsageRecordStart};
use a4n_domain::llm::{
    ContextPackReference, RecordLlmCallTraceInput, RecordOrReuseContextPackInput,
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::LlmTraceContext;
use a4n_provider_contract::{EmbeddingProviderConfig, LlmProviderConfig};
use a4n_provider_runtime_contract::{
    ModelUsageRecordHandle, ProviderRuntimePort, ProviderTaskPriority, TaskLease,
};
use a4n_queue::{
    TaskWorkflowContext,
    priority::{TaskKind, TaskPriority as QueueTaskPriority},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct DynamicProviderFacade {
    port: Arc<dyn ProviderRuntimePort>,
    runtime_role: crate::RuntimeRole,
}

impl DynamicProviderFacade {
    pub(crate) fn new(
        port: Arc<dyn ProviderRuntimePort>,
        runtime_role: crate::RuntimeRole,
    ) -> Self {
        Self { port, runtime_role }
    }

    pub(crate) fn allows_host_provider_config(&self) -> bool {
        self.runtime_role == crate::RuntimeRole::DesktopHost
            || crate::runtime_config::current_user_id().is_none()
    }

    pub(crate) async fn resolve_llm_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<LlmProviderConfig, AppError> {
        self.port.resolve_llm_config(config_id).await
    }

    pub(crate) async fn default_embedding_config_optional(
        &self,
    ) -> Result<Option<EmbeddingProviderConfig>, AppError> {
        self.port.default_embedding_config_optional().await
    }

    pub(crate) async fn default_embedding_config(
        &self,
    ) -> Result<EmbeddingProviderConfig, AppError> {
        self.port.default_embedding_config().await
    }

    pub(crate) async fn project_root_id(&self, project_id: i64) -> Result<Option<i64>, AppError> {
        self.port.project_root_id(project_id).await
    }

    pub(crate) async fn record_llm_call_trace(
        &self,
        input: RecordLlmCallTraceInput,
    ) -> Result<(), AppError> {
        self.port.record_llm_call_trace(input).await
    }

    pub(crate) async fn start_model_usage(
        &self,
        mut input: ModelUsageRecordStart,
    ) -> Result<ModelUsageRecordHandle, AppError> {
        input.request_json = input
            .request_json
            .map(crate::actions::sanitize_preview_value);
        self.port.start_model_usage(input).await
    }

    pub(crate) async fn finish_model_usage(
        &self,
        handle: ModelUsageRecordHandle,
        input: ModelUsageRecordCompletion,
    ) -> Result<(), AppError> {
        self.port.finish_model_usage(handle, input).await
    }

    pub(crate) async fn record_or_reuse_context_pack_reference(
        &self,
        input: RecordOrReuseContextPackInput,
    ) -> Result<ContextPackReference, AppError> {
        self.port
            .record_or_reuse_context_pack_reference(input)
            .await
    }

    pub(crate) async fn record_or_reuse_context_pack_references(
        &self,
        inputs: Vec<RecordOrReuseContextPackInput>,
    ) -> Result<Vec<ContextPackReference>, AppError> {
        self.port
            .record_or_reuse_context_pack_references(inputs)
            .await
    }

    pub(crate) async fn queue_run_tracked<F, Fut, T>(
        &self,
        task_type: TaskKind,
        priority: QueueTaskPriority,
        project_id: Option<i64>,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.queue_run_tracked_with_cancel(
            task_type,
            priority,
            project_id,
            CancellationToken::new(),
            operation,
        )
        .await
    }

    pub(crate) async fn queue_run_tracked_with_workflow_context<F, Fut, T>(
        &self,
        task_type: TaskKind,
        priority: QueueTaskPriority,
        project_id: Option<i64>,
        workflow: TaskWorkflowContext,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.queue_run_tracked_with_workflow_context_and_cancel(
            task_type,
            priority,
            project_id,
            Some(workflow),
            CancellationToken::new(),
            operation,
        )
        .await
    }

    pub(crate) async fn queue_run_tracked_with_cancel<F, Fut, T>(
        &self,
        task_type: TaskKind,
        priority: QueueTaskPriority,
        project_id: Option<i64>,
        cancel_token: CancellationToken,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.queue_run_tracked_with_workflow_context_and_cancel(
            task_type,
            priority,
            project_id,
            None,
            cancel_token,
            operation,
        )
        .await
    }

    pub(crate) async fn queue_run_tracked_with_workflow_context_and_cancel<F, Fut, T>(
        &self,
        task_type: TaskKind,
        priority: QueueTaskPriority,
        project_id: Option<i64>,
        workflow: Option<TaskWorkflowContext>,
        cancel_token: CancellationToken,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        let task = self
            .port
            .start_task(
                task_type,
                provider_task_priority(priority),
                project_id,
                workflow.map(|workflow| workflow.workflow_run_id),
                cancel_token,
                None,
            )
            .await?;
        finish_provider_task(task, operation().await).await
    }

    pub(crate) async fn queue_start_llm_task(
        &self,
        limit: a4n_domain::task_queue::LlmConcurrencyLimit,
        priority: QueueTaskPriority,
        trace: Option<&LlmTraceContext>,
    ) -> Result<Box<dyn TaskLease>, AppError> {
        self.port
            .start_task(
                TaskKind::LlmCall,
                provider_task_priority(priority),
                trace.and_then(|trace| trace.project_id),
                trace.and_then(|trace| trace.workflow_run_id),
                CancellationToken::new(),
                Some(limit),
            )
            .await
    }
}

pub(crate) async fn finish_provider_task<T>(
    task: Box<dyn TaskLease>,
    result: Result<T, AppError>,
) -> Result<T, AppError> {
    match result {
        Ok(value) => task.complete().await.map(|()| value).map_err(|error| {
            AppError::Internal(anyhow::anyhow!(
                "队列任务业务操作已完成，但同步完成状态失败: {error}"
            ))
        }),
        Err(error) => {
            let status = if matches!(error, AppError::Cancelled) {
                task.cancel().await
            } else {
                task.fail(error.to_string()).await
            };
            match status {
                Ok(()) => Err(error),
                Err(status_error) => Err(AppError::Internal(anyhow::anyhow!(
                    "{error}；同步队列任务失败状态失败: {status_error}"
                ))),
            }
        }
    }
}

fn provider_task_priority(priority: QueueTaskPriority) -> ProviderTaskPriority {
    match priority {
        QueueTaskPriority::High => ProviderTaskPriority::High,
        QueueTaskPriority::Medium => ProviderTaskPriority::Medium,
        QueueTaskPriority::Low => ProviderTaskPriority::Low,
    }
}
