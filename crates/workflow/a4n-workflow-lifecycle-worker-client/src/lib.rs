//! Typed Host adapter for the workflow lifecycle worker.

use a4n_error::AppError;
#[cfg(test)]
use a4n_worker_client::WorkerClientError;
use a4n_worker_client::{HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};
use a4n_workflow_lifecycle_worker_contract::WORKFLOW_LIFECYCLE_WORKER_ID;

mod mutations;

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47678";

#[derive(Clone, Debug)]
pub struct WorkflowLifecycleWorkerClient {
    transport: HttpWorkerClient,
}

impl WorkflowLifecycleWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, WORKFLOW_LIFECYCLE_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_WORKFLOW_LIFECYCLE_WORKER_URL",
            "A4N_WORKFLOW_LIFECYCLE_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
        Self::new(url, token)
    }

    async fn command<I, O>(&self, operation: &str, payload: I) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::Service,
                subject: "runtime".into(),
            },
            operation_id: operation.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease::control_request(WORKFLOW_LIFECYCLE_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "workflow lifecycle worker",
                    WorkerClientErrorPolicy::TransientAsTimeout,
                )
            })?;
        a4n_worker_client::worker_response_payload(
            response,
            "workflow lifecycle worker",
            Some(operation),
            |error| worker_response_error(error, operation),
        )
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
                "workflow lifecycle worker {operation} 失败: {}",
                error.message
            ))
        }
        WorkerErrorCode::Cancelled => AppError::Cancelled,
        _ => AppError::ConfigError(format!(
            "workflow lifecycle worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_workflow_lifecycle_worker_contract::WORKFLOW_LIFECYCLE_CANCEL_OPERATION;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = WorkflowLifecycleWorkerClient::new(
            "https://workflow-lifecycle-worker.example.test",
            Some("lifecycle-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("workflow-lifecycle-worker.example.test"));
        assert!(!debug.contains("lifecycle-token"));
    }

    #[test]
    fn retryable_worker_failures_remain_retryable() {
        assert!(matches!(
            map_worker_client_error(
                WorkerClientError::Timeout,
                "workflow lifecycle worker",
                WorkerClientErrorPolicy::TransientAsTimeout,
            ),
            AppError::Timeout(_)
        ));
        assert!(matches!(
            map_worker_client_error(
                WorkerClientError::HttpStatus { status: 503 },
                "workflow lifecycle worker",
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
                WORKFLOW_LIFECYCLE_CANCEL_OPERATION,
            ),
            AppError::Timeout(_)
        ));
    }
}
