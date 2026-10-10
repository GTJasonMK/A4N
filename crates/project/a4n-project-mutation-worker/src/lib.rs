//! PostgreSQL-backed project creation and update worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_db::DbHandle;
use a4n_domain::{
    collaboration::ProjectAccessRole,
    project::{optional_text, required_text},
};
use a4n_project_mutation_worker_contract::{
    CreateCanvasProjectInput, CreateDerivedProjectInput, CreateProjectInput,
    CreateStandaloneComicProjectInput, PROJECT_MUTATION_CAPABILITY,
    PROJECT_MUTATION_CREATE_CANVAS_OPERATION, PROJECT_MUTATION_CREATE_DERIVED_OPERATION,
    PROJECT_MUTATION_CREATE_OPERATION, PROJECT_MUTATION_CREATE_STANDALONE_COMIC_OPERATION,
    PROJECT_MUTATION_UPDATE_OPERATION, PROJECT_MUTATION_WORKER_CONTRACT_VERSION,
    PROJECT_MUTATION_WORKER_ID, ProjectSummary, UpdateProjectInput,
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

const WORKER_ADDR_ENV: &str = "A4N_PROJECT_MUTATION_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_PROJECT_MUTATION_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47669";

#[derive(Clone)]
pub struct ProjectMutationWorker {
    state: WorkerState,
}

#[derive(Clone)]
struct WorkerState {
    database: DbHandle,
    worker_id: Arc<str>,
    auth_token: Arc<str>,
}

mod mutations;

impl ProjectMutationWorker {
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
            .route("/v1/commands/project.mutation.create", post(create))
            .route(
                "/v1/commands/project.mutation.create_derived",
                post(create_derived),
            )
            .route(
                "/v1/commands/project.mutation.create_canvas",
                post(create_canvas),
            )
            .route(
                "/v1/commands/project.mutation.create_standalone_comic",
                post(create_standalone_comic),
            )
            .route("/v1/commands/project.mutation.update", post(update))
            .with_state(self.state.clone())
    }

    pub async fn serve(self, address: SocketAddr) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(address).await?;
        axum::serve(listener, self.router())
            .with_graceful_shutdown(a4n_worker_runtime::wait_for_shutdown_signal())
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
    ProjectMutationWorker::new(database, PROJECT_MUTATION_WORKER_ID, auth_token)
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
            name: PROJECT_MUTATION_CAPABILITY.into(),
            version: PROJECT_MUTATION_WORKER_CONTRACT_VERSION.into(),
            enabled: true,
        }],
    })
    .into_response()
}

async fn create(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<CreateProjectInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.create(request)).await
}

async fn create_derived(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<CreateDerivedProjectInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.create_derived(request)
    })
    .await
}

async fn create_canvas(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<CreateCanvasProjectInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.create_canvas(request)
    })
    .await
}

async fn create_standalone_comic(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<CreateStandaloneComicProjectInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.create_standalone_comic(request)
    })
    .await
}

async fn update(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<UpdateProjectInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.update(request)).await
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

fn request_validate<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    request.validate_worker_command(
        operation,
        PROJECT_MUTATION_WORKER_ID,
        "project mutation worker",
    )
}

fn unscoped_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    request_validate(request, operation)?;
    if request.project_id.is_some() {
        return Err(WorkerFailure::invalid_request(
            "该项目创建操作不接受 project scope",
        ));
    }
    Ok(())
}

fn project_scope<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
    payload_project_id: i64,
) -> Result<i64, WorkerFailure> {
    request_validate(request, operation)?;
    if request.project_id.is_none() {
        return Err(WorkerFailure::invalid_request("项目 scope 缺少 project_id"));
    }
    request.require_project_scope(payload_project_id)
}

fn summary_error(error: a4n_project_read_core::ProjectSummaryError) -> WorkerFailure {
    match error {
        a4n_project_read_core::ProjectSummaryError::Database(error) => {
            a4n_worker_db_adapter::map_database_error(error)
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

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{WorkerLease, WorkerPrincipal, WorkerPrincipalKind};

    #[test]
    fn update_scope_rejects_payload_mismatch() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: PROJECT_MUTATION_UPDATE_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(9),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: PROJECT_MUTATION_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: UpdateProjectInput {
                project_id: 8,
                title: None,
                description: None,
                fact_feedback_interval_chapters: None,
            },
        };
        assert!(project_scope(&request, PROJECT_MUTATION_UPDATE_OPERATION, 8).is_err());
    }
}
