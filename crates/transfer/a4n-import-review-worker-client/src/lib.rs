//! Typed Host adapter for the import-review read worker.

use a4n_api_contract::import_review::{
    CheckImportDedupInput, DedupResult, ImportProjectReviewsInput, ImportReviewIdInput,
    ImportReviewListItem, ImportReviewMutationApplicationPort, ImportReviewReadApplicationPort,
    ImportReviewSummary,
};
use a4n_error::AppError;
use a4n_import_review_worker_contract::{
    IMPORT_REVIEW_DEDUP_OPERATION, IMPORT_REVIEW_DELETE_OPERATION, IMPORT_REVIEW_DETAIL_OPERATION,
    IMPORT_REVIEW_LIST_INDEX_OPERATION, IMPORT_REVIEW_LIST_OPERATION, IMPORT_REVIEW_READ_WORKER_ID,
    IMPORT_REVIEW_REJECT_OPERATION,
};
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47665";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

mod mutations;
mod reads;

#[derive(Clone)]
pub struct ImportReviewWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for ImportReviewWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImportReviewWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl ImportReviewWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, IMPORT_REVIEW_READ_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_IMPORT_REVIEW_READ_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_IMPORT_REVIEW_READ_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        operation_id: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        if user_id <= 0 {
            return Err(AppError::InvalidArgument("user_id 必须大于 0".into()));
        }
        if project_id.is_some_and(|project_id| project_id <= 0) {
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
            operation_id: operation_id.to_string(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: IMPORT_REVIEW_READ_WORKER_ID.into(),
                epoch: 1,
                fence_token: CONTROL_LEASE_FENCE.into(),
                expires_at: CONTROL_LEASE_EXPIRY.into(),
            },
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation_id, request)
            .await
            .map_err(worker_client_error)?;
        match response.status {
            WorkerOperationStatus::Completed => response.payload.ok_or_else(|| {
                AppError::ConfigError(format!(
                    "import review read worker 完成响应缺少 payload: {operation_id}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation_id,
            )),
            status => Err(AppError::ConfigError(format!(
                "import review read worker 返回未完成状态: operation={operation_id}, status={status:?}"
            ))),
        }
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("import review read worker 请求失败: {error}"))
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    match error.code {
        WorkerErrorCode::InvalidRequest => AppError::InvalidArgument(error.message.clone()),
        WorkerErrorCode::Unauthorized => AppError::Forbidden(error.message.clone()),
        WorkerErrorCode::NotFound => AppError::NotFound(error.message.clone()),
        WorkerErrorCode::Conflict => AppError::InvalidState {
            current: error.message.clone(),
            operation: operation.to_string(),
        },
        _ => AppError::ConfigError(format!(
            "import review read worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = ImportReviewWorkerClient::new(
            "https://import-review-worker.example.test",
            Some("import-review-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("import-review-worker.example.test"));
        assert!(!debug.contains("import-review-token"));
    }
}
