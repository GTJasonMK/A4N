//! Typed Host adapter for the theme read worker.

use a4n_api_contract::theme::{
    CreateThemeInput, PersistThemeResourceInput, ThemeMutationApplicationPort,
    ThemePreferenceSummary, ThemeReadApplicationPort, ThemeResourceRecord, ThemeSummary,
    UpdateThemeInput, UpdateThemePreferenceInput,
};
use a4n_error::AppError;
use a4n_theme_read_worker_contract::{
    ResourceIdInput, THEME_ACTIVATE_OPERATION, THEME_CREATE_OPERATION, THEME_DELETE_OPERATION,
    THEME_EXPORT_OPERATION, THEME_IMPORT_OPERATION, THEME_LIST_OPERATION,
    THEME_PREFERENCE_OPERATION, THEME_PREFERENCE_UPDATE_OPERATION, THEME_READ_WORKER_ID,
    THEME_RESOURCE_DELETE_OPERATION, THEME_RESOURCE_UPSERT_OPERATION, THEME_RESOURCES_OPERATION,
    THEME_UPDATE_OPERATION, ThemeIdInput, ThemeUpdateInput, ThemeUserInput,
};
use a4n_worker_client::{
    HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error, map_worker_response_error,
};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerLease, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47663";

mod mutations;
mod reads;

#[derive(Clone, Debug)]
pub struct ThemeReadWorkerClient {
    transport: HttpWorkerClient,
}

impl ThemeReadWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, THEME_READ_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_THEME_READ_WORKER_URL",
            "A4N_THEME_READ_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
        Self::new(url, token)
    }

    async fn command<O>(
        &self,
        user_id: i64,
        operation_id: &str,
        payload: impl serde::Serialize,
    ) -> Result<O, AppError>
    where
        O: serde::de::DeserializeOwned,
    {
        let principal = a4n_worker_client::user_principal(user_id)?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal,
            operation_id: operation_id.to_string(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease::control_request(THEME_READ_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation_id, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "theme read worker",
                    WorkerClientErrorPolicy::Configuration,
                )
            })?;
        a4n_worker_client::worker_response_payload(
            response,
            "theme read worker",
            Some(operation_id),
            |error| worker_response_error(error, operation_id),
        )
    }
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    map_worker_response_error(error, "theme read worker", operation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = ThemeReadWorkerClient::new(
            "https://theme-read-worker.example.test",
            Some("theme-read-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("theme-read-worker.example.test"));
        assert!(!debug.contains("theme-read-token"));
    }
}
