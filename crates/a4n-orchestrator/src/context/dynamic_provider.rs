//! 动态 LLM / Embedding provider 的运行时端口适配。
//!
//! 这里仅把 application owner 接到 `a4n-provider-contract`；Runtime 不需要
//! 看到 `OrchestratorContext`、ORM model 或持久队列实现。

use std::sync::Arc;

use a4n_db_models::models::ai::{embedding_config, llm_config};
use a4n_domain::cost::{ModelUsageRecordCompletion, ModelUsageRecordStart};
use a4n_domain::llm::ContextPackReference;
use a4n_error::AppError;
use a4n_provider_contract::{EmbeddingProviderConfig, LlmProviderConfig};
use a4n_provider_runtime_contract::{
    ModelUsageRecordHandle, ProviderRuntimePort, ProviderTaskPriority, TaskLease,
};
use a4n_queue::{
    TaskWorkflowContext,
    priority::{TaskKind, TaskPriority as QueueTaskPriority},
};
use tokio_util::sync::CancellationToken;

use super::{
    ConfigApplication, GenerationApplication, LlmContextApplication, ProjectReadApplication,
};
use a4n_task_runtime::persistent::{PersistentTaskQueue, RunningTask};

/// Orchestrator 侧的 provider port 实现。
pub(super) struct ProviderRuntimeAdapter {
    task_queue: Arc<PersistentTaskQueue>,
    settings: ConfigApplication,
    generation: GenerationApplication,
    llm_context: LlmContextApplication,
    project_read: ProjectReadApplication,
}

impl ProviderRuntimeAdapter {
    pub(super) fn from_applications(
        task_queue: Arc<PersistentTaskQueue>,
        settings: ConfigApplication,
        generation: GenerationApplication,
        llm_context: LlmContextApplication,
        project_read: ProjectReadApplication,
    ) -> Self {
        Self {
            task_queue,
            settings,
            generation,
            llm_context,
            project_read,
        }
    }
}

#[async_trait::async_trait]
impl ProviderRuntimePort for ProviderRuntimeAdapter {
    async fn resolve_llm_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<LlmProviderConfig, AppError> {
        self.settings
            .resolve_llm_config(config_id)
            .await
            .and_then(llm_provider_config)
    }

    async fn default_embedding_config_optional(
        &self,
    ) -> Result<Option<EmbeddingProviderConfig>, AppError> {
        self.settings
            .default_embedding_config_optional()
            .await
            .and_then(|model| model.map(embedding_provider_config).transpose())
    }

    async fn default_embedding_config(&self) -> Result<EmbeddingProviderConfig, AppError> {
        self.settings
            .default_embedding_config()
            .await
            .and_then(embedding_provider_config)
    }

    async fn project_root_id(&self, project_id: i64) -> Result<Option<i64>, AppError> {
        self.project_read
            .by_id(project_id)
            .await
            .map(|model| model.map(|model| model.root_project_id.unwrap_or(project_id)))
    }

    async fn record_llm_call_trace(
        &self,
        input: a4n_domain::llm::RecordLlmCallTraceInput,
    ) -> Result<(), AppError> {
        self.generation
            .record_llm_call_trace(input)
            .await
            .map(|_| ())
    }

    async fn start_model_usage(
        &self,
        input: ModelUsageRecordStart,
    ) -> Result<ModelUsageRecordHandle, AppError> {
        self.settings
            .start_model_usage_record(input)
            .await
            .map(|record| ModelUsageRecordHandle { id: record.id })
    }

    async fn finish_model_usage(
        &self,
        handle: ModelUsageRecordHandle,
        input: ModelUsageRecordCompletion,
    ) -> Result<(), AppError> {
        self.settings
            .finish_model_usage_record(handle.id, input)
            .await
    }

    async fn record_or_reuse_context_pack_reference(
        &self,
        input: a4n_domain::llm::RecordOrReuseContextPackInput,
    ) -> Result<ContextPackReference, AppError> {
        self.llm_context
            .record_or_reuse_context_pack_reference(input)
            .await
    }

    async fn record_or_reuse_context_pack_references(
        &self,
        inputs: Vec<a4n_domain::llm::RecordOrReuseContextPackInput>,
    ) -> Result<Vec<ContextPackReference>, AppError> {
        self.llm_context
            .record_or_reuse_context_pack_references(inputs)
            .await
    }

    async fn start_task(
        &self,
        task_type: TaskKind,
        priority: ProviderTaskPriority,
        project_id: Option<i64>,
        workflow_run_id: Option<i64>,
        cancel_token: CancellationToken,
        llm_limit: Option<a4n_domain::task_queue::LlmConcurrencyLimit>,
    ) -> Result<Box<dyn TaskLease>, AppError> {
        let workflow = workflow_run_id.map(TaskWorkflowContext::new).transpose()?;
        self.task_queue
            .start_task_with_limit(
                task_type,
                queue_task_priority(priority),
                project_id,
                workflow,
                cancel_token,
                llm_limit,
            )
            .await
            .map(|task| Box::new(DurableTaskLease(task)) as Box<dyn TaskLease>)
    }
}

fn queue_task_priority(priority: ProviderTaskPriority) -> QueueTaskPriority {
    match priority {
        ProviderTaskPriority::High => QueueTaskPriority::High,
        ProviderTaskPriority::Medium => QueueTaskPriority::Medium,
        ProviderTaskPriority::Low => QueueTaskPriority::Low,
    }
}

struct DurableTaskLease(RunningTask);

#[async_trait::async_trait]
impl TaskLease for DurableTaskLease {
    fn cancellation_token(&self) -> CancellationToken {
        self.0.cancellation_token()
    }

    async fn complete(self: Box<Self>) -> Result<(), AppError> {
        self.0.complete().await
    }

    async fn fail(self: Box<Self>, error: String) -> Result<(), AppError> {
        self.0.fail(&error).await
    }

    async fn cancel(self: Box<Self>) -> Result<(), AppError> {
        self.0.cancel().await
    }
}

fn llm_provider_config(model: llm_config::Model) -> Result<LlmProviderConfig, AppError> {
    Ok(LlmProviderConfig {
        id: model.id,
        name: model.name,
        provider: model.provider,
        base_url: model.base_url,
        api_key: model.api_key_encrypted,
        model: model.model,
        max_concurrent: model.max_concurrent,
        timeout_secs: model.timeout_secs,
        max_context_tokens: model.max_context_tokens,
        max_output_tokens: model.max_output_tokens,
        extra_body_json: model.extra_body_json,
        pricing: parse_pricing(model.pricing_json)?,
    })
}

fn embedding_provider_config(
    model: embedding_config::Model,
) -> Result<EmbeddingProviderConfig, AppError> {
    Ok(EmbeddingProviderConfig {
        id: model.id,
        name: model.name,
        backend: model.backend,
        base_url: model.base_url,
        api_key: model.api_key_encrypted,
        model: model.model,
        dimension: model.dimension,
        batch_size: model.batch_size,
        local_model_path: model.local_model_path,
        pricing: parse_pricing(model.pricing_json)?,
    })
}

fn parse_pricing(value: serde_json::Value) -> Result<a4n_domain::cost::PricingPlan, AppError> {
    let pricing: a4n_domain::cost::PricingPlan =
        serde_json::from_value(value).map_err(AppError::Serialization)?;
    pricing.validate().map_err(AppError::InvalidArgument)
}
