//! Typed Host adapter for the project-mutation worker.

use a4n_error::AppError;
use a4n_project_mutation_worker_contract::PROJECT_MUTATION_WORKER_ID;
use a4n_worker_client::{
    HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error, map_worker_response_error,
};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerLease, WorkerOperationStatus,
    WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

mod mutations;

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47669";

#[derive(Clone, Debug)]
pub struct ProjectMutationWorkerClient {
    transport: HttpWorkerClient,
}

impl ProjectMutationWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, PROJECT_MUTATION_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_PROJECT_MUTATION_WORKER_URL",
            "A4N_PROJECT_MUTATION_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        operation: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let principal = a4n_worker_client::user_principal(user_id)?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal,
            operation_id: operation.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease::control_request(PROJECT_MUTATION_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "project mutation worker",
                    WorkerClientErrorPolicy::Configuration,
                )
            })?;
        match response.status {
            WorkerOperationStatus::Completed => response.payload.ok_or_else(|| {
                AppError::ConfigError(format!(
                    "project mutation worker 完成响应缺少 payload: {operation}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation,
            )),
            status => Err(AppError::ConfigError(format!(
                "project mutation worker 返回未完成状态: operation={operation}, status={status:?}"
            ))),
        }
    }

    fn positive_id(field: &str, value: i64) -> Result<i64, AppError> {
        if value <= 0 {
            return Err(AppError::InvalidArgument(format!("{field} 必须大于 0")));
        }
        Ok(value)
    }
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    map_worker_response_error(error, "project mutation worker", operation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = ProjectMutationWorkerClient::new(
            "https://project-mutation-worker.example.test",
            Some("project-mutation-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("project-mutation-worker.example.test"));
        assert!(!debug.contains("project-mutation-token"));
    }
}
