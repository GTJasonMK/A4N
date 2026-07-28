//! 动态 LLM / Embedding provider 的窄依赖 facade。
//!
//! 只克隆队列与相关领域 owner，不暴露完整 `OrchestratorContext`。

use std::future::Future;
use std::sync::Arc;

use a4n_core::{
    config::app_config::LlmMaxTokensConfig,
    error::AppError,
    models::{
        ai::{embedding_config, llm_config},
        generation::{llm_call_trace, llm_context_pack},
        project::novel_project,
    },
};
use a4n_db::generation::RecordLlmCallTraceInput;
use a4n_db::llm_context::RecordOrReuseContextPackInput;
use a4n_queue::priority::{TaskKind, TaskPermit, TaskPriority, TaskQueue};
use tokio_util::sync::CancellationToken;

use super::{
    ConfigApplication, GenerationApplication, LlmContextApplication, OrchestratorContext,
    ProjectReadApplication,
};
use crate::task_queue::{PersistentTaskQueue, SubmittedTaskGuard};

/// 动态 provider 运行期依赖：队列 + settings/generation/llm_context/project_read owner。
#[derive(Clone)]
pub struct DynamicProviderFacade {
    task_queue: Arc<PersistentTaskQueue>,
    settings: ConfigApplication,
    generation: GenerationApplication,
    llm_context: LlmContextApplication,
    project_read: ProjectReadApplication,
}

impl OrchestratorContext {
    /// 构造动态 provider 窄 facade；不克隆完整上下文。
    pub fn dynamic_provider_facade(&self) -> DynamicProviderFacade {
        DynamicProviderFacade {
            task_queue: Arc::clone(&self.task_queue),
            settings: self.settings.clone(),
            generation: self.generation.clone(),
            llm_context: self.llm_context.clone(),
            project_read: self.project_read.clone(),
        }
    }
}

impl DynamicProviderFacade {
    pub async fn resolve_llm_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<llm_config::Model, AppError> {
        self.settings.resolve_llm_config(config_id).await
    }

    pub async fn llm_max_tokens_config(&self) -> Result<LlmMaxTokensConfig, AppError> {
        self.settings.llm_max_tokens_config().await
    }

    pub async fn default_embedding_config_optional(
        &self,
    ) -> Result<Option<embedding_config::Model>, AppError> {
        self.settings.default_embedding_config_optional().await
    }

    pub async fn default_embedding_config(&self) -> Result<embedding_config::Model, AppError> {
        self.settings.default_embedding_config().await
    }

    pub async fn project_by_id(
        &self,
        project_id: i64,
    ) -> Result<Option<novel_project::Model>, AppError> {
        self.project_read.by_id(project_id).await
    }

    pub async fn record_llm_call_trace(
        &self,
        input: RecordLlmCallTraceInput,
    ) -> Result<llm_call_trace::Model, AppError> {
        self.generation.record_llm_call_trace(input).await
    }

    pub async fn record_or_reuse_context_pack(
        &self,
        input: RecordOrReuseContextPackInput,
    ) -> Result<llm_context_pack::Model, AppError> {
        self.llm_context.record_or_reuse_context_pack(input).await
    }

    pub async fn queue_run_tracked<F, Fut, T>(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        project_id: Option<i64>,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.task_queue
            .run_tracked(task_type, priority, project_id, operation)
            .await
    }

    pub async fn queue_submit(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        TaskQueue::submit(self.task_queue.as_ref(), task_type, priority, project_id).await
    }

    pub fn queue_submitted_task_guard(&self, task_id: String) -> SubmittedTaskGuard {
        self.task_queue.submitted_task_guard(task_id)
    }

    pub async fn queue_acquire_task_with_cancel(
        &self,
        task_id: &str,
        cancel_token: CancellationToken,
    ) -> Result<Option<TaskPermit>, AppError> {
        self.task_queue
            .acquire_task_with_cancel(task_id, cancel_token)
            .await
    }

    pub async fn queue_cancel(&self, task_id: &str) -> Result<(), AppError> {
        TaskQueue::cancel(self.task_queue.as_ref(), task_id).await
    }

    pub async fn queue_mark_done(&self, task_id: &str) -> Result<(), AppError> {
        self.task_queue.mark_done(task_id).await
    }

    pub async fn queue_mark_failed(&self, task_id: &str, error: &str) -> Result<(), AppError> {
        self.task_queue.mark_failed(task_id, error).await
    }
}
