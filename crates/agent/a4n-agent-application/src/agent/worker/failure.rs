use super::super::{AgentApplication, AgentWorkerRun};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerFailureApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerFailureRequest;

impl AgentApplication {
    pub(super) async fn fail_claimed_run(
        &self,
        claimed: &AgentWorkerRun,
        error_message: impl Into<String>,
    ) -> Result<agent::AgentRunRecord, AppError> {
        let error_message = error_message.into();
        agent::fail_run_for_lease(
            self.executor(),
            claimed.run.id,
            &claimed.lease_token,
            Some(error_message),
        )
        .await
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn fail_child_run(
        &self,
        claimed: &AgentWorkerRun,
        error_message: impl Into<String>,
    ) -> Result<(), AppError> {
        if claimed.run.parent_run_id.is_none() {
            self.fail_claimed_run(claimed, error_message).await?;
            return Ok(());
        }
        agent::fail_child_delegation(
            self.executor(),
            a4n_db::agent::AgentChildFailureInput {
                child_run_id: claimed.run.id,
                child_lease_token: claimed.lease_token.clone(),
                error_message: error_message.into(),
            },
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn fail_worker_run(
        &self,
        request: &AgentWorkerFailureRequest,
    ) -> Result<(), AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        let run = agent::run_by_id(self.executor(), request.run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent run 不存在: {}", request.run_id)))?;
        let result = if run.parent_run_id.is_none() {
            agent::fail_run_for_lease(
                self.executor(),
                request.run_id,
                &request.lease.fence_token,
                Some(request.error_message.clone()),
            )
            .await
            .map(|_| ())
            .map_err(agent_db_error_to_app)
        } else {
            agent::fail_child_delegation(
                self.executor(),
                a4n_db::agent::AgentChildFailureInput {
                    child_run_id: request.run_id,
                    child_lease_token: request.lease.fence_token.clone(),
                    error_message: request.error_message.clone(),
                },
            )
            .await
            .map(|_| ())
            .map_err(agent_db_error_to_app)
        };
        result?;
        agent::cancel_run_tree(self.executor(), run.id, Some(&request.error_message))
            .await
            .map_err(agent_db_error_to_app)?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl AgentWorkerFailureApplicationPort for AgentApplication {
    async fn fail_child_run(
        &self,
        claimed: &AgentWorkerRun,
        error_message: String,
    ) -> Result<(), AppError> {
        AgentApplication::fail_child_run(self, claimed, error_message).await
    }

    async fn fail_worker_run(&self, request: &AgentWorkerFailureRequest) -> Result<(), AppError> {
        AgentApplication::fail_worker_run(self, request).await
    }
}
