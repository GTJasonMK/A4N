//! PostgreSQL-backed provider configuration read worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_api_contract::config::{
    EmbeddingConfigSummary, ImageConfigForConnectivityTest, ImageConfigSummary, LlmConfigSummary,
    VideoConfigSummary,
};
use a4n_config::ensure_default_config_delete_allowed;
use a4n_config_read_core::{
    embedding_provider_config, embedding_summary, image_connectivity_config, image_summary,
    llm_provider_config, llm_summary, video_summary,
};
use a4n_config_read_worker_contract::{
    CONFIG_MUTATION_CAPABILITY, CONFIG_READ_CAPABILITY,
    CONFIG_READ_EMBEDDING_CONNECTIVITY_OPERATION, CONFIG_READ_EMBEDDING_LIST_OPERATION,
    CONFIG_READ_IMAGE_CONNECTIVITY_OPERATION, CONFIG_READ_IMAGE_EXISTS_OPERATION,
    CONFIG_READ_IMAGE_LIST_OPERATION, CONFIG_READ_LLM_CONNECTIVITY_OPERATION,
    CONFIG_READ_LLM_EXISTS_OPERATION, CONFIG_READ_LLM_LIST_OPERATION,
    CONFIG_READ_SAVED_API_KEY_OPERATION, CONFIG_READ_VIDEO_EXISTS_OPERATION,
    CONFIG_READ_VIDEO_LIST_OPERATION, CONFIG_READ_WORKER_CONTRACT_VERSION, CONFIG_READ_WORKER_ID,
    CONFIG_SETTINGS_CAPABILITY, ConfigIdInput, ConfigProviderKindWire, CreateEmbeddingCommand,
    CreateImageCommand, CreateLlmCommand, CreateVideoCommand, EmptyInput, SavedApiKeyInput,
    SettingsExportInput, SettingsImportCommand, SettingsPrepareImportResult,
    UpdateEmbeddingCommand, UpdateImageCommand, UpdateLlmCommand, UpdateVideoCommand,
};
use a4n_db::DbHandle;
use a4n_db::config::ProviderConfigKind;
use a4n_db_models::models::user::role::{UserRole, parse_user_role};
use a4n_provider_contract::{EmbeddingProviderConfig, LlmProviderConfig};
use a4n_worker_client::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerHealth,
    WorkerHealthStatus, WorkerOperationRef, WorkerOperationStatus, WorkerPrincipalKind,
    WorkerResponse,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Statement};
use serde_json::Value;
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_CONFIG_READ_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_CONFIG_READ_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47673";

#[derive(Clone)]
pub struct ConfigReadWorker {
    state: WorkerState,
}

#[derive(Clone)]
struct WorkerState {
    database: DbHandle,
    worker_id: Arc<str>,
    auth_token: Arc<str>,
}

#[derive(Debug)]
struct WorkerFailure {
    code: WorkerErrorCode,
    message: String,
    retryable: bool,
}

impl ConfigReadWorker {
    pub fn new(
        database: DatabaseConnection,
        worker_id: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Self {
        Self {
            state: WorkerState {
                database: DbHandle::new(Arc::new(database)),
                worker_id: Arc::<str>::from(worker_id.into()),
                auth_token: Arc::<str>::from(auth_token.into()),
            },
        }
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route(WORKER_HEALTH_PATH, get(health))
            .route(WORKER_CAPABILITIES_PATH, get(capabilities))
            .route("/v1/commands/config.read.llm_exists", post(llm_exists))
            .route("/v1/commands/config.read.image_exists", post(image_exists))
            .route("/v1/commands/config.read.video_exists", post(video_exists))
            .route(
                "/v1/commands/config.read.saved_api_key",
                post(saved_api_key),
            )
            .route(
                "/v1/commands/config.read.llm_connectivity",
                post(llm_connectivity),
            )
            .route(
                "/v1/commands/config.read.embedding_connectivity",
                post(embedding_connectivity),
            )
            .route(
                "/v1/commands/config.read.image_connectivity",
                post(image_connectivity),
            )
            .route("/v1/commands/config.read.llm_list", post(llm_list))
            .route("/v1/commands/config.read.image_list", post(image_list))
            .route("/v1/commands/config.read.video_list", post(video_list))
            .route(
                "/v1/commands/config.read.embedding_list",
                post(embedding_list),
            )
            .route(
                "/v1/commands/config.mutation.baseline_embedding",
                post(baseline_embedding),
            )
            .route(
                "/v1/commands/config.mutation.baseline_llm",
                post(baseline_llm),
            )
            .route(
                "/v1/commands/config.mutation.baseline_image",
                post(baseline_image),
            )
            .route(
                "/v1/commands/config.mutation.baseline_video",
                post(baseline_video),
            )
            .route("/v1/commands/config.mutation.create_llm", post(create_llm))
            .route("/v1/commands/config.mutation.update_llm", post(update_llm))
            .route(
                "/v1/commands/config.mutation.set_default_llm",
                post(set_default_llm),
            )
            .route("/v1/commands/config.mutation.delete_llm", post(delete_llm))
            .route(
                "/v1/commands/config.mutation.create_image",
                post(create_image),
            )
            .route(
                "/v1/commands/config.mutation.update_image",
                post(update_image),
            )
            .route(
                "/v1/commands/config.mutation.set_default_image",
                post(set_default_image),
            )
            .route(
                "/v1/commands/config.mutation.delete_image",
                post(delete_image),
            )
            .route(
                "/v1/commands/config.mutation.create_video",
                post(create_video),
            )
            .route(
                "/v1/commands/config.mutation.update_video",
                post(update_video),
            )
            .route(
                "/v1/commands/config.mutation.set_default_video",
                post(set_default_video),
            )
            .route(
                "/v1/commands/config.mutation.delete_video",
                post(delete_video),
            )
            .route(
                "/v1/commands/config.mutation.create_embedding",
                post(create_embedding),
            )
            .route(
                "/v1/commands/config.mutation.update_embedding",
                post(update_embedding),
            )
            .route(
                "/v1/commands/config.mutation.set_default_embedding",
                post(set_default_embedding),
            )
            .route(
                "/v1/commands/config.mutation.delete_embedding",
                post(delete_embedding),
            )
            .route("/v1/commands/config.settings.export", post(settings_export))
            .route(
                "/v1/commands/config.settings.prepare_import",
                post(settings_prepare_import),
            )
            .route("/v1/commands/config.settings.import", post(settings_import))
            .with_state(self.state.clone())
    }

    pub async fn serve(self, address: SocketAddr) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(address).await?;
        axum::serve(listener, self.router())
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(anyhow::Error::from)
    }
}

include!("worker/runtime.rs");
include!("worker/routes.rs");
include!("worker/reads.rs");
include!("worker/llm.rs");
include!("worker/mutations.rs");
include!("worker/settings.rs");
include!("worker/helpers.rs");
