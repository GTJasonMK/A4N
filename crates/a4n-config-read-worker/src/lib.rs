//! PostgreSQL-backed provider configuration read worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_api_contract::config::{
    CreateEmbeddingInput, CreateLlmInput, EmbeddingConfigSummary, ImageConfigForConnectivityTest,
    ImageConfigInput, ImageConfigSummary, LlmConfigSummary, UpdateLlmInput, VideoConfigInput,
    VideoConfigSummary,
};
use a4n_config::{
    ensure_default_config_delete_allowed, image_sizes_to_json, validate_embedding_config_params,
    validate_image_config_params,
};
use a4n_config_read_core::{
    embedding_provider_config, embedding_summary, image_connectivity_config, image_summary,
    llm_provider_config, llm_summary, video_summary,
};
use a4n_config_read_worker_contract::{
    CONFIG_MUTATION_BASELINE_EMBEDDING_OPERATION, CONFIG_MUTATION_BASELINE_IMAGE_OPERATION,
    CONFIG_MUTATION_BASELINE_LLM_OPERATION, CONFIG_MUTATION_BASELINE_VIDEO_OPERATION,
    CONFIG_MUTATION_CAPABILITY, CONFIG_MUTATION_CREATE_EMBEDDING_OPERATION,
    CONFIG_MUTATION_CREATE_IMAGE_OPERATION, CONFIG_MUTATION_CREATE_LLM_OPERATION,
    CONFIG_MUTATION_CREATE_VIDEO_OPERATION, CONFIG_MUTATION_DELETE_EMBEDDING_OPERATION,
    CONFIG_MUTATION_DELETE_IMAGE_OPERATION, CONFIG_MUTATION_DELETE_LLM_OPERATION,
    CONFIG_MUTATION_DELETE_VIDEO_OPERATION, CONFIG_MUTATION_SET_DEFAULT_EMBEDDING_OPERATION,
    CONFIG_MUTATION_SET_DEFAULT_IMAGE_OPERATION, CONFIG_MUTATION_SET_DEFAULT_LLM_OPERATION,
    CONFIG_MUTATION_SET_DEFAULT_VIDEO_OPERATION, CONFIG_MUTATION_UPDATE_EMBEDDING_OPERATION,
    CONFIG_MUTATION_UPDATE_IMAGE_OPERATION, CONFIG_MUTATION_UPDATE_LLM_OPERATION,
    CONFIG_MUTATION_UPDATE_VIDEO_OPERATION, CONFIG_READ_CAPABILITY,
    CONFIG_READ_EMBEDDING_CONNECTIVITY_OPERATION, CONFIG_READ_EMBEDDING_LIST_OPERATION,
    CONFIG_READ_IMAGE_CONNECTIVITY_OPERATION, CONFIG_READ_IMAGE_EXISTS_OPERATION,
    CONFIG_READ_IMAGE_LIST_OPERATION, CONFIG_READ_LLM_CONNECTIVITY_OPERATION,
    CONFIG_READ_LLM_EXISTS_OPERATION, CONFIG_READ_LLM_LIST_OPERATION,
    CONFIG_READ_SAVED_API_KEY_OPERATION, CONFIG_READ_VIDEO_EXISTS_OPERATION,
    CONFIG_READ_VIDEO_LIST_OPERATION, CONFIG_READ_WORKER_CONTRACT_VERSION, CONFIG_READ_WORKER_ID,
    ConfigIdInput, ConfigProviderKindWire, CreateEmbeddingCommand, CreateImageCommand,
    CreateLlmCommand, CreateVideoCommand, EmptyInput, SavedApiKeyInput, UpdateEmbeddingCommand,
    UpdateImageCommand, UpdateLlmCommand, UpdateVideoCommand,
};
use a4n_db::DbHandle;
use a4n_db::config::{
    EmbeddingConfigRecordInput, ImageConfigRecordInput, LlmConfigRecordInput, ProviderConfigKind,
    VideoConfigRecordInput,
};
use a4n_db_models::models::user::role::{UserRole, parse_user_role};
use a4n_domain::embedding::CreateEmbeddingConfigParams;
use a4n_llm_config::{
    CreateLlmConfigInput, LlmConfigSnapshot, NormalizedLlmConfig, UpdateLlmConfigInput,
    normalize_create_llm_config_input, normalize_update_llm_config_input, validate_llm_config,
};
use a4n_provider_contract::{EmbeddingProviderConfig, LlmProviderConfig};
use a4n_video::{VideoConfigParams, validate_video_config_params};
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

pub fn run() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    init_tracing();
    let database_url = a4n_config::required_non_empty_env_string(DATABASE_URL_ENV)?;
    a4n_config::validate_postgres_database_url(DATABASE_URL_ENV, &database_url)?;
    let auth_token = a4n_config::required_non_empty_env_string(WORKER_TOKEN_ENV)?;
    let address = worker_addr()?;
    let mut options = ConnectOptions::new(database_url);
    options
        .max_connections(8)
        .min_connections(1)
        .connect_timeout(std::time::Duration::from_secs(5));
    let database = Database::connect(options).await?;
    ConfigReadWorker::new(database, CONFIG_READ_WORKER_ID, auth_token)
        .serve(address)
        .await
}

fn worker_addr() -> anyhow::Result<SocketAddr> {
    let value = env::var(WORKER_ADDR_ENV).unwrap_or_else(|_| DEFAULT_ADDR.into());
    let address: SocketAddr = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{WORKER_ADDR_ENV} 无效: {error}"))?;
    if !address.ip().is_loopback() || address.port() == 0 {
        anyhow::bail!("{WORKER_ADDR_ENV} 必须是 loopback 且端口大于 0")
    }
    Ok(address)
}

async fn health(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let status = match state
        .database
        .connection()
        .execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT 1",
        ))
        .await
    {
        Ok(_) => WorkerHealthStatus::Healthy,
        Err(_) => WorkerHealthStatus::Degraded,
    };
    Json(WorkerHealth {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: state.worker_id.to_string(),
        status,
        version: env!("CARGO_PKG_VERSION").into(),
    })
    .into_response()
}

async fn capabilities(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    Json(WorkerCapabilities {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: state.worker_id.to_string(),
        capabilities: vec![
            WorkerCapability {
                name: CONFIG_READ_CAPABILITY.into(),
                version: CONFIG_READ_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: CONFIG_MUTATION_CAPABILITY.into(),
                version: CONFIG_READ_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

macro_rules! command_handler {
    ($name:ident, $method:ident, $input:ty) => {
        async fn $name(
            State(state): State<WorkerState>,
            headers: HeaderMap,
            Json(request): Json<WorkerEnvelope<$input>>,
        ) -> Response {
            command_response(&state, &headers, request, |request| state.$method(request)).await
        }
    };
}

command_handler!(llm_exists, llm_exists, EmptyInput);
command_handler!(image_exists, image_exists, EmptyInput);
command_handler!(video_exists, video_exists, EmptyInput);
command_handler!(saved_api_key, saved_api_key, SavedApiKeyInput);
command_handler!(llm_connectivity, llm_connectivity, ConfigIdInput);
command_handler!(
    embedding_connectivity,
    embedding_connectivity,
    ConfigIdInput
);
command_handler!(image_connectivity, image_connectivity, ConfigIdInput);
command_handler!(llm_list, llm_list, EmptyInput);
command_handler!(image_list, image_list, EmptyInput);
command_handler!(video_list, video_list, EmptyInput);
command_handler!(embedding_list, embedding_list, EmptyInput);
command_handler!(baseline_embedding, baseline_embedding, EmptyInput);
command_handler!(baseline_llm, baseline_llm, CreateLlmCommand);
command_handler!(baseline_image, baseline_image, CreateImageCommand);
command_handler!(baseline_video, baseline_video, CreateVideoCommand);
command_handler!(create_llm, create_llm, CreateLlmCommand);
command_handler!(update_llm, update_llm, UpdateLlmCommand);
command_handler!(set_default_llm, set_default_llm, ConfigIdInput);
command_handler!(delete_llm, delete_llm, ConfigIdInput);
command_handler!(create_image, create_image, CreateImageCommand);
command_handler!(update_image, update_image, UpdateImageCommand);
command_handler!(set_default_image, set_default_image, ConfigIdInput);
command_handler!(delete_image, delete_image, ConfigIdInput);
command_handler!(create_video, create_video, CreateVideoCommand);
command_handler!(update_video, update_video, UpdateVideoCommand);
command_handler!(set_default_video, set_default_video, ConfigIdInput);
command_handler!(delete_video, delete_video, ConfigIdInput);
command_handler!(create_embedding, create_embedding, CreateEmbeddingCommand);
command_handler!(update_embedding, update_embedding, UpdateEmbeddingCommand);
command_handler!(set_default_embedding, set_default_embedding, ConfigIdInput);
command_handler!(delete_embedding, delete_embedding, ConfigIdInput);

async fn command_response<T, O, F, Fut>(
    state: &WorkerState,
    headers: &HeaderMap,
    request: WorkerEnvelope<T>,
    command: F,
) -> Response
where
    T: serde::Serialize + Clone,
    O: serde::Serialize,
    F: FnOnce(WorkerEnvelope<T>) -> Fut,
    Fut: std::future::Future<Output = Result<O, WorkerFailure>>,
{
    if !state.authorized(headers) {
        return unauthorized();
    }
    let response = match command(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

impl WorkerState {
    async fn llm_exists(&self, request: WorkerEnvelope<EmptyInput>) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_READ_LLM_EXISTS_OPERATION)?;
        a4n_db::config::llm_configs_exist(self.database.connection())
            .await
            .map_err(app_error)
    }
    async fn image_exists(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_READ_IMAGE_EXISTS_OPERATION)?;
        a4n_db::config::image_configs_exist(self.database.connection())
            .await
            .map_err(app_error)
    }
    async fn video_exists(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_READ_VIDEO_EXISTS_OPERATION)?;
        a4n_db::config::video_configs_exist(self.database.connection())
            .await
            .map_err(app_error)
    }
    async fn saved_api_key(
        &self,
        request: WorkerEnvelope<SavedApiKeyInput>,
    ) -> Result<Option<String>, WorkerFailure> {
        validate_user_request(&request, CONFIG_READ_SAVED_API_KEY_OPERATION)?;
        require_admin(&self.database, user_id(&request)?).await?;
        let config_id = positive_id(request.payload.config_id)?;
        let value = match request.payload.kind {
            ConfigProviderKindWire::Llm => {
                a4n_db::config::llm_config_by_id(self.database.connection(), config_id)
                    .await
                    .map_err(app_error)?
                    .api_key_encrypted
            }
            ConfigProviderKindWire::Image => {
                a4n_db::config::image_config_by_id(self.database.connection(), config_id)
                    .await
                    .map_err(app_error)?
                    .api_key_encrypted
            }
            ConfigProviderKindWire::Video => {
                a4n_db::config::video_config_by_id(self.database.connection(), config_id)
                    .await
                    .map_err(app_error)?
                    .api_key_encrypted
            }
        };
        Ok(value
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()))
    }
    async fn llm_connectivity(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<LlmProviderConfig, WorkerFailure> {
        validate_user_request(&request, CONFIG_READ_LLM_CONNECTIVITY_OPERATION)?;
        require_admin(&self.database, user_id(&request)?).await?;
        llm_provider_config(
            a4n_db::config::llm_config_by_id(
                self.database.connection(),
                positive_id(request.payload.config_id)?,
            )
            .await
            .map_err(app_error)?,
        )
        .map_err(app_error)
    }
    async fn embedding_connectivity(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<EmbeddingProviderConfig, WorkerFailure> {
        validate_user_request(&request, CONFIG_READ_EMBEDDING_CONNECTIVITY_OPERATION)?;
        require_admin(&self.database, user_id(&request)?).await?;
        embedding_provider_config(
            a4n_db::config::embedding_config_by_id(
                self.database.connection(),
                positive_id(request.payload.config_id)?,
            )
            .await
            .map_err(app_error)?,
        )
        .map_err(app_error)
    }
    async fn image_connectivity(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<ImageConfigForConnectivityTest, WorkerFailure> {
        validate_user_request(&request, CONFIG_READ_IMAGE_CONNECTIVITY_OPERATION)?;
        require_admin(&self.database, user_id(&request)?).await?;
        image_connectivity_config(
            a4n_db::config::image_config_by_id(
                self.database.connection(),
                positive_id(request.payload.config_id)?,
            )
            .await
            .map_err(app_error)?,
        )
        .map_err(app_error)
    }
    async fn llm_list(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<Vec<LlmConfigSummary>, WorkerFailure> {
        validate_service_request(&request, CONFIG_READ_LLM_LIST_OPERATION)?;
        a4n_db::config::list_llm_config_records(self.database.connection())
            .await
            .map_err(app_error)?
            .iter()
            .map(|model| llm_summary(model).map_err(app_error))
            .collect()
    }
    async fn image_list(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<Vec<ImageConfigSummary>, WorkerFailure> {
        validate_service_request(&request, CONFIG_READ_IMAGE_LIST_OPERATION)?;
        a4n_db::config::list_image_config_records(self.database.connection())
            .await
            .map_err(app_error)?
            .iter()
            .map(|model| image_summary(model).map_err(app_error))
            .collect()
    }
    async fn video_list(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<Vec<VideoConfigSummary>, WorkerFailure> {
        validate_service_request(&request, CONFIG_READ_VIDEO_LIST_OPERATION)?;
        a4n_db::config::list_video_config_records(self.database.connection())
            .await
            .map_err(app_error)?
            .iter()
            .map(|model| video_summary(model).map_err(app_error))
            .collect()
    }
    async fn embedding_list(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<Vec<EmbeddingConfigSummary>, WorkerFailure> {
        validate_service_request(&request, CONFIG_READ_EMBEDDING_LIST_OPERATION)?;
        a4n_db::config::list_embedding_configs(self.database.connection())
            .await
            .map_err(app_error)?
            .iter()
            .map(|model| embedding_summary(model).map_err(app_error))
            .collect()
    }

    async fn baseline_embedding(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_MUTATION_BASELINE_EMBEDDING_OPERATION)?;
        a4n_db::config::insert_baseline_embedding_config_if_empty(self.database.connection())
            .await
            .map_err(app_error)
    }

    async fn baseline_llm(
        &self,
        request: WorkerEnvelope<CreateLlmCommand>,
    ) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_MUTATION_BASELINE_LLM_OPERATION)?;
        let input = llm_record_input(normalize_llm_input(request.payload.0).map_err(app_error)?);
        a4n_db::config::insert_baseline_llm_config_if_empty(self.database.connection(), input)
            .await
            .map_err(app_error)
    }

    async fn baseline_image(
        &self,
        request: WorkerEnvelope<CreateImageCommand>,
    ) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_MUTATION_BASELINE_IMAGE_OPERATION)?;
        let (params, pricing) = image_params(request.payload.0).map_err(app_error)?;
        a4n_db::config::insert_baseline_image_config_if_empty(
            self.database.connection(),
            image_record_input(params, pricing).map_err(app_error)?,
        )
        .await
        .map_err(app_error)
    }

    async fn baseline_video(
        &self,
        request: WorkerEnvelope<CreateVideoCommand>,
    ) -> Result<bool, WorkerFailure> {
        validate_service_request(&request, CONFIG_MUTATION_BASELINE_VIDEO_OPERATION)?;
        let (params, pricing) = video_params(request.payload.0).map_err(app_error)?;
        a4n_db::config::insert_baseline_video_config_if_empty(
            self.database.connection(),
            video_record_input(params, pricing),
        )
        .await
        .map_err(app_error)
    }

    async fn create_llm(
        &self,
        request: WorkerEnvelope<CreateLlmCommand>,
    ) -> Result<LlmConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_CREATE_LLM_OPERATION,
            &self.database,
        )
        .await?;
        let model = a4n_db::config::create_llm_config_record(
            self.database.connection(),
            llm_record_input(normalize_llm_input(request.payload.0).map_err(app_error)?),
        )
        .await
        .map_err(app_error)?;
        llm_summary(&model).map_err(app_error)
    }

    async fn update_llm(
        &self,
        request: WorkerEnvelope<UpdateLlmCommand>,
    ) -> Result<LlmConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_UPDATE_LLM_OPERATION,
            &self.database,
        )
        .await?;
        let existing = a4n_db::config::llm_config_by_id(
            self.database.connection(),
            positive_id(request.payload.config_id)?,
        )
        .await
        .map_err(app_error)?;
        let input =
            normalize_update_llm_input(&existing, request.payload.input).map_err(app_error)?;
        let model = a4n_db::config::update_llm_config_record(
            self.database.connection(),
            existing,
            llm_record_input(input),
        )
        .await
        .map_err(app_error)?;
        llm_summary(&model).map_err(app_error)
    }

    async fn set_default_llm(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_SET_DEFAULT_LLM_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::llm_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        validate_llm_config(&llm_config_snapshot(&model)).map_err(app_error)?;
        a4n_db::config::set_default_provider_config(
            self.database.connection(),
            ProviderConfigKind::Llm,
            config_id,
        )
        .await
        .map_err(app_error)
    }

    async fn delete_llm(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_DELETE_LLM_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::llm_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        ensure_default_config_delete_allowed(" LLM 配置", model.is_default).map_err(app_error)?;
        a4n_db::config::delete_llm_config_record(self.database.connection(), config_id)
            .await
            .map_err(app_error)
    }

    async fn create_image(
        &self,
        request: WorkerEnvelope<CreateImageCommand>,
    ) -> Result<ImageConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_CREATE_IMAGE_OPERATION,
            &self.database,
        )
        .await?;
        let (params, pricing) = image_params(request.payload.0).map_err(app_error)?;
        let model = a4n_db::config::create_image_config_record(
            self.database.connection(),
            image_record_input(params, pricing).map_err(app_error)?,
        )
        .await
        .map_err(app_error)?;
        image_summary(&model).map_err(app_error)
    }

    async fn update_image(
        &self,
        request: WorkerEnvelope<UpdateImageCommand>,
    ) -> Result<ImageConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_UPDATE_IMAGE_OPERATION,
            &self.database,
        )
        .await?;
        let existing = a4n_db::config::image_config_by_id(
            self.database.connection(),
            positive_id(request.payload.config_id)?,
        )
        .await
        .map_err(app_error)?;
        let input = request.payload.input;
        let api_key_encrypted =
            merge_api_key_update(input.api_key_encrypted, existing.api_key_encrypted.clone());
        let pricing = pricing_json(input.pricing).map_err(app_error)?;
        let params = validate_image_config_params(a4n_domain::image::CreateImageConfigParams {
            name: input.name,
            backend: input.backend,
            base_url: input.base_url,
            api_key_encrypted,
            model: input.model,
            max_concurrent: input.max_concurrent,
            supported_sizes: input.supported_sizes,
        })
        .map_err(app_error)?;
        let model = a4n_db::config::update_image_config_record(
            self.database.connection(),
            existing,
            image_record_input(params, pricing).map_err(app_error)?,
        )
        .await
        .map_err(app_error)?;
        image_summary(&model).map_err(app_error)
    }

    async fn set_default_image(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_SET_DEFAULT_IMAGE_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::image_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        a4n_db::config::validate_image_config_model(&model).map_err(app_error)?;
        a4n_db::config::set_default_provider_config(
            self.database.connection(),
            ProviderConfigKind::Image,
            config_id,
        )
        .await
        .map_err(app_error)
    }

    async fn delete_image(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_DELETE_IMAGE_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::image_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        ensure_default_config_delete_allowed("图片配置", model.is_default).map_err(app_error)?;
        a4n_db::config::delete_image_config_record(self.database.connection(), config_id)
            .await
            .map_err(app_error)
    }

    async fn create_video(
        &self,
        request: WorkerEnvelope<CreateVideoCommand>,
    ) -> Result<VideoConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_CREATE_VIDEO_OPERATION,
            &self.database,
        )
        .await?;
        let (params, pricing) = video_params(request.payload.0).map_err(app_error)?;
        let model = a4n_db::config::create_video_config_record(
            self.database.connection(),
            video_record_input(params, pricing),
        )
        .await
        .map_err(app_error)?;
        video_summary(&model).map_err(app_error)
    }

    async fn update_video(
        &self,
        request: WorkerEnvelope<UpdateVideoCommand>,
    ) -> Result<VideoConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_UPDATE_VIDEO_OPERATION,
            &self.database,
        )
        .await?;
        let existing = a4n_db::config::video_config_by_id(
            self.database.connection(),
            positive_id(request.payload.config_id)?,
        )
        .await
        .map_err(app_error)?;
        let input = request.payload.input;
        let params = validate_video_config_params(VideoConfigParams {
            name: input.name,
            backend: input.backend,
            base_url: input.base_url,
            api_key_encrypted: merge_api_key_update(
                input.api_key_encrypted,
                existing.api_key_encrypted.clone(),
            ),
            model: input.model,
            max_concurrent: input.max_concurrent,
            default_width: input.default_width,
            default_height: input.default_height,
            default_duration_secs: input.default_duration_secs,
            min_duration_secs: input.min_duration_secs,
            max_duration_secs: input.max_duration_secs,
            duration_mode: input.duration_mode.to_string(),
            supported_durations_json: input.supported_durations_json,
            default_fps: input.default_fps,
        })
        .map_err(|error| app_error(a4n_error::AppError::InvalidArgument(error)))?;
        let model = a4n_db::config::update_video_config_record(
            self.database.connection(),
            existing,
            video_record_input(params, pricing_json(input.pricing).map_err(app_error)?),
        )
        .await
        .map_err(app_error)?;
        video_summary(&model).map_err(app_error)
    }

    async fn set_default_video(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_SET_DEFAULT_VIDEO_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::video_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        validate_video_model(&model).map_err(app_error)?;
        a4n_db::config::set_default_provider_config(
            self.database.connection(),
            ProviderConfigKind::Video,
            config_id,
        )
        .await
        .map_err(app_error)
    }

    async fn delete_video(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_DELETE_VIDEO_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::video_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        ensure_default_config_delete_allowed("视频配置", model.is_default).map_err(app_error)?;
        let active = a4n_db::config::active_generated_video_ids_for_config(
            self.database.connection(),
            config_id,
        )
        .await
        .map_err(app_error)?;
        if !active.is_empty() {
            return Err(WorkerFailure::invalid_request(format!(
                "视频配置 #{config_id} 仍有 {} 个进行中的视频任务，不能删除",
                active.len()
            )));
        }
        a4n_db::config::delete_video_config_record(self.database.connection(), config_id)
            .await
            .map_err(app_error)
    }

    async fn create_embedding(
        &self,
        request: WorkerEnvelope<CreateEmbeddingCommand>,
    ) -> Result<EmbeddingConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_CREATE_EMBEDDING_OPERATION,
            &self.database,
        )
        .await?;
        let (params, pricing) = embedding_params(request.payload.0).map_err(app_error)?;
        let model = a4n_db::config::create_embedding_config_record(
            self.database.connection(),
            embedding_record_input(params, pricing),
        )
        .await
        .map_err(app_error)?;
        embedding_summary(&model).map_err(app_error)
    }

    async fn update_embedding(
        &self,
        request: WorkerEnvelope<UpdateEmbeddingCommand>,
    ) -> Result<EmbeddingConfigSummary, WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_UPDATE_EMBEDDING_OPERATION,
            &self.database,
        )
        .await?;
        let existing = a4n_db::config::embedding_config_by_id(
            self.database.connection(),
            positive_id(request.payload.config_id)?,
        )
        .await
        .map_err(app_error)?;
        let input = request.payload.input;
        let params = validate_embedding_config_params(CreateEmbeddingConfigParams {
            name: input.name,
            backend: input.backend,
            base_url: input.base_url,
            api_key_encrypted: merge_api_key_update(
                input.api_key_encrypted,
                existing.api_key_encrypted.clone(),
            ),
            model: input.model,
            dimension: input.dimension,
            batch_size: input.batch_size,
            local_model_path: input.local_model_path,
        })
        .map_err(app_error)?;
        let model = a4n_db::config::update_embedding_config_record(
            self.database.connection(),
            existing,
            embedding_record_input(params, pricing_json(input.pricing).map_err(app_error)?),
        )
        .await
        .map_err(app_error)?;
        embedding_summary(&model).map_err(app_error)
    }

    async fn set_default_embedding(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_SET_DEFAULT_EMBEDDING_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::embedding_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        a4n_db::config::validate_embedding_config_model(&model).map_err(app_error)?;
        a4n_db::config::set_default_provider_config(
            self.database.connection(),
            ProviderConfigKind::Embedding,
            config_id,
        )
        .await
        .map_err(app_error)
    }

    async fn delete_embedding(
        &self,
        request: WorkerEnvelope<ConfigIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_admin_request(
            &request,
            CONFIG_MUTATION_DELETE_EMBEDDING_OPERATION,
            &self.database,
        )
        .await?;
        let config_id = positive_id(request.payload.config_id)?;
        let model = a4n_db::config::embedding_config_by_id(self.database.connection(), config_id)
            .await
            .map_err(app_error)?;
        ensure_default_config_delete_allowed(" Embedding 配置", model.is_default)
            .map_err(app_error)?;
        a4n_db::config::delete_embedding_config_record(self.database.connection(), config_id)
            .await
            .map_err(app_error)
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}

fn pricing_json(
    pricing: a4n_api_contract::config::PricingPlan,
) -> Result<Value, a4n_error::AppError> {
    let pricing = pricing
        .validate()
        .map_err(a4n_error::AppError::InvalidArgument)?;
    serde_json::to_value(pricing).map_err(a4n_error::AppError::Serialization)
}

fn normalize_llm_input(input: CreateLlmInput) -> Result<NormalizedLlmConfig, a4n_error::AppError> {
    let CreateLlmInput {
        name,
        provider,
        base_url,
        api_key_encrypted,
        model,
        max_concurrent,
        timeout_secs,
        max_context_tokens,
        max_output_tokens,
        agent_protocol,
        extra_body_json,
        pricing,
    } = input;
    let mut normalized = normalize_create_llm_config_input(CreateLlmConfigInput {
        name: &name,
        provider: &provider,
        base_url: &base_url,
        api_key_encrypted: api_key_encrypted.as_deref(),
        model: &model,
        max_concurrent,
        timeout_secs,
        max_context_tokens,
        max_output_tokens,
        agent_protocol,
        extra_body_json,
    })?;
    normalized.pricing_json = pricing_json(pricing)?;
    Ok(normalized)
}

fn normalize_update_llm_input(
    existing: &a4n_db_models::models::ai::llm_config::Model,
    input: UpdateLlmInput,
) -> Result<NormalizedLlmConfig, a4n_error::AppError> {
    let mut normalized = normalize_update_llm_config_input(
        existing.api_key_encrypted.as_deref(),
        UpdateLlmConfigInput {
            name: &input.name,
            provider: &input.provider,
            base_url: &input.base_url,
            api_key_encrypted: input
                .api_key_encrypted
                .as_ref()
                .map(|value| value.as_deref()),
            model: &input.model,
            max_concurrent: input.max_concurrent,
            timeout_secs: input.timeout_secs,
            max_context_tokens: input.max_context_tokens,
            max_output_tokens: input.max_output_tokens,
            agent_protocol: input.agent_protocol,
            extra_body_json: input.extra_body_json,
        },
    )?;
    normalized.pricing_json = pricing_json(input.pricing)?;
    Ok(normalized)
}

fn llm_record_input(normalized: NormalizedLlmConfig) -> LlmConfigRecordInput {
    LlmConfigRecordInput {
        name: normalized.name,
        provider: normalized.provider,
        base_url: normalized.base_url,
        api_key_encrypted: normalized.api_key_encrypted,
        model: normalized.model,
        max_concurrent: normalized.max_concurrent,
        timeout_secs: normalized.timeout_secs,
        max_context_tokens: normalized.max_context_tokens,
        max_output_tokens: normalized.max_output_tokens,
        agent_protocol: normalized.agent_protocol,
        extra_body_json: normalized.extra_body_json,
        pricing_json: normalized.pricing_json,
    }
}

fn llm_config_snapshot(model: &a4n_db_models::models::ai::llm_config::Model) -> LlmConfigSnapshot {
    LlmConfigSnapshot {
        name: model.name.clone(),
        provider: model.provider.clone(),
        base_url: model.base_url.clone(),
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        timeout_secs: model.timeout_secs,
        max_context_tokens: model.max_context_tokens,
        max_output_tokens: model.max_output_tokens,
        agent_protocol: model.agent_protocol.clone(),
        extra_body_json: model.extra_body_json.clone(),
    }
}

fn image_params(
    input: ImageConfigInput,
) -> Result<(a4n_domain::image::CreateImageConfigParams, Value), a4n_error::AppError> {
    let pricing = pricing_json(input.pricing)?;
    let params = validate_image_config_params(a4n_domain::image::CreateImageConfigParams {
        name: input.name,
        backend: input.backend,
        base_url: input.base_url,
        api_key_encrypted: input.api_key_encrypted,
        model: input.model,
        max_concurrent: input.max_concurrent,
        supported_sizes: input.supported_sizes,
    })?;
    Ok((params, pricing))
}

fn image_record_input(
    params: a4n_domain::image::CreateImageConfigParams,
    pricing_json: Value,
) -> Result<ImageConfigRecordInput, a4n_error::AppError> {
    Ok(ImageConfigRecordInput {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted,
        model: params.model,
        max_concurrent: params.max_concurrent,
        supported_sizes_json: image_sizes_to_json(&params.supported_sizes)?,
        pricing_json,
    })
}

fn video_params(
    input: VideoConfigInput,
) -> Result<(VideoConfigParams, Value), a4n_error::AppError> {
    let pricing = pricing_json(input.pricing)?;
    let params = validate_video_config_params(VideoConfigParams {
        name: input.name,
        backend: input.backend,
        base_url: input.base_url,
        api_key_encrypted: input.api_key_encrypted,
        model: input.model,
        max_concurrent: input.max_concurrent,
        default_width: input.default_width,
        default_height: input.default_height,
        default_duration_secs: input.default_duration_secs,
        min_duration_secs: input.min_duration_secs,
        max_duration_secs: input.max_duration_secs,
        duration_mode: input.duration_mode.to_string(),
        supported_durations_json: input.supported_durations_json,
        default_fps: input.default_fps,
    })
    .map_err(a4n_error::AppError::InvalidArgument)?;
    Ok((params, pricing))
}

fn video_record_input(params: VideoConfigParams, pricing_json: Value) -> VideoConfigRecordInput {
    VideoConfigRecordInput {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted,
        model: params.model,
        max_concurrent: params.max_concurrent,
        default_width: params.default_width,
        default_height: params.default_height,
        default_duration_secs: params.default_duration_secs,
        min_duration_secs: params.min_duration_secs,
        max_duration_secs: params.max_duration_secs,
        duration_mode: params.duration_mode,
        supported_durations_json: params.supported_durations_json,
        default_fps: params.default_fps,
        pricing_json,
    }
}

fn validate_video_model(
    model: &a4n_db_models::models::ai::video_gen_config::Model,
) -> Result<(), a4n_error::AppError> {
    validate_video_config_params(VideoConfigParams {
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model.base_url.clone().unwrap_or_default(),
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        default_width: model.default_width,
        default_height: model.default_height,
        default_duration_secs: model.default_duration_secs,
        min_duration_secs: model.min_duration_secs,
        max_duration_secs: model.max_duration_secs,
        duration_mode: model.duration_mode.clone(),
        supported_durations_json: model.supported_durations_json.clone(),
        default_fps: model.default_fps,
    })
    .map(|_| ())
    .map_err(a4n_error::AppError::InvalidArgument)
}

fn embedding_params(
    input: CreateEmbeddingInput,
) -> Result<(CreateEmbeddingConfigParams, Value), a4n_error::AppError> {
    let pricing = pricing_json(input.pricing)?;
    let params = a4n_config::validate_embedding_config_params(CreateEmbeddingConfigParams {
        name: input.name,
        backend: input.backend,
        base_url: input.base_url,
        api_key_encrypted: input.api_key_encrypted,
        model: input.model,
        dimension: input.dimension,
        batch_size: input.batch_size,
        local_model_path: input.local_model_path,
    })?;
    Ok((params, pricing))
}

fn embedding_record_input(
    params: CreateEmbeddingConfigParams,
    pricing_json: Value,
) -> EmbeddingConfigRecordInput {
    EmbeddingConfigRecordInput {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted,
        model: params.model,
        dimension: params.dimension,
        batch_size: params.batch_size,
        local_model_path: params.local_model_path,
        pricing_json,
    }
}

fn merge_api_key_update(
    update: Option<Option<String>>,
    existing: Option<String>,
) -> Option<String> {
    update.unwrap_or(existing)
}

async fn validate_admin_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
    database: &DbHandle,
) -> Result<(), WorkerFailure> {
    validate_user_request(request, operation)?;
    require_admin(database, user_id(request)?).await
}

fn validate_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    request
        .validate()
        .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
    if request.operation_id != operation {
        return Err(WorkerFailure::invalid_request(format!(
            "operation_id 不匹配: expected={operation}, actual={}",
            request.operation_id
        )));
    }
    if request.lease.worker_id != CONFIG_READ_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 config read worker",
        ));
    }
    Ok(())
}

fn validate_service_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    validate_request(request, operation)?;
    if request.principal.kind != WorkerPrincipalKind::Service {
        return Err(WorkerFailure::unauthorized("仅允许 service principal"));
    }
    Ok(())
}

fn validate_user_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    validate_request(request, operation)?;
    if request.principal.kind != WorkerPrincipalKind::User {
        return Err(WorkerFailure::unauthorized("仅允许用户 principal"));
    }
    Ok(())
}

fn user_id<T: serde::Serialize>(request: &WorkerEnvelope<T>) -> Result<i64, WorkerFailure> {
    request
        .principal
        .subject
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| WorkerFailure::unauthorized("用户 principal subject 无效"))
}

fn positive_id(value: i64) -> Result<i64, WorkerFailure> {
    if value > 0 {
        Ok(value)
    } else {
        Err(WorkerFailure::invalid_request("配置 ID 必须为正整数"))
    }
}

async fn require_admin(database: &DbHandle, user_id: i64) -> Result<(), WorkerFailure> {
    let user = a4n_db::auth::user_by_id(database.connection(), user_id)
        .await
        .map_err(app_error)?;
    if !user.is_active {
        return Err(WorkerFailure::unauthorized("用户已停用"));
    }
    if parse_user_role(&user.role).map_err(app_error)? != UserRole::Admin {
        return Err(WorkerFailure::unauthorized("需要管理员配置权限"));
    }
    Ok(())
}

fn app_error(error: a4n_error::AppError) -> WorkerFailure {
    match error {
        a4n_error::AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        a4n_error::AppError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_error::AppError::Forbidden(message) | a4n_error::AppError::Unauthorized(message) => {
            WorkerFailure::unauthorized(message)
        }
        a4n_error::AppError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}: {operation}"))
        }
        other => WorkerFailure::unavailable(other.to_string()),
    }
}

impl WorkerFailure {
    fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::InvalidRequest,
            message: message.into(),
            retryable: false,
        }
    }
    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Unauthorized,
            message: message.into(),
            retryable: false,
        }
    }
    fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::NotFound,
            message: message.into(),
            retryable: false,
        }
    }
    fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Conflict,
            message: message.into(),
            retryable: false,
        }
    }
    fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Unavailable,
            message: message.into(),
            retryable: true,
        }
    }
}

fn completed<T>(request: &WorkerEnvelope<impl serde::Serialize>, payload: T) -> WorkerResponse<T> {
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        trace_id: request.trace_id.clone(),
        operation: WorkerOperationRef {
            operation_id: request.operation_id.clone(),
            task_id: None,
        },
        status: WorkerOperationStatus::Completed,
        payload: Some(payload),
        resource_refs: Vec::new(),
        committed_revision: None,
        provider_job_id: None,
        result_summary: None,
        pagination: None,
        error: None,
    }
}

fn failed<T>(
    request: &WorkerEnvelope<impl serde::Serialize>,
    failure: WorkerFailure,
) -> WorkerResponse<T> {
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        trace_id: request.trace_id.clone(),
        operation: WorkerOperationRef {
            operation_id: request.operation_id.clone(),
            task_id: None,
        },
        status: WorkerOperationStatus::Failed,
        payload: None,
        resource_refs: Vec::new(),
        committed_revision: None,
        provider_job_id: None,
        result_summary: None,
        pagination: None,
        error: Some(WorkerError {
            code: failure.code,
            message: failure.message,
            retryable: failure.retryable,
        }),
    }
}

fn unauthorized() -> Response {
    StatusCode::UNAUTHORIZED.into_response()
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .try_init();
}
