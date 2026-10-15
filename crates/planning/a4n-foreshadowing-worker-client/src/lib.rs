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
#[cfg(test)]
use a4n_worker_client::WorkerClientError;
use a4n_worker_client::{HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47656";

mod mutations;
mod reads;

#[derive(Clone, Debug)]
pub struct ForeshadowingWorkerClient {
    transport: HttpWorkerClient,
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
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_FORESHADOWING_WORKER_URL",
            "A4N_FORESHADOWING_WORKER_TOKEN",
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
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal: a4n_worker_client::user_principal_with_scope(
                user_id,
                Some(project_id),
                "user_id 和 project_id 必须大于 0",
            )?,
            operation_id: operation_id.to_string(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease::control_request(FORESHADOWING_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation_id, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "foreshadowing worker",
                    WorkerClientErrorPolicy::TransientAsTimeout,
                )
            })?;
        a4n_worker_client::worker_response_payload(
            response,
            "foreshadowing worker",
            Some(operation_id),
            |error| worker_response_error(error, operation_id),
        )
    }
}

fn sync_warning(
    record: Option<ForeshadowingSummary>,
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
    ForeshadowingWriteResult { record, warnings }
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
            None,
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
            map_worker_client_error(
                WorkerClientError::Timeout,
                "foreshadowing worker",
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
