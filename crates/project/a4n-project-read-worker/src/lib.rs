//! PostgreSQL-backed project summary/list read worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_db::DbHandle;
use a4n_project_read_core::project_summary_from_parts;
use a4n_project_read_worker_contract::{
    PROJECT_READ_CAPABILITY, PROJECT_READ_EXISTS_OPERATION, PROJECT_READ_IDS_OPERATION,
    PROJECT_READ_LIST_OPERATION, PROJECT_READ_SUMMARY_OPERATION,
    PROJECT_READ_WORKER_CONTRACT_VERSION, PROJECT_READ_WORKER_ID, PaginatedProjects,
    ProjectIdInput, ProjectListInput, ProjectServiceInput, ProjectSummary,
};
use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerFailure, WorkerHealth, WorkerHealthStatus,
    WorkerPrincipalKind,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};

const WORKER_ADDR_ENV: &str = "A4N_PROJECT_READ_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_PROJECT_READ_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47664";

#[derive(Clone)]
pub struct ProjectReadWorker {
    state: WorkerState,
}

#[derive(Clone)]
struct WorkerState {
    database: DbHandle,
    worker_id: Arc<str>,
    auth_token: Arc<str>,
}

mod reads;

impl ProjectReadWorker {
    pub fn new(
        database: DbHandle,
        worker_id: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Self {
        Self {
            state: WorkerState {
                database,
                worker_id: Arc::<str>::from(worker_id.into()),
                auth_token: Arc::<str>::from(auth_token.into()),
            },
        }
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route(WORKER_HEALTH_PATH, get(health))
            .route(WORKER_CAPABILITIES_PATH, get(capabilities))
            .route("/v1/commands/project.read.summary", post(summary))
            .route("/v1/commands/project.read.list", post(list))
            .route("/v1/commands/project.read.ids", post(ids))
            .route("/v1/commands/project.read.exists", post(exists))
            .with_state(self.state.clone())
    }

    pub async fn serve(self, address: SocketAddr) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(address).await?;
        axum::serve(listener, self.router())
            .with_graceful_shutdown(a4n_worker_runtime::wait_for_ctrl_c())
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
    a4n_worker_runtime::init_worker_tracing();
    let database_url = a4n_config::required_non_empty_env_string(DATABASE_URL_ENV)?;
    a4n_config::validate_postgres_database_url(DATABASE_URL_ENV, &database_url)?;
    let auth_token = a4n_config::required_non_empty_env_string(WORKER_TOKEN_ENV)?;
    let address = a4n_config::worker_listen_addr(WORKER_ADDR_ENV, DEFAULT_ADDR)?;
    let database = DbHandle::connect(database_url).await?;
    ProjectReadWorker::new(database, PROJECT_READ_WORKER_ID, auth_token)
        .serve(address)
        .await
}

async fn health(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let status = match state.database.ping().await {
        Ok(()) => WorkerHealthStatus::Healthy,
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
            name: PROJECT_READ_CAPABILITY.into(),
            version: PROJECT_READ_WORKER_CONTRACT_VERSION.into(),
            enabled: true,
        }],
    })
    .into_response()
}

async fn summary(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.summary(request)).await
}

async fn list(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectListInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.list(request)).await
}

async fn ids(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectServiceInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.ids(request)).await
}

async fn exists(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.exists(request)).await
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
    a4n_worker_runtime::worker_command_response(
        state.authorized(headers),
        request,
        command,
        unauthorized,
    )
    .await
}

impl WorkerState {
    fn authorized(&self, headers: &HeaderMap) -> bool {
        a4n_worker_runtime::worker_token_authorized(
            headers,
            WORKER_AUTH_HEADER,
            self.auth_token.as_bytes(),
        )
    }
}

fn validate_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
    service: bool,
) -> Result<(), WorkerFailure> {
    request.validate_worker_command(operation, PROJECT_READ_WORKER_ID, "project read worker")?;
    if service {
        request.principal.require_kind(
            WorkerPrincipalKind::Service,
            "该操作仅允许 service principal",
        )?;
    } else {
        request
            .principal
            .require_kind(WorkerPrincipalKind::User, "该操作仅允许 user principal")?;
    }
    Ok(())
}

fn service_principal<T>(request: &WorkerEnvelope<T>) -> Result<(), WorkerFailure> {
    if request.principal.subject != "runtime" {
        return Err(WorkerFailure::unauthorized(
            "service principal subject 无效",
        ));
    }
    Ok(())
}

fn summary_error(error: a4n_project_read_core::ProjectSummaryError) -> WorkerFailure {
    match error {
        a4n_project_read_core::ProjectSummaryError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
        a4n_project_read_core::ProjectSummaryError::Progress(error) => {
            a4n_worker_db_adapter::map_project_progress_read_error(error)
        }
        a4n_project_read_core::ProjectSummaryError::Family(error) => {
            a4n_worker_db_adapter::map_project_family_metadata_read_error(error)
        }
        a4n_project_read_core::ProjectSummaryError::Collaboration(error) => {
            a4n_worker_db_adapter::map_collaboration_db_error(error)
        }
    }
}

fn unauthorized() -> Response {
    StatusCode::UNAUTHORIZED.into_response()
}
