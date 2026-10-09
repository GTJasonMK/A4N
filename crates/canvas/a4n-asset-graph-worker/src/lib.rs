//! PostgreSQL-backed project asset dependency graph read worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_asset_graph_worker_contract::{
    ASSET_GRAPH_NODE_DETAIL_OPERATION, ASSET_GRAPH_OPERATION, ASSET_GRAPH_READ_CAPABILITY,
    ASSET_GRAPH_WORKER_CONTRACT_VERSION, ASSET_GRAPH_WORKER_ID, ProjectAssetGraphInput,
    ProjectAssetGraphNodeDetailInput,
};
use a4n_db::DbHandle;
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_error::AppError;
use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerFailure, WorkerHealth, WorkerHealthStatus,
    WorkerResponse,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_ASSET_GRAPH_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_ASSET_GRAPH_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47655";

#[derive(Clone)]
pub struct AssetGraphWorker {
    state: WorkerState,
}

#[derive(Clone)]
struct WorkerState {
    database: DbHandle,
    worker_id: Arc<str>,
    auth_token: Arc<str>,
}

mod reads;

impl AssetGraphWorker {
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
            .route("/v1/commands/project.asset_graph.read", post(graph))
            .route(
                "/v1/commands/project.asset_graph.node_detail",
                post(node_detail),
            )
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
    AssetGraphWorker::new(database, ASSET_GRAPH_WORKER_ID, auth_token)
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
            name: ASSET_GRAPH_READ_CAPABILITY.into(),
            version: ASSET_GRAPH_WORKER_CONTRACT_VERSION.into(),
            enabled: true,
        }],
    })
    .into_response()
}

async fn graph(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectAssetGraphInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.graph(request)).await
}

async fn node_detail(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectAssetGraphNodeDetailInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.node_detail(request)
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
    async fn verify_permission(&self, project_id: i64, user_id: i64) -> Result<(), WorkerFailure> {
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await
        .map(|_| ())
        .map_err(collaboration_error)
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
    if request.lease.worker_id != ASSET_GRAPH_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 asset graph worker",
        ));
    }
    Ok(())
}

fn validate_project_scope<T>(
    request: &WorkerEnvelope<T>,
    project_id: i64,
) -> Result<(), WorkerFailure> {
    if project_id <= 0 || request.project_id != Some(project_id) {
        return Err(WorkerFailure::invalid_request(
            "worker scope project_id 与 payload 不一致或无效",
        ));
    }
    Ok(())
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
        AppError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{operation}: {current}"))
        }
        other => WorkerFailure::unavailable(other.to_string()),
    }
}

fn collaboration_error(error: a4n_db::collaboration::CollaborationDbError) -> WorkerFailure {
    match error {
        a4n_db::collaboration::CollaborationDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::collaboration::CollaborationDbError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::collaboration::CollaborationDbError::Forbidden(message) => {
            WorkerFailure::unauthorized(message)
        }
        a4n_db::collaboration::CollaborationDbError::InvalidState(message)
        | a4n_db::collaboration::CollaborationDbError::Internal(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::collaboration::CollaborationDbError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

fn completed<T>(request: &WorkerEnvelope<impl serde::Serialize>, payload: T) -> WorkerResponse<T> {
    WorkerResponse::completed_for(request, payload)
}

fn failed<T>(
    request: &WorkerEnvelope<impl serde::Serialize>,
    failure: WorkerFailure,
) -> WorkerResponse<T> {
    WorkerResponse::failed_for(request, failure.into())
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        "Asset graph worker authentication required",
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::{validate_project_scope, validate_request};
    use a4n_asset_graph_worker_contract::{
        ASSET_GRAPH_OPERATION, ASSET_GRAPH_WORKER_ID, ProjectAssetGraphInput,
    };
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    fn request(project_id: Option<i64>, operation: &str) -> WorkerEnvelope<ProjectAssetGraphInput> {
        WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: operation.into(),
            idempotency_key: "key".into(),
            project_id,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: ASSET_GRAPH_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ProjectAssetGraphInput { project_id: 1 },
        }
    }

    #[test]
    fn project_scope_rejects_mismatched_ids() {
        let request = request(Some(2), ASSET_GRAPH_OPERATION);
        assert!(validate_request(&request, ASSET_GRAPH_OPERATION).is_ok());
        assert!(validate_project_scope(&request, 1).is_err());
    }
}
