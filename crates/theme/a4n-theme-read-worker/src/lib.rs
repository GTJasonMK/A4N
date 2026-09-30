//! PostgreSQL-backed theme read worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_api_contract::theme::{ThemePreferenceSummary, ThemeResourceRecord, ThemeSummary};
use a4n_db::DbHandle;
use a4n_error::AppError;
use a4n_theme_core::{
    theme_preference_summary, theme_resource_record, theme_summary_with_resources,
};
use a4n_theme_read_worker_contract::{
    ResourceIdInput, THEME_ACTIVATE_OPERATION, THEME_CREATE_OPERATION, THEME_DELETE_OPERATION,
    THEME_EXPORT_OPERATION, THEME_IMPORT_OPERATION, THEME_LIST_OPERATION,
    THEME_MUTATION_CAPABILITY, THEME_PREFERENCE_OPERATION, THEME_PREFERENCE_UPDATE_OPERATION,
    THEME_READ_CAPABILITY, THEME_READ_WORKER_CONTRACT_VERSION, THEME_READ_WORKER_ID,
    THEME_RESOURCE_DELETE_OPERATION, THEME_RESOURCE_UPSERT_OPERATION, THEME_RESOURCES_OPERATION,
    THEME_UPDATE_OPERATION, ThemeIdInput, ThemeUpdateInput, ThemeUserInput,
};
use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerHealth,
    WorkerHealthStatus, WorkerOperationRef, WorkerOperationStatus, WorkerResponse,
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

const WORKER_ADDR_ENV: &str = "A4N_THEME_READ_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_THEME_READ_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47663";

#[derive(Clone)]
pub struct ThemeReadWorker {
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

mod mutations;
mod reads;

impl ThemeReadWorker {
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
            .route("/v1/commands/theme.read.list", post(list))
            .route("/v1/commands/theme.read.export", post(export))
            .route("/v1/commands/theme.read.resources", post(resources))
            .route("/v1/commands/theme.read.preference", post(preference))
            .route("/v1/commands/theme.mutation.create", post(create_theme))
            .route("/v1/commands/theme.mutation.update", post(update_theme))
            .route("/v1/commands/theme.mutation.delete", post(delete_theme))
            .route("/v1/commands/theme.mutation.activate", post(activate_theme))
            .route("/v1/commands/theme.mutation.import", post(import_theme))
            .route(
                "/v1/commands/theme.mutation.resource_upsert",
                post(upsert_resource),
            )
            .route(
                "/v1/commands/theme.mutation.resource_delete",
                post(delete_resource),
            )
            .route(
                "/v1/commands/theme.mutation.preference_update",
                post(update_preference),
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
    ThemeReadWorker::new(database, THEME_READ_WORKER_ID, auth_token)
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
                name: THEME_READ_CAPABILITY.into(),
                version: THEME_READ_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: THEME_MUTATION_CAPABILITY.into(),
                version: THEME_READ_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

async fn list(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ThemeUserInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.list(request)).await
}

async fn export(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ThemeIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.export(request)).await
}

async fn resources(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ThemeIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.resources(request)
    })
    .await
}

async fn preference(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ThemeUserInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.preference(request)
    })
    .await
}

async fn create_theme(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<a4n_api_contract::theme::CreateThemeInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.create_theme(request)
    })
    .await
}

async fn update_theme(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ThemeUpdateInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.update_theme(request)
    })
    .await
}

async fn delete_theme(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ThemeIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.delete_theme(request)
    })
    .await
}

async fn activate_theme(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ThemeIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.activate_theme(request)
    })
    .await
}

async fn import_theme(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<a4n_api_contract::theme::CreateThemeInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.import_theme(request)
    })
    .await
}

async fn upsert_resource(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<a4n_api_contract::theme::PersistThemeResourceInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.upsert_resource(request)
    })
    .await
}

async fn delete_resource(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ResourceIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.delete_resource(request)
    })
    .await
}

async fn update_preference(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<a4n_api_contract::theme::UpdateThemePreferenceInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.update_preference(request)
    })
    .await
}

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
    async fn summary(
        &self,
        theme: a4n_db_models::models::user::theme_config::Model,
    ) -> Result<ThemeSummary, WorkerFailure> {
        let resources = a4n_db::config::list_theme_resources(self.database.connection(), theme.id)
            .await
            .map_err(app_error)?;
        theme_summary_with_resources(theme, resources).map_err(app_error)
    }

    async fn require_user(&self, user_id: i64) -> Result<(), WorkerFailure> {
        if user_id <= 0 {
            return Err(WorkerFailure::unauthorized("用户 ID 无效"));
        }
        Ok(())
    }

    async fn require_owner(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<a4n_db_models::models::user::theme_config::Model, WorkerFailure> {
        let theme = a4n_db::config::theme_by_id(self.database.connection(), theme_id)
            .await
            .map_err(app_error)?
            .ok_or_else(|| WorkerFailure::not_found("主题不存在"))?;
        if theme.user_id != Some(user_id) {
            return Err(WorkerFailure::unauthorized("只能修改当前登录用户主题"));
        }
        Ok(theme)
    }

    async fn require_theme_access(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<a4n_db_models::models::user::theme_config::Model, WorkerFailure> {
        if theme_id <= 0 {
            return Err(WorkerFailure::invalid_request("theme_id 必须大于 0"));
        }
        let theme = a4n_db::config::theme_by_id(self.database.connection(), theme_id)
            .await
            .map_err(app_error)?
            .ok_or_else(|| WorkerFailure::not_found("主题不存在"))?;
        if theme.user_id.is_some_and(|owner_id| owner_id != user_id) {
            return Err(WorkerFailure::unauthorized("只能访问当前登录用户主题"));
        }
        Ok(theme)
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
    if request.lease.worker_id != THEME_READ_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 theme read worker",
        ));
    }
    Ok(())
}

fn normalize_name(name: String) -> Result<String, WorkerFailure> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(WorkerFailure::invalid_request("主题名称不能为空"));
    }
    Ok(name)
}

fn user_id<T>(request: &WorkerEnvelope<T>) -> Result<i64, WorkerFailure> {
    if request.principal.kind != a4n_worker_contract::WorkerPrincipalKind::User {
        return Err(WorkerFailure::unauthorized("仅允许用户 principal"));
    }
    request
        .principal
        .subject
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| WorkerFailure::unauthorized("用户 principal subject 无效"))
}

fn app_error(error: AppError) -> WorkerFailure {
    match error {
        AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        AppError::NotFound(message) => WorkerFailure::not_found(message),
        AppError::Forbidden(message) | AppError::Unauthorized(message) => {
            WorkerFailure::unauthorized(message)
        }
        AppError::ConfigError(message)
        | AppError::InvalidState {
            current: message, ..
        } => WorkerFailure::conflict(message),
        error => WorkerFailure::unavailable(error.to_string()),
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

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .try_init();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
