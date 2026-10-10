//! Typed Host adapter for the Manga record read worker.

use a4n_api_contract::manga::{
    MangaPagePromptSummary, MangaPipelineReadApplicationPort, MangaPipelineStatusSummary,
    MangaRecordReadApplicationPort,
};
use a4n_error::AppError;
use a4n_manga_record_worker_contract::{
    MANGA_PIPELINE_STATUS_OPERATION, MANGA_PIPELINE_STATUSES_OPERATION,
    MANGA_RECORD_LIST_OPERATION, MANGA_RECORD_WORKER_ID, MangaChapterInput,
    MangaPipelineStatusesInput,
};
use a4n_worker_client::{
    HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error, map_worker_response_error,
};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerLease, WorkerResponse,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47676";

mod pipeline;
mod reads;

#[derive(Clone, Debug)]
pub struct MangaRecordWorkerClient {
    transport: HttpWorkerClient,
}

impl MangaRecordWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, MANGA_RECORD_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_MANGA_RECORD_WORKER_URL",
            "A4N_MANGA_RECORD_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        user_id: i64,
        project_id: i64,
        operation: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: Serialize,
        O: DeserializeOwned,
    {
        let principal = a4n_worker_client::user_principal(user_id)?;
        a4n_worker_client::positive_id("project_id", project_id)?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal,
            operation_id: operation.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease::control_request(MANGA_RECORD_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "manga record worker",
                    WorkerClientErrorPolicy::Configuration,
                )
            })?;
        a4n_worker_client::worker_response_payload(
            response,
            "manga record worker",
            Some(operation),
            |error| worker_response_error(error, operation),
        )
    }
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    map_worker_response_error(error, "manga record worker", operation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = MangaRecordWorkerClient::new(
            "https://manga-record-worker.example.test",
            Some("manga-record-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("manga-record-worker.example.test"));
        assert!(!debug.contains("manga-record-token"));
    }
}
