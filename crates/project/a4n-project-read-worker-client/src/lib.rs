//! Typed Host adapter for the project read worker.

use a4n_error::AppError;
use a4n_project_read_worker_contract::PROJECT_READ_WORKER_ID;
use a4n_worker_client::{
    HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error, map_worker_response_error,
};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerLease, WorkerPrincipal,
    WorkerPrincipalKind, WorkerResponse,
};

mod reads;

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47664";

#[derive(Clone, Debug)]
pub struct ProjectReadWorkerClient {
    transport: HttpWorkerClient,
}

impl ProjectReadWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, PROJECT_READ_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_PROJECT_READ_WORKER_URL",
            "A4N_PROJECT_READ_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        principal: WorkerPrincipal,
        operation_id: &str,
        project_id: Option<i64>,
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
            principal,
            operation_id: operation_id.to_string(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease::control_request(PROJECT_READ_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation_id, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "project read worker",
                    WorkerClientErrorPolicy::Configuration,
                )
            })?;
        a4n_worker_client::worker_response_payload(
            response,
            "project read worker",
            Some(operation_id),
            |error| worker_response_error(error, operation_id),
        )
    }

    fn service_principal() -> WorkerPrincipal {
        WorkerPrincipal {
            kind: WorkerPrincipalKind::Service,
            subject: "runtime".into(),
        }
    }
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    map_worker_response_error(error, "project read worker", operation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = ProjectReadWorkerClient::new(
            "https://project-read-worker.example.test",
            Some("project-read-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("project-read-worker.example.test"));
        assert!(!debug.contains("project-read-token"));
    }
}
