//! Typed Host adapter for the workflow read worker.

use a4n_api_contract::task::{TaskInfoSummary, TaskReadApplicationPort};
use a4n_api_contract::workflow::{
    ActiveTaskLookupRecord, GetWorkflowRunInput, ListWorkflowRunsInput, TaskLookupRecord,
    WorkflowProgressEvent, WorkflowProgressEventsInput, WorkflowReadApplicationPort,
    WorkflowResumeRecord, WorkflowRunDetailSummary, WorkflowRunSummary, WorkflowTaskAccessRecord,
    WorkflowTaskLookupApplicationPort,
};
use a4n_error::AppError;
#[cfg(test)]
use a4n_worker_client::WorkerClientError;
use a4n_worker_client::{HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};
use a4n_workflow_read_worker_contract::{
    EmptyInput, GetTaskHistoryInput, TASK_LOOKUP_ACTIVE_OPERATION, TASK_LOOKUP_BY_ID_OPERATION,
    TASK_READ_HISTORY_OPERATION, TaskIdInput, WORKFLOW_ACCESS_ACTIVE_TASK_OPERATION,
    WORKFLOW_READ_DETAIL_OPERATION, WORKFLOW_READ_LIST_OPERATION,
    WORKFLOW_READ_PROGRESS_EVENTS_OPERATION, WORKFLOW_READ_RESUME_RECORD_OPERATION,
    WORKFLOW_READ_SUMMARY_OPERATION, WORKFLOW_READ_WORKER_ID, WorkflowRunIdInput,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47672";

mod tasks;
mod workflow;

#[derive(Clone, Debug)]
pub struct WorkflowReadWorkerClient {
    transport: HttpWorkerClient,
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
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_WORKFLOW_READ_WORKER_URL",
            "A4N_WORKFLOW_READ_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
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
        let project_id = a4n_worker_client::optional_positive_id("project_id", project_id)?;
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
            lease: WorkerLease::control_request(WORKFLOW_READ_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "workflow read worker",
                    WorkerClientErrorPolicy::TransientAsTimeout,
                )
            })?;
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

    fn service_principal() -> WorkerPrincipal {
        WorkerPrincipal {
            kind: WorkerPrincipalKind::Service,
            subject: "runtime".into(),
        }
    }

    async fn service_command<I, O>(&self, operation: &str, payload: I) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        self.command(Self::service_principal(), None, operation, payload)
            .await
    }
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    match error.code {
        WorkerErrorCode::InvalidRequest => AppError::InvalidArgument(error.message.clone()),
        WorkerErrorCode::Unauthorized => AppError::Forbidden(error.message.clone()),
        WorkerErrorCode::NotFound => AppError::NotFound(error.message.clone()),
        WorkerErrorCode::Conflict | WorkerErrorCode::VersionMismatch => AppError::InvalidState {
            current: error.message.clone(),
            operation: operation.into(),
        },
        WorkerErrorCode::Unavailable | WorkerErrorCode::Timeout if error.retryable => {
            AppError::Timeout(format!(
                "workflow read worker {operation} 失败: {}",
                error.message
            ))
        }
        WorkerErrorCode::Cancelled => AppError::Cancelled,
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

    #[test]
    fn retryable_worker_failures_remain_retryable() {
        assert!(matches!(
            map_worker_client_error(
                WorkerClientError::Timeout,
                "workflow read worker",
                WorkerClientErrorPolicy::TransientAsTimeout,
            ),
            AppError::Timeout(_)
        ));
        assert!(matches!(
            map_worker_client_error(
                WorkerClientError::HttpStatus { status: 503 },
                "workflow read worker",
                WorkerClientErrorPolicy::TransientAsTimeout,
            ),
            AppError::Timeout(_)
        ));
        assert!(matches!(
            worker_response_error(
                &WorkerError {
                    code: WorkerErrorCode::Unavailable,
                    message: "database unavailable".into(),
                    retryable: true,
                },
                WORKFLOW_READ_LIST_OPERATION,
            ),
            AppError::Timeout(_)
        ));
    }
}
