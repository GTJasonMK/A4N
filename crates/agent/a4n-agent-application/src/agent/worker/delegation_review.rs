use super::super::AgentApplication;
use super::*;
use a4n_agent_runtime_contract::AgentWorkerDelegationReviewApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerDelegationReviewRequest, AgentWorkerDelegationReviewSnapshotWire,
};

impl AgentApplication {
    pub(crate) async fn review_delegation_request(
        &self,
        request: &AgentWorkerDelegationReviewRequest,
    ) -> Result<AgentWorkerDelegationReviewSnapshotWire, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        self.review_delegation(
            request.user_id,
            request.input.clone(),
            request.parent_run_id,
        )
        .await
    }
}

#[async_trait::async_trait]
impl AgentWorkerDelegationReviewApplicationPort for AgentApplication {
    async fn review_delegation_request(
        &self,
        request: &AgentWorkerDelegationReviewRequest,
    ) -> Result<AgentWorkerDelegationReviewSnapshotWire, AppError> {
        AgentApplication::review_delegation_request(self, request).await
    }
}
