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
    CANVAS_GRAPH_SAVE_OPERATION, CANVAS_RECORD_WORKER_ID, CANVAS_TEXT_ASSET_LIST_OPERATION,
    CANVAS_TEXT_ASSET_SAVE_OPERATION, CANVAS_TEXT_ASSET_VERSIONS_OPERATION,
};
use a4n_error::AppError;
#[cfg(test)]
use a4n_worker_client::WorkerClientError;
use a4n_worker_client::{HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47677";

mod mutations;
mod reads;

#[derive(Clone, Debug)]
pub struct CanvasRecordWorkerClient {
    transport: HttpWorkerClient,
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
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_CANVAS_RECORD_WORKER_URL",
            "A4N_CANVAS_RECORD_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
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
            lease: WorkerLease::control_request(CANVAS_RECORD_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation_id, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "Canvas record worker",
                    WorkerClientErrorPolicy::TransientAsTimeout,
                )
            })?;
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
            map_worker_client_error(
                WorkerClientError::Timeout,
                "Canvas record worker",
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
