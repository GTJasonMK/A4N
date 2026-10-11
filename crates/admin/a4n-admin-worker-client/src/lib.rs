//! Typed Host adapter for the administrative worker.

mod mutations;
mod reads;

use a4n_admin_worker_contract::ADMIN_WORKER_ID;
use a4n_error::AppError;
#[cfg(test)]
use a4n_worker_client::WorkerClientError;
use a4n_worker_client::{HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47679";

#[derive(Clone, Debug)]
pub struct AdminWorkerClient {
    transport: HttpWorkerClient,
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
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_ADMIN_WORKER_URL",
            "A4N_ADMIN_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
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
            lease: WorkerLease::control_request(ADMIN_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "admin worker",
                    WorkerClientErrorPolicy::TransientAsTimeout,
                )
            })?;
        a4n_worker_client::worker_response_payload(
            response,
            "admin worker",
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
            map_worker_client_error(
                WorkerClientError::Timeout,
                "admin worker",
                WorkerClientErrorPolicy::TransientAsTimeout,
            ),
            AppError::Timeout(_)
        ));
        assert!(matches!(
            map_worker_client_error(
                WorkerClientError::HttpStatus { status: 503 },
                "admin worker",
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
                ADMIN_OVERVIEW_OPERATION,
            ),
            AppError::Timeout(_)
        ));
    }

    #[test]
    fn unauthorized_worker_failures_map_to_forbidden() {
        let error = WorkerError {
            code: WorkerErrorCode::Unauthorized,
            message: "creator access required".into(),
            retryable: false,
        };

        assert!(matches!(
            worker_response_error(&error, ADMIN_OVERVIEW_OPERATION),
            AppError::Forbidden(message) if message == "creator access required"
        ));
    }
}
