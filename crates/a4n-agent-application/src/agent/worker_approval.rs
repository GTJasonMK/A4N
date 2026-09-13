use super::{AgentApplication, AgentWorkerRun};
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerApprovalCheckRequest, AgentWorkerApprovalCheckSnapshotWire,
    AgentWorkerApprovalRequest,
};
use chrono::NaiveDateTime;
use serde_json::Value;

/// Runtime-facing boundary for Agent approval evidence and requests.
#[async_trait::async_trait]
pub trait AgentWorkerApprovalApplicationPort: Send + Sync {
    async fn approved_execution(&self, thread_id: i64, execution_id: i64)
    -> Result<bool, AppError>;

    async fn approved_execution_request(
        &self,
        request: &AgentWorkerApprovalCheckRequest,
    ) -> Result<AgentWorkerApprovalCheckSnapshotWire, AppError>;

    async fn request_worker_approval(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        operation_json: Value,
        resource_preconditions_json: Option<Value>,
        expires_at: Option<NaiveDateTime>,
    ) -> Result<(), AppError>;

    async fn request_worker_approval_request(
        &self,
        request: &AgentWorkerApprovalRequest,
    ) -> Result<(), AppError>;
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
