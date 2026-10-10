//! Typed Host adapter for the object profile worker.

use a4n_api_contract::profile::{
    CreateObjectProfileInput, DeleteObjectProfileInput, ObjectProfileDeleteOutcome,
    ObjectProfileMutationApplicationPort, ObjectProfileMutationOutcome,
    ObjectProfileReadApplicationPort, ObjectProfileSummary, UpdateObjectProfileInput,
};
use a4n_error::AppError;
use a4n_object_profile_worker_contract::{
    OBJECT_PROFILE_CREATE_OPERATION, OBJECT_PROFILE_DELETE_OPERATION,
    OBJECT_PROFILE_LIST_OPERATION, OBJECT_PROFILE_UPDATE_OPERATION, OBJECT_PROFILE_WORKER_ID,
    ObjectProfileDeleteResponse, ObjectProfileMutationResponse, ObjectProfileProjectInput,
};
use a4n_worker_client::{
    HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error, map_worker_response_error,
};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerLease, WorkerOperationStatus,
    WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47657";

mod mutations;
mod reads;

#[derive(Clone, Debug)]
pub struct ObjectProfileWorkerClient {
    transport: HttpWorkerClient,
}

impl ObjectProfileWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, OBJECT_PROFILE_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_OBJECT_PROFILE_WORKER_URL",
            "A4N_OBJECT_PROFILE_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
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
        let principal = a4n_worker_client::user_principal_with_scope(
            user_id,
            project_id,
            "user_id 和 project_id 必须大于 0",
        )?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal,
            operation_id: operation_id.to_string(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease::control_request(OBJECT_PROFILE_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation_id, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "object profile worker",
                    WorkerClientErrorPolicy::Configuration,
                )
            })?;
        match response.status {
            WorkerOperationStatus::Completed => response.payload.ok_or_else(|| {
                AppError::ConfigError(format!(
                    "object profile worker 完成响应缺少 payload: {operation_id}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation_id,
            )),
            status => Err(AppError::ConfigError(format!(
                "object profile worker 返回未完成状态: operation={operation_id}, status={status:?}"
            ))),
        }
    }
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    map_worker_response_error(error, "object profile worker", operation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = ObjectProfileWorkerClient::new(
            "https://object-profile-worker.example.test",
            Some("object-profile-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("object-profile-worker.example.test"));
        assert!(!debug.contains("object-profile-token"));
    }
}
