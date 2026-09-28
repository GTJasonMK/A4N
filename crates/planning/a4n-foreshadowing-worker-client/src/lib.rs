//! Typed Host adapter for the foreshadowing records worker.

use a4n_api_contract::profile::{
    CreateForeshadowingInput, DeleteForeshadowingInput, ForeshadowingMutationApplicationPort,
    ForeshadowingRagSyncHostPort, ForeshadowingReadApplicationPort, ForeshadowingSummary,
    ForeshadowingWriteResult, UpdateForeshadowingInput,
};
use a4n_error::AppError;
use a4n_foreshadowing_worker_contract::{
    CreateInput, DeleteInput, FORESHADOWING_CREATE_OPERATION, FORESHADOWING_DELETE_OPERATION,
    FORESHADOWING_LIST_OPERATION, FORESHADOWING_UPDATE_OPERATION, FORESHADOWING_WORKER_ID,
    ForeshadowingMutationResult, ForeshadowingProjectInput, UpdateInput,
};
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47656";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

mod mutations;
mod reads;

#[derive(Clone)]
pub struct ForeshadowingWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for ForeshadowingWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ForeshadowingWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl ForeshadowingWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, FORESHADOWING_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_FORESHADOWING_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_FORESHADOWING_WORKER_TOKEN")
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
                worker_id: FORESHADOWING_WORKER_ID.into(),
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
                    "foreshadowing worker 完成响应缺少 payload: {operation_id}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation_id,
            )),
            status => Err(AppError::ConfigError(format!(
                "foreshadowing worker 返回未完成状态: operation={operation_id}, status={status:?}"
            ))),
        }
    }
}

fn sync_warning(
    result: Result<(), AppError>,
    project_id: i64,
    foreshadowing_id: i64,
    message: &str,
) -> ForeshadowingWriteResult {
    let warnings = match result {
        Ok(()) => Vec::new(),
        Err(error) => {
            tracing::warn!(
                project_id,
                foreshadowing_id,
                error = %error,
                "{message}"
            );
            vec![message.to_string()]
        }
    };
    ForeshadowingWriteResult { warnings }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    match error {
        WorkerClientError::Timeout => {
            AppError::Timeout("foreshadowing worker 请求超时".to_string())
        }
        WorkerClientError::Transport(error) => {
            AppError::Timeout(format!("foreshadowing worker 请求失败: {error}"))
        }
        WorkerClientError::HttpStatus { status } if status >= 500 => {
            AppError::Timeout(format!("foreshadowing worker 返回 HTTP {status}"))
        }
        error => AppError::ConfigError(format!("foreshadowing worker 请求失败: {error}")),
    }
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
        WorkerErrorCode::Unavailable | WorkerErrorCode::Timeout if error.retryable => {
            AppError::Timeout(format!(
                "foreshadowing worker {operation} 失败: {}",
                error.message
            ))
        }
        WorkerErrorCode::Cancelled => AppError::Cancelled,
        WorkerErrorCode::Internal => AppError::Internal(anyhow::anyhow!(
            "foreshadowing worker {operation} 失败: {}",
            error.message
        )),
        _ => AppError::ConfigError(format!(
            "foreshadowing worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = ForeshadowingWorkerClient::new(
            "https://foreshadowing-worker.example.test",
            Some("foreshadowing-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("foreshadowing-worker.example.test"));
        assert!(!debug.contains("foreshadowing-token"));
    }

    #[test]
    fn sync_warning_preserves_committed_result_when_host_sync_fails() {
        let result = sync_warning(
            Err(AppError::InvalidArgument("索引服务不可用".into())),
            1,
            2,
            "伏笔已保存，但检索索引更新失败，可稍后重试",
        );

        assert_eq!(
            result.warnings,
            vec!["伏笔已保存，但检索索引更新失败，可稍后重试"]
        );
    }

    #[test]
    fn retryable_worker_failures_remain_retryable() {
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
                FORESHADOWING_CREATE_OPERATION,
            ),
            AppError::Timeout(_)
        ));
        assert!(matches!(
            worker_response_error(
                &WorkerError {
                    code: WorkerErrorCode::Internal,
                    message: "query failed".into(),
                    retryable: false,
                },
                FORESHADOWING_CREATE_OPERATION,
            ),
            AppError::Internal(_)
        ));
    }
}
