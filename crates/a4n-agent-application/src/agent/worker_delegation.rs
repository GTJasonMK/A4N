use super::{
    AgentApplication, AgentThreadSnapshot, AgentWorkerRun, EnsureCanonicalSubagentThreadInput,
    WorkerDelegationInput, WorkerDelegationResult, thread_snapshot,
};
use a4n_api_contract::agent::{AgentDelegationView, AgentScope, ReviewAgentDelegationInput};
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerDelegationReviewRequest, AgentWorkerDelegationReviewSnapshotWire,
    AgentWorkerDelegationStartRequest, AgentWorkerDelegationStartSnapshotWire,
    AgentWorkerDelegationThreadRequest, AgentWorkerDelegationThreadSnapshotWire,
    AgentWorkerEventRequest,
};
use serde_json::Value;

/// Runtime-facing boundary for starting a child Agent delegation.
#[async_trait::async_trait]
pub trait AgentWorkerDelegationApplicationPort: Send + Sync {
    async fn review_delegation(
        &self,
        user_id: i64,
        input: ReviewAgentDelegationInput,
        parent_run_id: Option<i64>,
    ) -> Result<AgentDelegationView, AppError>;

    async fn review_delegation_request(
        &self,
        request: &AgentWorkerDelegationReviewRequest,
    ) -> Result<AgentWorkerDelegationReviewSnapshotWire, AppError>;

    async fn ensure_canonical_subagent_thread_request(
        &self,
        request: &AgentWorkerDelegationThreadRequest,
    ) -> Result<AgentWorkerDelegationThreadSnapshotWire, AppError>;

    async fn append_worker_update_request(
        &self,
        request: &AgentWorkerEventRequest,
    ) -> Result<(), AppError>;

    async fn create_child_run_request(
        &self,
        request: &AgentWorkerDelegationStartRequest,
    ) -> Result<AgentWorkerDelegationStartSnapshotWire, AppError>;

    fn inherit_subagent_allowlists(
        &self,
        parent: &AgentScope,
        child: &mut AgentScope,
    ) -> Result<(), AppError>;

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
    async fn review_delegation(
        &self,
        user_id: i64,
        input: ReviewAgentDelegationInput,
        parent_run_id: Option<i64>,
    ) -> Result<AgentDelegationView, AppError> {
        AgentApplication::review_delegation(self, user_id, input, parent_run_id).await
    }

    async fn review_delegation_request(
        &self,
        request: &AgentWorkerDelegationReviewRequest,
    ) -> Result<AgentWorkerDelegationReviewSnapshotWire, AppError> {
        AgentApplication::review_delegation_request(self, request).await
    }

    async fn ensure_canonical_subagent_thread_request(
        &self,
        request: &AgentWorkerDelegationThreadRequest,
    ) -> Result<AgentWorkerDelegationThreadSnapshotWire, AppError> {
        AgentApplication::ensure_canonical_subagent_thread_request(self, request).await
    }

    async fn append_worker_update_request(
        &self,
        request: &AgentWorkerEventRequest,
    ) -> Result<(), AppError> {
        AgentApplication::append_worker_update_request(self, request).await
    }

    async fn create_child_run_request(
        &self,
        request: &AgentWorkerDelegationStartRequest,
    ) -> Result<AgentWorkerDelegationStartSnapshotWire, AppError> {
        AgentApplication::create_child_run_request(self, request).await
    }

    fn inherit_subagent_allowlists(
        &self,
        parent: &AgentScope,
        child: &mut AgentScope,
    ) -> Result<(), AppError> {
        super::inherit_subagent_allowlists(parent, child)
    }

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
