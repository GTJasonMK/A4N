use super::super::{AgentApplication, AgentWorkerRun};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerRecoveryApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerRecoveryRequest;

impl AgentApplication {
    /// Converts expired leases into the explicit recovery terminal state.
    pub(crate) async fn recover_expired_runs(
        &self,
        limit: u64,
        error_message: Option<&str>,
    ) -> Result<Vec<agent::AgentRunRecord>, AppError> {
        let rows = agent::expired_running_runs(self.executor(), limit)
            .await
            .map_err(agent_db_error_to_app)?;
        let mut recovered = Vec::with_capacity(rows.len());
        for row in rows {
            match agent::recover_expired_run(self.executor(), row.id, error_message).await {
                Ok(recovered_row) => recovered.push(recovered_row),
                // Another worker may win the race between the read-only scan
                // and the fenced recovery.  Surface all other failures.
                Err(AgentDbError::Conflict(_)) => {}
                Err(error) => return Err(agent_db_error_to_app(error)),
            }
        }
        Ok(recovered)
    }

    pub(crate) async fn mark_recovery_required(
        &self,
        claimed: &AgentWorkerRun,
        reason: impl Into<String>,
    ) -> Result<agent::AgentRunRecord, AppError> {
        agent::recovery_required_for_lease(
            self.executor(),
            claimed.run.id,
            &claimed.lease_token,
            Some(reason.into()),
        )
        .await
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn mark_recovery_required_request(
        &self,
        request: &AgentWorkerRecoveryRequest,
    ) -> Result<(), AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        agent::recovery_required_for_lease(
            self.executor(),
            request.run_id,
            &request.lease.fence_token,
            Some(request.reason.clone()),
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }
}

#[async_trait::async_trait]
impl AgentRunLeaseRecoveryApplicationPort for AgentApplication {
    async fn recover_expired_runs(
        &self,
        limit: u64,
        error_message: Option<&str>,
    ) -> Result<usize, AppError> {
        Ok(
            AgentApplication::recover_expired_runs(self, limit, error_message)
                .await?
                .len(),
        )
    }
}

#[async_trait::async_trait]
impl AgentWorkerRecoveryApplicationPort for AgentApplication {
    async fn mark_recovery_required(
        &self,
        claimed: &AgentWorkerRun,
        reason: String,
    ) -> Result<(), AppError> {
        AgentApplication::mark_recovery_required(self, claimed, reason)
            .await
            .map(|_| ())
    }

    async fn mark_recovery_required_request(
        &self,
        request: &AgentWorkerRecoveryRequest,
    ) -> Result<(), AppError> {
        AgentApplication::mark_recovery_required_request(self, request).await
    }
}
