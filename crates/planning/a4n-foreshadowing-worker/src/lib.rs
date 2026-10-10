//! PostgreSQL-backed foreshadowing records worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_api_contract::profile::ForeshadowingSummary;
use a4n_db::DbHandle;
use a4n_domain::character::ForeshadowingIndexRecord;
use a4n_domain::character::ForeshadowingStatus;
use a4n_foreshadowing_worker_contract::{
    CreateInput, DeleteInput, FORESHADOWING_CREATE_OPERATION, FORESHADOWING_DELETE_OPERATION,
    FORESHADOWING_LIST_OPERATION, FORESHADOWING_MUTATION_CAPABILITY, FORESHADOWING_READ_CAPABILITY,
    FORESHADOWING_UPDATE_OPERATION, FORESHADOWING_WORKER_CONTRACT_VERSION, FORESHADOWING_WORKER_ID,
    ForeshadowingMutationResult, ForeshadowingProjectInput, UpdateInput,
};
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

const WORKER_ADDR_ENV: &str = "A4N_FORESHADOWING_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_FORESHADOWING_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47656";

#[derive(Clone)]
pub struct ForeshadowingWorker {
    state: WorkerState,
}

#[derive(Clone)]
struct WorkerState {
    database: DbHandle,
    worker_id: Arc<str>,
    auth_token: Arc<str>,
}

mod mutations;
mod reads;

impl ForeshadowingWorker {
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
            .route("/v1/commands/profile.foreshadowing.list", post(list))
            .route("/v1/commands/profile.foreshadowing.create", post(create))
            .route("/v1/commands/profile.foreshadowing.update", post(update))
            .route("/v1/commands/profile.foreshadowing.delete", post(delete))
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
    ForeshadowingWorker::new(database, FORESHADOWING_WORKER_ID, auth_token)
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
                name: FORESHADOWING_READ_CAPABILITY.into(),
                version: FORESHADOWING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: FORESHADOWING_MUTATION_CAPABILITY.into(),
                version: FORESHADOWING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

async fn list(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ForeshadowingProjectInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.list(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

async fn create(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<CreateInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.create(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

async fn update(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<UpdateInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.update(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

async fn delete(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<DeleteInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.delete(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
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
    expected_operation: &str,
) -> Result<(), WorkerFailure> {
    request.validate_worker_command(
        expected_operation,
        FORESHADOWING_WORKER_ID,
        "foreshadowing worker",
    )
}

fn foreshadowing_summary(
    record: ForeshadowingIndexRecord,
) -> Result<ForeshadowingSummary, WorkerFailure> {
    let status = ForeshadowingStatus::parse(&record.status).ok_or_else(|| {
        WorkerFailure::conflict(format!("伏笔 #{} 状态非法：{}", record.id, record.status))
    })?;
    let related_characters = record
        .related_characters
        .map_or_else(|| Ok(Vec::new()), serde_json::from_value)
        .map_err(|error| {
            WorkerFailure::conflict(format!("伏笔 #{} 关联角色格式非法：{error}", record.id))
        })?;
    Ok(ForeshadowingSummary {
        id: record.id,
        project_id: record.project_id,
        description: record.description,
        setup_chapter: record.setup_chapter,
        payoff_chapter: record.payoff_chapter,
        status,
        related_characters,
        importance: record.importance,
        created_at: record.created_at.to_string(),
        updated_at: record.updated_at.to_string(),
    })
}

fn collaboration_error(error: a4n_db::collaboration::CollaborationDbError) -> WorkerFailure {
    a4n_worker_db_adapter::map_collaboration_db_error_preserving_internal(error)
}

fn foreshadowing_error(error: a4n_db::project::ForeshadowingDbError) -> WorkerFailure {
    match error {
        a4n_db::project::ForeshadowingDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ForeshadowingDbError::ProjectNotFound { project_id } => {
            WorkerFailure::not_found(format!("项目不存在: {project_id}"))
        }
        a4n_db::project::ForeshadowingDbError::RecordNotFound {
            project_id,
            foreshadowing_id,
        } => WorkerFailure::not_found(format!(
            "伏笔不存在: project_id={project_id}, foreshadowing_id={foreshadowing_id}"
        )),
        a4n_db::project::ForeshadowingDbError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{operation}: {current}"))
        }
        a4n_db::project::ForeshadowingDbError::InvalidResolutionTarget {
            foreshadowing_id,
            chapter_number,
            reason,
        } => WorkerFailure::conflict(format!(
            "第 {chapter_number} 章不能回收伏笔 {foreshadowing_id}: {reason}"
        )),
        a4n_db::project::ForeshadowingDbError::DuplicateResolutionTarget {
            foreshadowing_id,
            chapter_number,
        } => WorkerFailure::conflict(format!(
            "第 {chapter_number} 章重复回收伏笔 {foreshadowing_id}"
        )),
        a4n_db::project::ForeshadowingDbError::Database(error) => {
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
        "Foreshadowing worker authentication required",
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::{WorkerEnvelope, collaboration_error, validate_request};
    use a4n_foreshadowing_worker_contract::{
        FORESHADOWING_LIST_OPERATION, FORESHADOWING_WORKER_ID, ForeshadowingProjectInput,
    };
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn request_contract_rejects_unknown_operation() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: "unknown".into(),
            idempotency_key: "key".into(),
            project_id: Some(1),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: FORESHADOWING_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ForeshadowingProjectInput { project_id: 1 },
        };
        assert!(validate_request(&request, FORESHADOWING_LIST_OPERATION).is_err());
        assert_eq!(FORESHADOWING_LIST_OPERATION, "profile.foreshadowing.list");
    }

    #[test]
    fn collaboration_internal_errors_are_not_reported_as_conflicts() {
        let error = collaboration_error(a4n_db::collaboration::CollaborationDbError::Internal(
            "query failed".into(),
        ));

        let error: a4n_worker_contract::WorkerError = error.into();
        assert_eq!(error.code, a4n_worker_contract::WorkerErrorCode::Internal);
        assert!(!error.retryable);
    }
}
