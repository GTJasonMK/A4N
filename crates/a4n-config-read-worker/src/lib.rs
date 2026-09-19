//! PostgreSQL-backed provider configuration read worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_api_contract::config::{
    EmbeddingConfigSummary, ImageConfigForConnectivityTest, ImageConfigSummary, LlmConfigSummary,
    VideoConfigSummary,
};
use a4n_config_read_core::{
    embedding_provider_config, embedding_summary, image_connectivity_config, image_summary,
    llm_provider_config, llm_summary, video_summary,
};
use a4n_config_read_worker_contract::{
    CONFIG_READ_CAPABILITY, CONFIG_READ_EMBEDDING_CONNECTIVITY_OPERATION,
    CONFIG_READ_EMBEDDING_LIST_OPERATION, CONFIG_READ_IMAGE_CONNECTIVITY_OPERATION,
    CONFIG_READ_IMAGE_EXISTS_OPERATION, CONFIG_READ_IMAGE_LIST_OPERATION,
    CONFIG_READ_LLM_CONNECTIVITY_OPERATION, CONFIG_READ_LLM_EXISTS_OPERATION,
    CONFIG_READ_LLM_LIST_OPERATION, CONFIG_READ_SAVED_API_KEY_OPERATION,
    CONFIG_READ_VIDEO_EXISTS_OPERATION, CONFIG_READ_VIDEO_LIST_OPERATION,
    CONFIG_READ_WORKER_CONTRACT_VERSION, CONFIG_READ_WORKER_ID, ConfigIdInput,
    ConfigProviderKindWire, EmptyInput, SavedApiKeyInput,
};
use a4n_db::DbHandle;
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
        capabilities: vec![WorkerCapability {
            name: CONFIG_READ_CAPABILITY.into(),
            version: CONFIG_READ_WORKER_CONTRACT_VERSION.into(),
            enabled: true,
        }],
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

    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
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
