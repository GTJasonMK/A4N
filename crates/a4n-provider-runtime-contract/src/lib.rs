//! Runtime-only port used by dynamic providers.
//!
//! Kept separate from provider configuration contracts so API and UI contract
//! crates do not inherit queue or Tokio dependencies.

use a4n_domain::llm::{
    ContextPackReference, RecordLlmCallTraceInput, RecordOrReuseContextPackInput,
};
use a4n_domain::task_queue::{LlmConcurrencyLimit, TaskKind};
use a4n_error::AppError;
use a4n_provider_contract::{EmbeddingProviderConfig, LlmProviderConfig};
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderTaskPriority {
    High,
    Medium,
    Low,
}

#[async_trait]
pub trait TaskLease: Send {
    fn cancellation_token(&self) -> CancellationToken;
    async fn complete(self: Box<Self>) -> Result<(), AppError>;
    async fn fail(self: Box<Self>, error: String) -> Result<(), AppError>;
    async fn cancel(self: Box<Self>) -> Result<(), AppError>;
}

#[async_trait]
pub trait ProviderRuntimePort: Send + Sync {
    async fn resolve_llm_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<LlmProviderConfig, AppError>;

    async fn default_embedding_config_optional(
        &self,
    ) -> Result<Option<EmbeddingProviderConfig>, AppError>;

    async fn default_embedding_config(&self) -> Result<EmbeddingProviderConfig, AppError>;

    async fn project_root_id(&self, project_id: i64) -> Result<Option<i64>, AppError>;

    async fn record_llm_call_trace(&self, input: RecordLlmCallTraceInput) -> Result<(), AppError>;

    async fn record_or_reuse_context_pack_reference(
        &self,
        input: RecordOrReuseContextPackInput,
    ) -> Result<ContextPackReference, AppError>;

    async fn record_or_reuse_context_pack_references(
        &self,
        inputs: Vec<RecordOrReuseContextPackInput>,
    ) -> Result<Vec<ContextPackReference>, AppError>;

    async fn start_task(
        &self,
        task_type: TaskKind,
        priority: ProviderTaskPriority,
        project_id: Option<i64>,
        workflow_run_id: Option<i64>,
        cancel_token: CancellationToken,
        llm_limit: Option<LlmConcurrencyLimit>,
    ) -> Result<Box<dyn TaskLease>, AppError>;
}
