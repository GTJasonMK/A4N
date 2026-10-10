//! Typed Host adapter for the import-review read worker.

use a4n_api_contract::import_review::{
    CheckImportDedupInput, DedupResult, ImportProjectReviewsInput, ImportReviewIdInput,
    ImportReviewMutationApplicationPort, ImportReviewReadApplicationPort, ImportReviewSummary,
};
use a4n_error::AppError;
use a4n_import_review_worker_contract::{
    IMPORT_REVIEW_DEDUP_OPERATION, IMPORT_REVIEW_DELETE_OPERATION, IMPORT_REVIEW_LIST_OPERATION,
    IMPORT_REVIEW_READ_WORKER_ID, IMPORT_REVIEW_REJECT_OPERATION,
};
use a4n_worker_client::{
    HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error, map_worker_response_error,
};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerLease, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47665";

mod mutations;
mod reads;

#[derive(Clone, Debug)]
pub struct ImportReviewWorkerClient {
    transport: HttpWorkerClient,
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
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_IMPORT_REVIEW_READ_WORKER_URL",
            "A4N_IMPORT_REVIEW_READ_WORKER_TOKEN",
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
        let principal = a4n_worker_client::user_principal(user_id)?;
        let project_id = a4n_worker_client::optional_positive_id("project_id", project_id)?;
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
            lease: WorkerLease::control_request(IMPORT_REVIEW_READ_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation_id, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "import review read worker",
                    WorkerClientErrorPolicy::Configuration,
                )
            })?;
        a4n_worker_client::worker_response_payload(
            response,
            "import review read worker",
            Some(operation_id),
            |error| worker_response_error(error, operation_id),
        )
    }
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    map_worker_response_error(error, "import review read worker", operation)
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
