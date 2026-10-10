//! Typed Host adapter for the source snapshot read worker.

use a4n_error::AppError;
use a4n_source_snapshot_worker_contract::{
    SOURCE_SNAPSHOT_CHAPTERS_OPERATION, SOURCE_SNAPSHOT_WORKER_ID,
};
use a4n_worker_client::{HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47660";

mod reads;

#[derive(Clone, Debug)]
pub struct SourceSnapshotWorkerClient {
    transport: HttpWorkerClient,
}

impl SourceSnapshotWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, SOURCE_SNAPSHOT_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_SOURCE_SNAPSHOT_WORKER_URL",
            "A4N_SOURCE_SNAPSHOT_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
        Self::new(url, token)
    }

    async fn command<I, O>(&self, user_id: i64, project_id: i64, payload: I) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let principal = a4n_worker_client::user_principal_with_scope(
            user_id,
            Some(project_id),
            "请求范围 ID 必须大于 0",
        )?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal,
            operation_id: SOURCE_SNAPSHOT_CHAPTERS_OPERATION.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease::control_request(SOURCE_SNAPSHOT_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(SOURCE_SNAPSHOT_CHAPTERS_OPERATION, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "source snapshot worker",
                    WorkerClientErrorPolicy::Configuration,
                )
            })?;
        a4n_worker_client::worker_response_payload(
            response,
            "source snapshot worker",
            None,
            worker_response_error,
        )
    }
}

fn worker_response_error(error: &WorkerError) -> AppError {
    match error.code {
        WorkerErrorCode::InvalidRequest => AppError::InvalidArgument(error.message.clone()),
        WorkerErrorCode::Unauthorized => AppError::Forbidden(error.message.clone()),
        WorkerErrorCode::NotFound => AppError::NotFound(error.message.clone()),
        WorkerErrorCode::Conflict => AppError::InvalidState {
            current: error.message.clone(),
            operation: SOURCE_SNAPSHOT_CHAPTERS_OPERATION.into(),
        },
        _ => AppError::ConfigError(format!("source snapshot worker 失败: {}", error.message)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = SourceSnapshotWorkerClient::new(
            "https://source-snapshot-worker.example.test",
            Some("source-snapshot-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("source-snapshot-worker.example.test"));
        assert!(!debug.contains("source-snapshot-token"));
    }
}
