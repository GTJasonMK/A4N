//! Typed Host adapter for provider configuration reads.

use a4n_api_contract::config::{
    ConfigMutationApplicationPort, ConfigProviderKind, ConfigReadApplicationPort,
    CreateEmbeddingInput, CreateLlmInput, EmbeddingConfigSummary, ImageConfigForConnectivityTest,
    ImageConfigInput, ImageConfigSummary, ImportAllConfigsInput, LlmConfigSummary,
    SettingsAllHostPort, SettingsExportApplicationPort, SettingsImportApplicationPort,
    UpdateEmbeddingInput, UpdateImageConfigInput, UpdateLlmInput, UpdateVideoConfigInput,
    VideoConfigInput, VideoConfigSummary,
};
use a4n_config_read_worker_contract::{
    CONFIG_MUTATION_BASELINE_EMBEDDING_OPERATION, CONFIG_MUTATION_BASELINE_IMAGE_OPERATION,
    CONFIG_MUTATION_BASELINE_LLM_OPERATION, CONFIG_MUTATION_BASELINE_VIDEO_OPERATION,
    CONFIG_MUTATION_CREATE_EMBEDDING_OPERATION, CONFIG_MUTATION_CREATE_IMAGE_OPERATION,
    CONFIG_MUTATION_CREATE_LLM_OPERATION, CONFIG_MUTATION_CREATE_VIDEO_OPERATION,
    CONFIG_MUTATION_DELETE_EMBEDDING_OPERATION, CONFIG_MUTATION_DELETE_IMAGE_OPERATION,
    CONFIG_MUTATION_DELETE_LLM_OPERATION, CONFIG_MUTATION_DELETE_VIDEO_OPERATION,
    CONFIG_MUTATION_SET_DEFAULT_EMBEDDING_OPERATION, CONFIG_MUTATION_SET_DEFAULT_IMAGE_OPERATION,
    CONFIG_MUTATION_SET_DEFAULT_LLM_OPERATION, CONFIG_MUTATION_SET_DEFAULT_VIDEO_OPERATION,
    CONFIG_MUTATION_UPDATE_EMBEDDING_OPERATION, CONFIG_MUTATION_UPDATE_IMAGE_OPERATION,
    CONFIG_MUTATION_UPDATE_LLM_OPERATION, CONFIG_MUTATION_UPDATE_VIDEO_OPERATION,
    CONFIG_READ_EMBEDDING_CONNECTIVITY_OPERATION, CONFIG_READ_EMBEDDING_LIST_OPERATION,
    CONFIG_READ_IMAGE_CONNECTIVITY_OPERATION, CONFIG_READ_IMAGE_EXISTS_OPERATION,
    CONFIG_READ_IMAGE_LIST_OPERATION, CONFIG_READ_LLM_CONNECTIVITY_OPERATION,
    CONFIG_READ_LLM_EXISTS_OPERATION, CONFIG_READ_LLM_LIST_OPERATION,
    CONFIG_READ_SAVED_API_KEY_OPERATION, CONFIG_READ_VIDEO_EXISTS_OPERATION,
    CONFIG_READ_VIDEO_LIST_OPERATION, CONFIG_READ_WORKER_ID, CONFIG_SETTINGS_EXPORT_OPERATION,
    CONFIG_SETTINGS_IMPORT_OPERATION, CONFIG_SETTINGS_PREPARE_IMPORT_OPERATION, ConfigIdInput,
    ConfigProviderKindWire, CreateEmbeddingCommand, CreateImageCommand, CreateLlmCommand,
    CreateVideoCommand, EmptyInput, SavedApiKeyInput, SettingsExportInput, SettingsImportCommand,
    SettingsPrepareImportResult, UpdateEmbeddingCommand, UpdateImageCommand, UpdateLlmCommand,
    UpdateVideoCommand,
};
use a4n_error::AppError;
use a4n_provider_contract::{EmbeddingProviderConfig, LlmProviderConfig};
use a4n_worker_client::{
    HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error, map_worker_response_error,
};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerLease, WorkerPrincipal,
    WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47673";

#[derive(Clone, Debug)]
pub struct ConfigReadWorkerClient {
    transport: HttpWorkerClient,
}

impl ConfigReadWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, CONFIG_READ_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_CONFIG_READ_WORKER_URL",
            "A4N_CONFIG_READ_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        principal: WorkerPrincipal,
        operation: &str,
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
            operation_id: operation.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease::control_request(CONFIG_READ_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "config read worker",
                    WorkerClientErrorPolicy::Configuration,
                )
            })?;
        a4n_worker_client::worker_response_payload(
            response,
            "config read worker",
            Some(operation),
            |error| worker_response_error(error, operation),
        )
    }

    fn service() -> WorkerPrincipal {
        WorkerPrincipal {
            kind: WorkerPrincipalKind::Service,
            subject: "runtime".into(),
        }
    }

    fn kind(kind: ConfigProviderKind) -> ConfigProviderKindWire {
        match kind {
            ConfigProviderKind::Llm => ConfigProviderKindWire::Llm,
            ConfigProviderKind::Image => ConfigProviderKindWire::Image,
            ConfigProviderKind::Video => ConfigProviderKindWire::Video,
        }
    }
}

include!("reads.rs");
include!("mutations.rs");
include!("settings.rs");

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    map_worker_response_error(error, "config read worker", operation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = ConfigReadWorkerClient::new(
            "https://config-read-worker.example.test",
            Some("config-read-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("config-read-worker.example.test"));
        assert!(!debug.contains("config-read-token"));
    }
}
