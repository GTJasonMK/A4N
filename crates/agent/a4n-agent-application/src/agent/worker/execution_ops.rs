//! Worker execution records and usage persistence.

use super::super::{
    AgentApplication, AgentExecutionSnapshot, WorkerExecutionInput, WorkerExecutionUpdate,
};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerExecutionApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerExecutionCreateRequest, AgentWorkerExecutionLookupRequest,
    AgentWorkerExecutionUpdateRequest,
};

impl AgentApplication {
    pub(crate) async fn worker_execution_by_key(
        &self,
        run_id: i64,
        key: &str,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        self.execution_by_key(run_id, key).await
    }

    pub(crate) async fn create_prepared_execution(
        &self,
        input: WorkerExecutionInput,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        match agent::create_execution_for_run_lease(
            self.connection(),
            AgentExecutionInput {
                run_id: input.run_id,
                execution_key: input.execution_key,
                execution_kind: input.execution_kind,
                status: "prepared".into(),
                tool_name: input.tool_name,
                effect: input.effect,
                replay_policy: input.replay_policy,
                arguments_json: input.arguments_json,
                resource_preconditions_json: input.resource_preconditions_json,
            },
            &input.lease_token,
        )
        .await
        {
            Ok(execution) => Ok(Some(execution_snapshot(execution))),
            Err(AgentDbError::BudgetExhausted(_)) => Ok(None),
            Err(error) => Err(agent_db_error_to_app(error)),
        }
    }

    pub(crate) async fn worker_execution_by_key_request(
        &self,
        request: &AgentWorkerExecutionLookupRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        self.worker_execution_by_key(request.run_id, &request.execution_key)
            .await
    }

    pub(crate) async fn create_prepared_execution_request(
        &self,
        request: &AgentWorkerExecutionCreateRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        self.create_prepared_execution(WorkerExecutionInput {
            run_id: request.run_id,
            lease_token: request.lease.fence_token.clone(),
            execution_key: request.execution_key.clone(),
            execution_kind: request.execution_kind.clone(),
            tool_name: request.tool_name.clone(),
            effect: request.effect.clone(),
            replay_policy: request.replay_policy.clone(),
            arguments_json: request.arguments_json.clone(),
            resource_preconditions_json: request.resource_preconditions_json.clone(),
        })
        .await
    }

    pub(crate) async fn update_worker_execution_request(
        &self,
        request: &AgentWorkerExecutionUpdateRequest,
    ) -> Result<AgentExecutionSnapshot, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        self.update_worker_execution(WorkerExecutionUpdate {
            run_id: request.run_id,
            lease_token: request.lease.fence_token.clone(),
            execution_id: request.execution_id,
            status: request.status.clone(),
            result_json: request.result_json.clone(),
            result_summary: request.result_summary.clone(),
            result_refs_json: request.result_refs_json.clone(),
            error_message: request.error_message.clone(),
        })
        .await
    }

    pub(crate) async fn update_worker_execution(
        &self,
        input: WorkerExecutionUpdate,
    ) -> Result<AgentExecutionSnapshot, AppError> {
        agent::update_execution_for_run_lease(
            self.connection(),
            input.run_id,
            &input.lease_token,
            input.execution_id,
            &input.status,
            input.result_json,
            input.result_summary,
            input.result_refs_json,
            input.error_message,
        )
        .await
        .map(execution_snapshot)
        .map_err(agent_db_error_to_app)
    }
}

#[async_trait::async_trait]
impl AgentWorkerExecutionApplicationPort for AgentApplication {
    async fn worker_execution_by_key(
        &self,
        run_id: i64,
        key: &str,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        AgentApplication::worker_execution_by_key(self, run_id, key).await
    }

    async fn create_prepared_execution(
        &self,
        input: WorkerExecutionInput,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        AgentApplication::create_prepared_execution(self, input).await
    }

    async fn update_worker_execution(
        &self,
        input: WorkerExecutionUpdate,
    ) -> Result<AgentExecutionSnapshot, AppError> {
        AgentApplication::update_worker_execution(self, input).await
    }

    async fn worker_execution_by_key_request(
        &self,
        request: &AgentWorkerExecutionLookupRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        AgentApplication::worker_execution_by_key_request(self, request).await
    }

    async fn create_prepared_execution_request(
        &self,
        request: &AgentWorkerExecutionCreateRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        AgentApplication::create_prepared_execution_request(self, request).await
    }

    async fn update_worker_execution_request(
        &self,
        request: &AgentWorkerExecutionUpdateRequest,
    ) -> Result<AgentExecutionSnapshot, AppError> {
        AgentApplication::update_worker_execution_request(self, request).await
    }
}
