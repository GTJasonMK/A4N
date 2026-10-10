//! Typed Host adapter for the inspiration history worker.

use a4n_api_contract::inspiration::InspirationHistoryInput;
use a4n_error::AppError;
use a4n_inspiration_worker_contract::{
    INSPIRATION_HISTORY_OPERATION, INSPIRATION_HISTORY_WORKER_ID,
};
use a4n_worker_client::{HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47666";

mod reads;

#[derive(Clone, Debug)]
pub struct InspirationWorkerClient {
    transport: HttpWorkerClient,
}

impl InspirationWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, INSPIRATION_HISTORY_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_INSPIRATION_HISTORY_WORKER_URL",
            "A4N_INSPIRATION_HISTORY_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
        Self::new(url, token)
    }

    async fn command<O>(&self, user_id: i64, input: InspirationHistoryInput) -> Result<O, AppError>
    where
        O: serde::de::DeserializeOwned,
    {
        let principal = a4n_worker_client::user_principal(user_id)?;
        if input.project_id <= 0 {
            return Err(AppError::InvalidArgument("project_id 必须大于 0".into()));
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal,
            operation_id: INSPIRATION_HISTORY_OPERATION.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: Some(input.project_id),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease::control_request(INSPIRATION_HISTORY_WORKER_ID),
            payload: input,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(INSPIRATION_HISTORY_OPERATION, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "inspiration history worker",
                    WorkerClientErrorPolicy::Configuration,
                )
            })?;
        match response.status {
            WorkerOperationStatus::Completed => response.payload.ok_or_else(|| {
                AppError::ConfigError("inspiration history worker 完成响应缺少 payload".into())
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
            )),
            status => Err(AppError::ConfigError(format!(
                "inspiration history worker 返回未完成状态: {status:?}"
            ))),
        }
    }
}

fn worker_response_error(error: &WorkerError) -> AppError {
    match error.code {
        WorkerErrorCode::InvalidRequest => AppError::InvalidArgument(error.message.clone()),
        WorkerErrorCode::Unauthorized => AppError::Forbidden(error.message.clone()),
        WorkerErrorCode::NotFound => AppError::NotFound(error.message.clone()),
        WorkerErrorCode::Conflict => AppError::InvalidState {
            current: error.message.clone(),
            operation: INSPIRATION_HISTORY_OPERATION.into(),
        },
        _ => AppError::ConfigError(format!(
            "inspiration history worker 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = InspirationWorkerClient::new(
            "https://inspiration-history-worker.example.test",
            Some("inspiration-history-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("inspiration-history-worker.example.test"));
        assert!(!debug.contains("inspiration-history-token"));
    }
}
