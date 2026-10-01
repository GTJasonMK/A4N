//! Typed Host adapter for the administrative worker.

mod mutations;
mod reads;

use a4n_admin_worker_contract::ADMIN_WORKER_ID;
use a4n_error::AppError;
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47679";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

#[derive(Clone)]
pub struct AdminWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for AdminWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdminWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl AdminWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, ADMIN_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_ADMIN_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_ADMIN_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        caller_user_id: i64,
        operation: &str,
        payload: I,
    ) -> Result<O, AppError>
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
                kind: WorkerPrincipalKind::User,
                subject: caller_user_id.to_string(),
            },
            operation_id: operation.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: ADMIN_WORKER_ID.into(),
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
                AppError::ConfigError(format!("admin worker 完成响应缺少 payload: {operation}"))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation,
            )),
            status => Err(AppError::ConfigError(format!(
                "admin worker 返回未完成状态: operation={operation}, status={status:?}"
            ))),
        }
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    match error {
        WorkerClientError::Timeout => AppError::Timeout("admin worker 请求超时".into()),
        WorkerClientError::Transport(error) => {
            AppError::Timeout(format!("admin worker 请求失败: {error}"))
        }
        WorkerClientError::HttpStatus { status } if status >= 500 => {
            AppError::Timeout(format!("admin worker 返回 HTTP {status}"))
        }
        error => AppError::ConfigError(format!("admin worker 请求失败: {error}")),
    }
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    match error.code {
        WorkerErrorCode::InvalidRequest => AppError::InvalidArgument(error.message.clone()),
        WorkerErrorCode::Unauthorized => AppError::Forbidden(error.message.clone()),
        WorkerErrorCode::NotFound => AppError::NotFound(error.message.clone()),
        WorkerErrorCode::Conflict | WorkerErrorCode::VersionMismatch => AppError::InvalidState {
            current: error.message.clone(),
            operation: operation.to_string(),
        },
        WorkerErrorCode::Unavailable | WorkerErrorCode::Timeout if error.retryable => {
            AppError::Timeout(format!("admin worker {operation} 失败: {}", error.message))
        }
        WorkerErrorCode::Cancelled => AppError::Cancelled,
        _ => AppError::ConfigError(format!("admin worker {operation} 失败: {}", error.message)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_admin_worker_contract::ADMIN_OVERVIEW_OPERATION;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = AdminWorkerClient::new(
            "https://admin-worker.example.test",
            Some("admin-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("admin-worker.example.test"));
        assert!(!debug.contains("admin-token"));
    }

    #[test]
    fn retryable_worker_failures_remain_retryable() {
        assert!(matches!(
            worker_client_error(WorkerClientError::Timeout),
            AppError::Timeout(_)
        ));
        assert!(matches!(
            worker_client_error(WorkerClientError::HttpStatus { status: 503 }),
            AppError::Timeout(_)
        ));
        assert!(matches!(
            worker_response_error(
                &WorkerError {
                    code: WorkerErrorCode::Unavailable,
                    message: "database unavailable".into(),
                    retryable: true,
                },
                ADMIN_OVERVIEW_OPERATION,
            ),
            AppError::Timeout(_)
        ));
    }
}
