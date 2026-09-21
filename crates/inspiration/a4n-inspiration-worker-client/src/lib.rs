//! Typed Host adapter for the inspiration history worker.

use a4n_api_contract::inspiration::{
    InspirationHistoryInput, InspirationHistoryReadApplicationPort, InspirationMessageSummary,
};
use a4n_error::AppError;
use a4n_inspiration_worker_contract::{
    INSPIRATION_HISTORY_OPERATION, INSPIRATION_HISTORY_WORKER_ID,
};
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47666";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

#[derive(Clone)]
pub struct InspirationWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for InspirationWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InspirationWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
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
        let url = std::env::var("A4N_INSPIRATION_HISTORY_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_INSPIRATION_HISTORY_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<O>(&self, user_id: i64, input: InspirationHistoryInput) -> Result<O, AppError>
    where
        O: serde::de::DeserializeOwned,
    {
        if user_id <= 0 {
            return Err(AppError::InvalidArgument("user_id 必须大于 0".into()));
        }
        if input.project_id <= 0 {
            return Err(AppError::InvalidArgument("project_id 必须大于 0".into()));
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: user_id.to_string(),
            },
            operation_id: INSPIRATION_HISTORY_OPERATION.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: Some(input.project_id),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: INSPIRATION_HISTORY_WORKER_ID.into(),
                epoch: 1,
                fence_token: CONTROL_LEASE_FENCE.into(),
                expires_at: CONTROL_LEASE_EXPIRY.into(),
            },
            payload: input,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(INSPIRATION_HISTORY_OPERATION, request)
            .await
            .map_err(worker_client_error)?;
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

#[async_trait::async_trait]
impl InspirationHistoryReadApplicationPort for InspirationWorkerClient {
    async fn history(
        &self,
        user_id: i64,
        input: InspirationHistoryInput,
    ) -> Result<Vec<InspirationMessageSummary>, AppError> {
        self.command(user_id, input).await
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("inspiration history worker 请求失败: {error}"))
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
