//! PostgreSQL-backed project-status read worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_api_contract::project::{
    ProjectRollbackPreview, ProjectStatus, ProjectStatusMachineSummary, ProjectStatusMutationResult,
};
use a4n_db::DbHandle;
use a4n_domain::{collaboration::ProjectAccessRole, project::rollback_actions};
use a4n_project_status_worker_contract::{
    PROJECT_STATUS_FORWARD_OPERATION, PROJECT_STATUS_MACHINE_OPERATION,
    PROJECT_STATUS_PREVIEW_ROLLBACK_OPERATION, PROJECT_STATUS_READ_CAPABILITY,
    PROJECT_STATUS_ROLLBACK_OPERATION, PROJECT_STATUS_WORKER_CONTRACT_VERSION,
    PROJECT_STATUS_WORKER_ID, PROJECT_STATUS_WRITE_CAPABILITY, ProjectStatusProjectInput,
    ProjectStatusRollbackInput,
};
use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerFailure, WorkerHealth, WorkerHealthStatus,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};

const WORKER_ADDR_ENV: &str = "A4N_PROJECT_STATUS_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_PROJECT_STATUS_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47662";

#[derive(Clone)]
pub struct ProjectStatusWorker {
    state: WorkerState,
}

#[derive(Clone)]
struct WorkerState {
    database: DbHandle,
    worker_id: Arc<str>,
    auth_token: Arc<str>,
}

mod operations;

impl ProjectStatusWorker {
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
            .route("/v1/commands/project.status.machine", post(status_machine))
            .route(
                "/v1/commands/project.status.preview_rollback",
                post(preview_rollback),
            )
            .route("/v1/commands/project.status.forward", post(forward))
            .route("/v1/commands/project.status.rollback", post(rollback))
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
    ProjectStatusWorker::new(database, PROJECT_STATUS_WORKER_ID, auth_token)
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
        capabilities: vec![
            WorkerCapability {
                name: PROJECT_STATUS_READ_CAPABILITY.into(),
                version: PROJECT_STATUS_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: PROJECT_STATUS_WRITE_CAPABILITY.into(),
                version: PROJECT_STATUS_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

async fn status_machine(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectStatusProjectInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.status_machine(request)
    })
    .await
}

async fn preview_rollback(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectStatusRollbackInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.preview_rollback(request)
    })
    .await
}

async fn forward(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectStatusRollbackInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.forward(request)).await
}

async fn rollback(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectStatusRollbackInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.rollback(request)).await
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

async fn verify_viewer(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(
        database.connection(),
        project_id,
        user_id,
        a4n_domain::collaboration::ProjectAccessRole::Viewer,
    )
    .await
    .map(|_| ())
    .map_err(a4n_worker_db_adapter::map_collaboration_db_error)
}

async fn verify_permission(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
    required: ProjectAccessRole,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(
        database.connection(),
        project_id,
        user_id,
        required,
    )
    .await
    .map(|_| ())
    .map_err(a4n_worker_db_adapter::map_collaboration_db_error)
}

fn validate_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    request.validate_worker_command(operation, PROJECT_STATUS_WORKER_ID, "project status worker")
}

fn project_mutation_error(error: a4n_db::project::ProjectMutationError) -> WorkerFailure {
    a4n_worker_db_adapter::map_project_mutation_error(error)
}

fn status_write_error(error: a4n_db::project::ProjectStatusWriteError) -> WorkerFailure {
    match error {
        a4n_db::project::ProjectStatusWriteError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ProjectStatusWriteError::ProjectNotFound { project_id } => {
            WorkerFailure::not_found(format!("项目不存在: {project_id}"))
        }
        a4n_db::project::ProjectStatusWriteError::StatusChanged { expected, actual } => {
            WorkerFailure::conflict(format!(
                "项目状态已变化，请刷新后重试: expected={expected}, actual={actual}"
            ))
        }
        a4n_db::project::ProjectStatusWriteError::Database { source } => {
            WorkerFailure::unavailable(source.to_string())
        }
    }
}

fn project_reset_error(error: a4n_db::project::reset::ProjectResetDbError) -> WorkerFailure {
    match error {
        a4n_db::project::reset::ProjectResetDbError::ProjectNotFound { project_id } => {
            WorkerFailure::not_found(format!("项目不存在: {project_id}"))
        }
        a4n_db::project::reset::ProjectResetDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::reset::ProjectResetDbError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("状态 {current} 不允许{operation}"))
        }
        a4n_db::project::reset::ProjectResetDbError::StatusChanged { expected, actual } => {
            WorkerFailure::conflict(format!(
                "项目状态已变化，请刷新后重试: expected={expected}, actual={actual}"
            ))
        }
        a4n_db::project::reset::ProjectResetDbError::Database { source } => {
            WorkerFailure::unavailable(source.to_string())
        }
        other => WorkerFailure::conflict(other.to_string()),
    }
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

fn status_read_error(error: a4n_db::project::ProjectStatusReadError) -> WorkerFailure {
    match error {
        a4n_db::project::ProjectStatusReadError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ProjectStatusReadError::Database { source, .. } => {
            WorkerFailure::unavailable(source.to_string())
        }
    }
}

fn rollback_impact_error(error: a4n_db::project::ProjectRollbackImpactReadError) -> WorkerFailure {
    match error {
        a4n_db::project::ProjectRollbackImpactReadError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ProjectRollbackImpactReadError::Database { source, .. } => {
            WorkerFailure::unavailable(source.to_string())
        }
    }
}

fn unauthorized() -> Response {
    StatusCode::UNAUTHORIZED.into_response()
}
