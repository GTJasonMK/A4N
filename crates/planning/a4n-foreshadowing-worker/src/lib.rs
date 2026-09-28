//! PostgreSQL-backed foreshadowing records worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_api_contract::profile::ForeshadowingSummary;
use a4n_db::DbHandle;
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

#[derive(Debug)]
struct WorkerFailure {
    code: WorkerErrorCode,
    message: String,
    retryable: bool,
}

impl ForeshadowingWorker {
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
            .route("/v1/commands/profile.foreshadowing.list", post(list))
            .route("/v1/commands/profile.foreshadowing.create", post(create))
            .route("/v1/commands/profile.foreshadowing.update", post(update))
            .route("/v1/commands/profile.foreshadowing.delete", post(delete))
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
    ForeshadowingWorker::new(database, FORESHADOWING_WORKER_ID, auth_token)
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
    async fn list(
        &self,
        request: WorkerEnvelope<ForeshadowingProjectInput>,
    ) -> Result<Vec<ForeshadowingSummary>, WorkerFailure> {
        validate_request(&request, FORESHADOWING_LIST_OPERATION)?;
        let project_id = request.payload.project_id;
        if request.project_id != Some(project_id) || project_id <= 0 {
            return Err(WorkerFailure::invalid_request(
                "worker scope project_id 与 payload 不一致或无效",
            ));
        }
        let user_id = user_id(&request)?;
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Viewer,
        )
        .await
        .map_err(collaboration_error)?;
        a4n_db::project::foreshadowing_records_by_project(self.database.connection(), project_id)
            .await
            .map_err(foreshadowing_error)?
            .into_iter()
            .map(foreshadowing_summary)
            .collect()
    }

    async fn create(
        &self,
        request: WorkerEnvelope<CreateInput>,
    ) -> Result<ForeshadowingMutationResult, WorkerFailure> {
        validate_request(&request, FORESHADOWING_CREATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload.0;
        let project_id = input.project_id;
        ensure_project_scope(request.project_id, project_id)?;
        self.verify_editor(project_id, user_id).await?;
        let record = a4n_db::project::create_foreshadowing_record(
            self.database.connection(),
            project_id,
            input.record,
        )
        .await
        .map_err(foreshadowing_error)?;
        Ok(ForeshadowingMutationResult {
            project_id,
            foreshadowing_id: record.id,
        })
    }

    async fn update(
        &self,
        request: WorkerEnvelope<UpdateInput>,
    ) -> Result<ForeshadowingMutationResult, WorkerFailure> {
        validate_request(&request, FORESHADOWING_UPDATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload.0;
        let project_id = input.project_id;
        ensure_project_scope(request.project_id, project_id)?;
        self.verify_editor(project_id, user_id).await?;
        let record = a4n_db::project::update_foreshadowing_record(
            self.database.connection(),
            project_id,
            input.foreshadowing_id,
            &input.expected_updated_at,
            input.record,
        )
        .await
        .map_err(foreshadowing_error)?;
        Ok(ForeshadowingMutationResult {
            project_id,
            foreshadowing_id: record.id,
        })
    }

    async fn delete(
        &self,
        request: WorkerEnvelope<DeleteInput>,
    ) -> Result<ForeshadowingMutationResult, WorkerFailure> {
        validate_request(&request, FORESHADOWING_DELETE_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload.0;
        let project_id = input.project_id;
        ensure_project_scope(request.project_id, project_id)?;
        self.verify_editor(project_id, user_id).await?;
        let record = a4n_db::project::delete_foreshadowing_record(
            self.database.connection(),
            project_id,
            input.foreshadowing_id,
            &input.expected_updated_at,
        )
        .await
        .map_err(foreshadowing_error)?;
        Ok(ForeshadowingMutationResult {
            project_id,
            foreshadowing_id: record.id,
        })
    }

    async fn verify_editor(&self, project_id: i64, user_id: i64) -> Result<(), WorkerFailure> {
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
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
    expected_operation: &str,
) -> Result<(), WorkerFailure> {
    request
        .validate()
        .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
    if request.operation_id != expected_operation {
        return Err(WorkerFailure::invalid_request(format!(
            "operation_id 不匹配: expected={}, actual={}",
            expected_operation, request.operation_id
        )));
    }
    if request.lease.worker_id != FORESHADOWING_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 foreshadowing worker",
        ));
    }
    Ok(())
}

fn ensure_project_scope(
    scope_project_id: Option<i64>,
    payload_project_id: i64,
) -> Result<(), WorkerFailure> {
    if payload_project_id <= 0 || scope_project_id != Some(payload_project_id) {
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

fn foreshadowing_summary(
    record: a4n_db_models::models::character::foreshadowing_index::Model,
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
        a4n_db::collaboration::CollaborationDbError::InvalidState(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::collaboration::CollaborationDbError::Internal(message) => {
            WorkerFailure::internal(message)
        }
        a4n_db::collaboration::CollaborationDbError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
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
    fn internal(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Internal,
            message: message.into(),
            retryable: false,
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
    (
        StatusCode::UNAUTHORIZED,
        "Foreshadowing worker authentication required",
    )
        .into_response()
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

#[cfg(test)]
mod tests {
    use super::{WorkerEnvelope, collaboration_error, ensure_project_scope, validate_request};
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
    fn mutation_scope_must_match_request_project() {
        assert!(ensure_project_scope(Some(7), 7).is_ok());
        assert!(ensure_project_scope(Some(8), 7).is_err());
        assert!(ensure_project_scope(None, 7).is_err());
        assert!(ensure_project_scope(Some(7), 0).is_err());
    }

    #[test]
    fn collaboration_internal_errors_are_not_reported_as_conflicts() {
        let error = collaboration_error(a4n_db::collaboration::CollaborationDbError::Internal(
            "query failed".into(),
        ));

        assert_eq!(error.code, a4n_worker_contract::WorkerErrorCode::Internal);
        assert!(!error.retryable);
    }
}
