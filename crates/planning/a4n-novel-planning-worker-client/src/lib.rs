//! Typed HTTP adapter for the novel-planning worker.

mod shooting_script;
mod visual_plan;

use a4n_error::AppError;
use a4n_novel_planning_worker_contract::NOVEL_PLANNING_WORKER_ID;
use a4n_worker_client::{HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47651";

#[derive(Clone, Debug)]
pub struct NovelPlanningWorkerClient {
    transport: HttpWorkerClient,
}

impl NovelPlanningWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, NOVEL_PLANNING_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_NOVEL_PLANNING_WORKER_URL",
            "A4N_NOVEL_PLANNING_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        user_id: i64,
        project_id: i64,
        target_revision: Option<i32>,
        operation_id: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let principal = a4n_worker_client::user_principal_with_scope(
            user_id,
            Some(project_id),
            "user_id 和 project_id 必须大于 0",
        )?;
        if target_revision.is_some_and(|revision| revision <= 0) {
            return Err(AppError::InvalidArgument(
                "expected_revision 必须大于 0".into(),
            ));
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal,
            operation_id: operation_id.to_string(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            target_revision: target_revision.map(|revision| revision as u64),
            runtime_config_snapshot_ref: None,
            lease: WorkerLease::control_request(NOVEL_PLANNING_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation_id, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "novel-planning worker",
                    WorkerClientErrorPolicy::Configuration,
                )
            })?;
        match response.status {
            WorkerOperationStatus::Completed => response.payload.ok_or_else(|| {
                AppError::ConfigError(format!(
                    "novel-planning worker 完成响应缺少 payload: {operation_id}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation_id,
            )),
            status => Err(AppError::ConfigError(format!(
                "novel-planning worker 返回未完成状态: operation={operation_id}, status={status:?}"
            ))),
        }
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
        WorkerErrorCode::LeaseLost
        | WorkerErrorCode::Unavailable
        | WorkerErrorCode::Timeout
        | WorkerErrorCode::VersionMismatch
        | WorkerErrorCode::Internal
        | WorkerErrorCode::Cancelled => AppError::ConfigError(format!(
            "novel-planning worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_novel_planning_worker_contract::EDIT_SHOOTING_SCRIPT_OPERATION;

    #[test]
    fn client_debug_does_not_expose_worker_url_or_token() {
        let client = NovelPlanningWorkerClient::new(
            "https://user:secret@worker.example.test",
            Some("worker-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("worker.example.test"));
        assert!(!debug.contains("worker-token"));
    }

    #[test]
    fn failed_worker_errors_keep_revision_conflict_semantics() {
        let error = WorkerError {
            code: WorkerErrorCode::Conflict,
            message: "shooting_script_revision_conflict:id=4,expected=2".into(),
            retryable: false,
        };
        assert!(matches!(
            worker_response_error(&error, EDIT_SHOOTING_SCRIPT_OPERATION),
            AppError::InvalidState { .. }
        ));
    }
}
