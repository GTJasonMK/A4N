use super::super::{
    AgentApplication, AgentThreadSnapshot, EnsureCanonicalSubagentThreadInput, thread_snapshot,
};
use a4n_agent_runtime_contract::AgentWorkerSubagentThreadApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerDelegationThreadRequest, AgentWorkerDelegationThreadSnapshotWire,
    AgentWorkerRunLeaseRequest,
};

impl AgentApplication {
    pub(crate) async fn ensure_canonical_subagent_thread_request(
        &self,
        request: &AgentWorkerDelegationThreadRequest,
    ) -> Result<AgentWorkerDelegationThreadSnapshotWire, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        let row = self
            .ensure_canonical_subagent_thread(
                request.user_id,
                EnsureCanonicalSubagentThreadInput {
                    canonical_binding_key: request.canonical_binding_key.clone(),
                    capability: request.capability.clone(),
                    title: request.title.clone(),
                    scope: request.scope.clone(),
                    bound_resource: request.bound_resource.clone(),
                    default_model_config_id: request.default_model_config_id,
                },
            )
            .await?;
        let snapshot = thread_snapshot(row)?;
        Ok(snapshot.into())
    }
}

#[async_trait::async_trait]
impl AgentWorkerSubagentThreadApplicationPort for AgentApplication {
    async fn ensure_canonical_subagent_thread_request(
        &self,
        request: &AgentWorkerDelegationThreadRequest,
    ) -> Result<AgentWorkerDelegationThreadSnapshotWire, AppError> {
        AgentApplication::ensure_canonical_subagent_thread_request(self, request).await
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
}
