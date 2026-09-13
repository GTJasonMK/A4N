use super::{
    AgentApplication, AgentExecutionSnapshot, WorkerExecutionInput, WorkerExecutionUpdate,
};
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerExecutionCreateRequest, AgentWorkerExecutionLookupRequest,
    AgentWorkerExecutionUpdateRequest,
};

/// Runtime-facing boundary for the durable execution record lifecycle of one
/// claimed Agent Run.
#[async_trait::async_trait]
pub trait AgentWorkerExecutionApplicationPort: Send + Sync {
    async fn worker_execution_by_key(
        &self,
        run_id: i64,
        key: &str,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError>;

    async fn create_prepared_execution(
        &self,
        input: WorkerExecutionInput,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError>;

    async fn update_worker_execution(
        &self,
        input: WorkerExecutionUpdate,
    ) -> Result<AgentExecutionSnapshot, AppError>;

    async fn worker_execution_by_key_request(
        &self,
        request: &AgentWorkerExecutionLookupRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError>;

    async fn create_prepared_execution_request(
        &self,
        request: &AgentWorkerExecutionCreateRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError>;

    async fn update_worker_execution_request(
        &self,
        request: &AgentWorkerExecutionUpdateRequest,
    ) -> Result<AgentExecutionSnapshot, AppError>;
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
