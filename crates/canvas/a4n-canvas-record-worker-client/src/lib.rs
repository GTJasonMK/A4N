//! Typed Host adapter for the Canvas graph-record worker.

use a4n_api_contract::canvas::{
    CanvasGraphDocument, CanvasGraphEditResult, CanvasGraphReadResult, CanvasGraphSaveResult,
    CanvasMutationApplicationPort, CanvasReadApplicationPort, CanvasTextAssetSummary,
    CanvasTextAssetVersionSummary, EditCanvasGraphInput, GetCanvasGraphInput,
    ListCanvasTextAssetVersionsInput, ListCanvasTextAssetsInput, ReadCanvasGraphInput,
    SaveCanvasGraphInput, SaveCanvasTextAssetInput,
};
use a4n_canvas_record_worker_contract::{
    CANVAS_GRAPH_EDIT_OPERATION, CANVAS_GRAPH_GET_OPERATION, CANVAS_GRAPH_READ_OPERATION,
    CANVAS_GRAPH_SAVE_OPERATION, CANVAS_RECORD_WORKER_ID, CANVAS_TEXT_ASSET_SAVE_OPERATION,
    CANVAS_TEXT_ASSET_LIST_OPERATION, CANVAS_TEXT_ASSET_VERSIONS_OPERATION,
};
use a4n_error::AppError;
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47677";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

mod mutations;
mod reads;

#[derive(Clone)]
pub struct CanvasRecordWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for CanvasRecordWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CanvasRecordWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl CanvasRecordWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, CANVAS_RECORD_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_CANVAS_RECORD_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_CANVAS_RECORD_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        user_id: i64,
        project_id: i64,
        operation_id: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        if user_id <= 0 || project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "user_id 和 project_id 必须大于 0".into(),
            ));
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
            project_id: Some(project_id),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: CANVAS_RECORD_WORKER_ID.into(),
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
                    "Canvas record worker 完成响应缺少 payload: {operation_id}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation_id,
            )),
            status => Err(AppError::ConfigError(format!(
                "Canvas record worker 返回未完成状态: operation={operation_id}, status={status:?}"
            ))),
        }
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    match error {
        WorkerClientError::Timeout => {
            AppError::Timeout("Canvas record worker 请求超时".to_string())
        }
        WorkerClientError::Transport(error) => {
            AppError::Timeout(format!("Canvas record worker 请求失败: {error}"))
        }
        WorkerClientError::HttpStatus { status } if status >= 500 => {
            AppError::Timeout(format!("Canvas record worker 返回 HTTP {status}"))
        }
        error => AppError::ConfigError(format!("Canvas record worker 请求失败: {error}")),
    }
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    match error.code {
        WorkerErrorCode::InvalidRequest => AppError::InvalidArgument(error.message.clone()),
        WorkerErrorCode::Unauthorized => AppError::Forbidden(error.message.clone()),
        WorkerErrorCode::NotFound => AppError::NotFound(error.message.clone()),
        WorkerErrorCode::Conflict | WorkerErrorCode::VersionMismatch => AppError::InvalidState {
            current: error.message.clone(),
            operation: if operation == CANVAS_GRAPH_SAVE_OPERATION {
                "保存画布"
            } else {
                "读取或修改画布"
            }
            .into(),
        },
        WorkerErrorCode::Unavailable | WorkerErrorCode::Timeout if error.retryable => {
            AppError::Timeout(format!(
                "Canvas record worker {operation} 失败: {}",
                error.message
            ))
        }
        WorkerErrorCode::Cancelled => AppError::Cancelled,
        WorkerErrorCode::Internal => AppError::Internal(anyhow::anyhow!(
            "Canvas record worker {operation} 失败: {}",
            error.message
        )),
        _ => AppError::ConfigError(format!(
            "Canvas record worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = CanvasRecordWorkerClient::new(
            "https://canvas-record-worker.example.test",
            Some("canvas-record-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("canvas-record-worker.example.test"));
        assert!(!debug.contains("canvas-record-token"));
    }

    #[test]
    fn edit_conflicts_keep_the_existing_operation_description() {
        let error = WorkerError {
            code: WorkerErrorCode::Conflict,
            message: "version mismatch".into(),
            retryable: false,
        };

        assert!(matches!(
            worker_response_error(&error, CANVAS_GRAPH_EDIT_OPERATION),
            AppError::InvalidState { operation, .. } if operation == "读取或修改画布"
        ));
    }

    #[test]
    fn retryable_and_internal_worker_failures_keep_their_error_semantics() {
        assert!(matches!(
            worker_client_error(WorkerClientError::Timeout),
            AppError::Timeout(_)
        ));
        assert!(matches!(
            worker_response_error(
                &WorkerError {
                    code: WorkerErrorCode::Unavailable,
                    message: "database unavailable".into(),
                    retryable: true,
                },
                CANVAS_GRAPH_SAVE_OPERATION,
            ),
            AppError::Timeout(_)
        ));
        assert!(matches!(
            worker_response_error(
                &WorkerError {
                    code: WorkerErrorCode::Internal,
                    message: "projection failed".into(),
                    retryable: false,
                },
                CANVAS_GRAPH_SAVE_OPERATION,
            ),
            AppError::Internal(_)
        ));
    }
}
