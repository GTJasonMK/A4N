use super::{
    AgentApplication, AgentThreadSnapshot, AgentWorkerRun, EnsureCanonicalSubagentThreadInput,
    WorkerDelegationInput, WorkerDelegationResult, thread_snapshot,
};
use a4n_error::AppError;
use serde_json::Value;

/// Runtime-facing boundary for starting a child Agent delegation.
#[async_trait::async_trait]
pub trait AgentWorkerDelegationApplicationPort: Send + Sync {
    async fn ensure_canonical_subagent_thread(
        &self,
        user_id: i64,
        input: EnsureCanonicalSubagentThreadInput,
    ) -> Result<AgentThreadSnapshot, AppError>;

    async fn append_worker_update(
        &self,
        claimed: &AgentWorkerRun,
        kind: &str,
        status: &str,
        public_text: String,
        payload_json: Option<Value>,
    ) -> Result<(), AppError>;

    async fn create_child_run(
        &self,
        claimed: &AgentWorkerRun,
        input: WorkerDelegationInput,
    ) -> Result<WorkerDelegationResult, AppError>;
}

#[async_trait::async_trait]
impl AgentWorkerDelegationApplicationPort for AgentApplication {
    async fn ensure_canonical_subagent_thread(
        &self,
        user_id: i64,
        input: EnsureCanonicalSubagentThreadInput,
    ) -> Result<AgentThreadSnapshot, AppError> {
        AgentApplication::ensure_canonical_subagent_thread(self, user_id, input)
            .await
            .and_then(thread_snapshot)
    }

    async fn append_worker_update(
        &self,
        claimed: &AgentWorkerRun,
        kind: &str,
        status: &str,
        public_text: String,
        payload_json: Option<Value>,
    ) -> Result<(), AppError> {
        AgentApplication::append_worker_update(
            self,
            claimed,
            kind,
            status,
            public_text,
            payload_json,
        )
        .await
        .map(|_| ())
    }

    async fn create_child_run(
        &self,
        claimed: &AgentWorkerRun,
        input: WorkerDelegationInput,
    ) -> Result<WorkerDelegationResult, AppError> {
        AgentApplication::create_child_run(self, claimed, input).await
    }
}
