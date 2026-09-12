use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use a4n_domain::{
    llm::{
        ContextPackReference, LlmMaxTokensConfig, RecordLlmCallTraceInput,
        RecordOrReuseContextPackInput,
    },
    task_queue::{LlmConcurrencyLimit, TaskKind},
};
use a4n_error::AppError;
use a4n_provider_contract::{EmbeddingProviderConfig, LlmProviderConfig};
use a4n_provider_runtime_contract::{ProviderRuntimePort, ProviderTaskPriority, TaskLease};
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub(super) struct TestProviderPort {
    pub resolutions: AtomicUsize,
    pub limits: Mutex<Vec<LlmConcurrencyLimit>>,
    pub statuses: Arc<Mutex<Vec<&'static str>>>,
    pub cancel: CancellationToken,
}

struct TestTask {
    statuses: Arc<Mutex<Vec<&'static str>>>,
    cancel: CancellationToken,
}

#[async_trait]
impl TaskLease for TestTask {
    fn cancellation_token(&self) -> CancellationToken {
        self.cancel.clone()
    }
    async fn complete(self: Box<Self>) -> Result<(), AppError> {
        self.statuses.lock().unwrap().push("completed");
        Ok(())
    }
    async fn fail(self: Box<Self>, _error: String) -> Result<(), AppError> {
        self.statuses.lock().unwrap().push("failed");
        Ok(())
    }
    async fn cancel(self: Box<Self>) -> Result<(), AppError> {
        self.statuses.lock().unwrap().push("cancelled");
        Ok(())
    }
}

#[async_trait]
impl ProviderRuntimePort for TestProviderPort {
    async fn resolve_llm_config(&self, _id: Option<i64>) -> Result<LlmProviderConfig, AppError> {
        self.resolutions.fetch_add(1, Ordering::SeqCst);
        Ok(super::test_llm_model())
    }
    async fn llm_max_tokens_config(&self) -> Result<LlmMaxTokensConfig, AppError> {
        Ok(Default::default())
    }
    async fn default_embedding_config_optional(
        &self,
    ) -> Result<Option<EmbeddingProviderConfig>, AppError> {
        unreachable!("LLM test must not access embeddings")
    }
    async fn default_embedding_config(&self) -> Result<EmbeddingProviderConfig, AppError> {
        unreachable!("LLM test must not access embeddings")
    }
    async fn project_root_id(&self, project_id: i64) -> Result<Option<i64>, AppError> {
        Ok(Some(project_id))
    }
    async fn record_llm_call_trace(&self, _input: RecordLlmCallTraceInput) -> Result<(), AppError> {
        Ok(())
    }
    async fn record_or_reuse_context_pack_reference(
        &self,
        input: RecordOrReuseContextPackInput,
    ) -> Result<ContextPackReference, AppError> {
        Ok(ContextPackReference {
            id: 1,
            pack_kind: input.pack_kind,
            content_sha256: input.content_sha256,
            compression_level: input.compression_level,
            token_estimate: i32::try_from(input.token_estimate).unwrap(),
        })
    }
    async fn record_or_reuse_context_pack_references(
        &self,
        inputs: Vec<RecordOrReuseContextPackInput>,
    ) -> Result<Vec<ContextPackReference>, AppError> {
        let mut references = Vec::new();
        for input in inputs {
            references.push(self.record_or_reuse_context_pack_reference(input).await?);
        }
        Ok(references)
    }
    async fn start_task(
        &self,
        task_type: TaskKind,
        _priority: ProviderTaskPriority,
        _project: Option<i64>,
        _workflow: Option<i64>,
        _cancel: CancellationToken,
        limit: Option<LlmConcurrencyLimit>,
    ) -> Result<Box<dyn TaskLease>, AppError> {
        assert_eq!(task_type, TaskKind::LlmCall);
        self.limits
            .lock()
            .unwrap()
            .push(limit.expect("model calls must use resolved concurrency"));
        Ok(Box::new(TestTask {
            statuses: self.statuses.clone(),
            cancel: self.cancel.clone(),
        }))
    }
}
