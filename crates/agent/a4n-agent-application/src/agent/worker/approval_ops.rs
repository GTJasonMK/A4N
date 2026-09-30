//! Worker approval operations.

use super::super::{AgentApplication, AgentWorkerRun};
use super::*;
use a4n_agent_runtime_contract::AgentApprovalExpiryApplicationPort;
use a4n_agent_runtime_contract::AgentWorkerApprovalApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerApprovalCheckRequest, AgentWorkerApprovalCheckSnapshotWire,
    AgentWorkerApprovalRequest,
};
use chrono::NaiveDateTime;
use serde_json::Value;

impl AgentApplication {
    pub(crate) async fn expire_pending_approvals(
        &self,
        limit: u64,
    ) -> Result<Vec<approval::Model>, AppError> {
        agent::expire_pending_approvals(self.connection(), limit)
            .await
            .map_err(agent_db_error_to_app)
    }

    /// Returns an approved operation for the given execution.  Approval
    /// consumption is intentionally left to the side-effect owner; an
    /// approved row is evidence, not permission to mutate unrelated data.
    pub(crate) async fn approved_execution(
        &self,
        thread_id: i64,
        execution_id: i64,
    ) -> Result<Option<approval::Model>, AppError> {
        agent::approved_execution(self.connection(), thread_id, execution_id)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn approved_execution_request(
        &self,
        request: &AgentWorkerApprovalCheckRequest,
    ) -> Result<AgentWorkerApprovalCheckSnapshotWire, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        let run = agent::run_by_id(self.connection(), request.run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent run 不存在: {}", request.run_id)))?;
        let approved = self
            .approved_execution(run.thread_id, request.execution_id)
            .await?
            .is_some();
        Ok(AgentWorkerApprovalCheckSnapshotWire { approved })
    }

    /// Atomically records a write approval request and releases the claimed
    /// Run into `waiting(approval)`. Runtime workers must use this façade
    /// instead of composing the two database operations themselves.
    pub(crate) async fn request_worker_approval(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        operation_json: Value,
        resource_preconditions_json: Option<Value>,
        expires_at: Option<chrono::NaiveDateTime>,
    ) -> Result<approval::Model, AppError> {
        self.request_worker_approval_fields(
            ApprovalRunFields {
                run_id: claimed.run.id,
                thread_id: claimed.run.thread_id,
                lease_token: &claimed.lease_token,
            },
            execution_id,
            operation_json,
            resource_preconditions_json,
            expires_at,
        )
        .await
    }

    pub(super) async fn request_worker_approval_fields(
        &self,
        fields: ApprovalRunFields<'_>,
        execution_id: i64,
        operation_json: Value,
        resource_preconditions_json: Option<Value>,
        expires_at: Option<chrono::NaiveDateTime>,
    ) -> Result<approval::Model, AppError> {
        let operation_digest = a4n_db::agent::sha256_hex(
            &serde_json::to_string(&operation_json).map_err(AppError::Serialization)?,
        );
        let (approval, _, _) = agent::request_approval_for_run_lease(
            self.connection(),
            fields.lease_token,
            a4n_domain::agent::AgentApprovalInput {
                thread_id: fields.thread_id,
                run_id: fields.run_id,
                execution_id,
                operation_digest,
                operation_json,
                resource_preconditions_json,
                expires_at,
            },
        )
        .await
        .map_err(agent_db_error_to_app)?;
        Ok(approval)
    }

    pub(crate) async fn request_worker_approval_request(
        &self,
        request: &AgentWorkerApprovalRequest,
    ) -> Result<(), AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        let run = agent::run_by_id(self.connection(), request.run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent run 不存在: {}", request.run_id)))?;
        self.request_worker_approval_fields(
            ApprovalRunFields {
                run_id: run.id,
                thread_id: run.thread_id,
                lease_token: &request.lease.fence_token,
            },
            request.execution_id,
            request.operation_json.clone(),
            request.resource_preconditions_json.clone(),
            request.expires_at,
        )
        .await
        .map(|_| ())
    }
}

#[async_trait::async_trait]
impl AgentWorkerApprovalApplicationPort for AgentApplication {
    async fn approved_execution(
        &self,
        thread_id: i64,
        execution_id: i64,
    ) -> Result<bool, AppError> {
        AgentApplication::approved_execution(self, thread_id, execution_id)
            .await
            .map(|approval| approval.is_some())
    }

    async fn approved_execution_request(
        &self,
        request: &AgentWorkerApprovalCheckRequest,
    ) -> Result<AgentWorkerApprovalCheckSnapshotWire, AppError> {
        AgentApplication::approved_execution_request(self, request).await
    }

    async fn request_worker_approval(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        operation_json: Value,
        resource_preconditions_json: Option<Value>,
        expires_at: Option<NaiveDateTime>,
    ) -> Result<(), AppError> {
        AgentApplication::request_worker_approval(
            self,
            claimed,
            execution_id,
            operation_json,
            resource_preconditions_json,
            expires_at,
        )
        .await
        .map(|_| ())
    }

    async fn request_worker_approval_request(
        &self,
        request: &AgentWorkerApprovalRequest,
    ) -> Result<(), AppError> {
        AgentApplication::request_worker_approval_request(self, request).await
    }
}

#[async_trait::async_trait]
impl AgentApprovalExpiryApplicationPort for AgentApplication {
    async fn expire_pending_approvals(&self, limit: u64) -> Result<usize, AppError> {
        Ok(AgentApplication::expire_pending_approvals(self, limit)
            .await?
            .len())
    }
}
