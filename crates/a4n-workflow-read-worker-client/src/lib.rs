//! Typed Host adapter for the workflow read worker.

use a4n_api_contract::task::{TaskInfoSummary, TaskReadApplicationPort};
use a4n_api_contract::workflow::{
    GetWorkflowRunInput, ListWorkflowRunsInput, WorkflowReadApplicationPort,
    WorkflowRunDetailSummary, WorkflowRunSummary,
};
use a4n_error::AppError;
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};
use a4n_workflow_read_worker_contract::{
    GetTaskHistoryInput, TASK_READ_HISTORY_OPERATION, WORKFLOW_READ_DETAIL_OPERATION,
    WORKFLOW_READ_LIST_OPERATION, WORKFLOW_READ_WORKER_ID,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47672";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

#[derive(Clone)]
pub struct WorkflowReadWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for WorkflowReadWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkflowReadWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl WorkflowReadWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, WORKFLOW_READ_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_WORKFLOW_READ_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_WORKFLOW_READ_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        principal: WorkerPrincipal,
        project_id: Option<i64>,
        operation: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        if project_id.is_some_and(|id| id <= 0) {
            return Err(AppError::InvalidArgument("project_id 必须大于 0".into()));
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal,
            operation_id: operation.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: WORKFLOW_READ_WORKER_ID.into(),
                epoch: 1,
                fence_token: CONTROL_LEASE_FENCE.into(),
                expires_at: CONTROL_LEASE_EXPIRY.into(),
            },
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation, request)
            .await
            .map_err(worker_client_error)?;
        match response.status {
            WorkerOperationStatus::Completed => response.payload.ok_or_else(|| {
                AppError::ConfigError(format!(
                    "workflow read worker 完成响应缺少 payload: {operation}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation,
            )),
            status => Err(AppError::ConfigError(format!(
                "workflow read worker 返回未完成状态: operation={operation}, status={status:?}"
            ))),
        }
    }

    fn user_principal(user_id: i64) -> Result<WorkerPrincipal, AppError> {
        if user_id <= 0 {
            return Err(AppError::InvalidArgument("user_id 必须大于 0".into()));
        }
        Ok(WorkerPrincipal {
            kind: WorkerPrincipalKind::User,
            subject: user_id.to_string(),
        })
    }

    fn service_principal() -> WorkerPrincipal {
        WorkerPrincipal {
            kind: WorkerPrincipalKind::Service,
            subject: "runtime".into(),
        }
    }
}

#[async_trait::async_trait]
impl WorkflowReadApplicationPort for WorkflowReadWorkerClient {
    async fn list_for_user(
        &self,
        user_id: i64,
        input: ListWorkflowRunsInput,
    ) -> Result<Vec<WorkflowRunSummary>, AppError> {
        self.command(
            Self::user_principal(user_id)?,
            input.project_id,
            WORKFLOW_READ_LIST_OPERATION,
            input,
        )
        .await
    }

    async fn detail_for_user(
        &self,
        user_id: i64,
        input: GetWorkflowRunInput,
    ) -> Result<WorkflowRunDetailSummary, AppError> {
        self.command(
            Self::user_principal(user_id)?,
            None,
            WORKFLOW_READ_DETAIL_OPERATION,
            input,
        )
        .await
    }
}

#[async_trait::async_trait]
impl TaskReadApplicationPort for WorkflowReadWorkerClient {
    async fn history(
        &self,
        project_id: Option<i64>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<TaskInfoSummary>, AppError> {
        self.command(
            Self::service_principal(),
            project_id,
            TASK_READ_HISTORY_OPERATION,
            GetTaskHistoryInput {
                project_id,
                offset,
                limit,
            },
        )
        .await
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("workflow read worker 请求失败: {error}"))
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    match error.code {
        WorkerErrorCode::InvalidRequest => AppError::InvalidArgument(error.message.clone()),
        WorkerErrorCode::Unauthorized => AppError::Forbidden(error.message.clone()),
        WorkerErrorCode::NotFound => AppError::NotFound(error.message.clone()),
        WorkerErrorCode::Conflict => AppError::InvalidState {
            current: error.message.clone(),
            operation: operation.into(),
        },
        _ => AppError::ConfigError(format!(
            "workflow read worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = WorkflowReadWorkerClient::new(
            "https://workflow-read-worker.example.test",
            Some("workflow-read-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("workflow-read-worker.example.test"));
        assert!(!debug.contains("workflow-read-token"));
    }
}
