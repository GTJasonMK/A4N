//! PostgreSQL-backed workflow read worker.

use std::{collections::HashSet, env, net::SocketAddr, sync::Arc};

use a4n_api_contract::task::{GetTaskHistoryInput, TaskInfoSummary};
use a4n_api_contract::workflow::{GetWorkflowRunInput, ListWorkflowRunsInput};
use a4n_db::DbHandle;
use a4n_db_models::models::user::role::{UserRole, parse_user_role};
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_worker_client::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerHealth,
    WorkerHealthStatus, WorkerOperationRef, WorkerOperationStatus, WorkerPrincipalKind,
    WorkerResponse,
};
use a4n_workflow_core::{workflow_run_detail_to_api, workflow_run_summary_to_api};
use a4n_workflow_read_worker_contract::{
    TASK_READ_HISTORY_OPERATION, WORKFLOW_READ_CAPABILITY, WORKFLOW_READ_DETAIL_OPERATION,
    WORKFLOW_READ_LIST_OPERATION, WORKFLOW_READ_WORKER_CONTRACT_VERSION, WORKFLOW_READ_WORKER_ID,
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

const WORKER_ADDR_ENV: &str = "A4N_WORKFLOW_READ_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_WORKFLOW_READ_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47672";

#[derive(Clone)]
pub struct WorkflowReadWorker {
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

impl WorkflowReadWorker {
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
            .route("/v1/commands/workflow.read.list", post(list))
            .route("/v1/commands/workflow.read.detail", post(detail))
            .route("/v1/commands/task.read.history", post(task_history))
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
    WorkflowReadWorker::new(database, WORKFLOW_READ_WORKER_ID, auth_token)
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
            name: WORKFLOW_READ_CAPABILITY.into(),
            version: WORKFLOW_READ_WORKER_CONTRACT_VERSION.into(),
            enabled: true,
        }],
    })
    .into_response()
}

async fn list(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ListWorkflowRunsInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.list(request)).await
}

async fn detail(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<GetWorkflowRunInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.detail(request)).await
}

async fn task_history(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<GetTaskHistoryInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.task_history(request)
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
    async fn list(
        &self,
        request: WorkerEnvelope<ListWorkflowRunsInput>,
    ) -> Result<Vec<a4n_api_contract::workflow::WorkflowRunSummary>, WorkerFailure> {
        validate_request(&request, WORKFLOW_READ_LIST_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        if !(1..=200).contains(&input.limit) {
            return Err(WorkerFailure::invalid_request("limit 必须在 1 到 200 之间"));
        }
        let runs = a4n_db::workflow::list_workflow_runs(
            self.database.connection(),
            a4n_db::workflow::ListWorkflowRunsInput {
                project_id: input.project_id,
                chapter_id: input.chapter_id,
                chapter_ids: None,
                workflow_kind: input.workflow_kind.clone(),
                status: input.status.clone(),
                resumable_only: Some(input.resumable_only),
                limit: Some(input.limit),
            },
        )
        .await
        .map_err(workflow_error)?;
        let visible = if let Some(project_id) = input.project_id {
            verify_project_editor(&self.database, project_id, user_id).await?;
            runs
        } else {
            let project_ids = runs
                .iter()
                .filter_map(|run| run.project_id.or(run.root_project_id))
                .collect::<Vec<_>>();
            let visible_project_ids = a4n_db::collaboration::project_ids_with_permission(
                self.database.connection(),
                &project_ids,
                user_id,
                ProjectAccessRole::Editor,
            )
            .await
            .map_err(collaboration_error)?
            .into_iter()
            .collect::<HashSet<_>>();
            let unscoped_visible = is_active_admin(&self.database, user_id).await?;
            runs.into_iter()
                .filter(|run| match run.project_id.or(run.root_project_id) {
                    Some(project_id) => visible_project_ids.contains(&project_id),
                    None => unscoped_visible,
                })
                .collect()
        };
        Ok(visible
            .into_iter()
            .map(workflow_run_summary_to_api)
            .collect())
    }

    async fn detail(
        &self,
        request: WorkerEnvelope<GetWorkflowRunInput>,
    ) -> Result<a4n_api_contract::workflow::WorkflowRunDetailSummary, WorkerFailure> {
        validate_request(&request, WORKFLOW_READ_DETAIL_OPERATION)?;
        if request.payload.workflow_run_id <= 0 {
            return Err(WorkerFailure::invalid_request("workflow_run_id 必须大于 0"));
        }
        let user_id = user_id(&request)?;
        let detail = a4n_db::workflow::workflow_run_detail(
            self.database.connection(),
            a4n_db::workflow::GetWorkflowRunInput {
                workflow_run_id: request.payload.workflow_run_id,
                include_generation_runs: request.payload.include_generation_runs,
            },
        )
        .await
        .map_err(workflow_error)?;
        match detail.run.project_id.or(detail.run.root_project_id) {
            Some(project_id) => verify_project_editor(&self.database, project_id, user_id).await?,
            None if !is_active_admin(&self.database, user_id).await? => {
                return Err(WorkerFailure::unauthorized("无权查看未绑定项目的 workflow"));
            }
            None => {}
        }
        Ok(workflow_run_detail_to_api(detail))
    }

    async fn task_history(
        &self,
        request: WorkerEnvelope<GetTaskHistoryInput>,
    ) -> Result<Vec<TaskInfoSummary>, WorkerFailure> {
        validate_request(&request, TASK_READ_HISTORY_OPERATION)?;
        if request.principal.kind != WorkerPrincipalKind::Service {
            return Err(WorkerFailure::unauthorized(
                "task history 仅允许 service principal",
            ));
        }
        let input = request.payload;
        if input.project_id.is_some_and(|id| id <= 0) {
            return Err(WorkerFailure::invalid_request("project_id 必须大于 0"));
        }
        if input.limit == 0 {
            return Err(WorkerFailure::invalid_request("limit 必须大于 0"));
        }
        let tasks = a4n_db::workflow::persisted_task_history(
            self.database.connection(),
            input.project_id,
            input.offset,
            input.limit,
        )
        .await
        .map_err(workflow_error)?;
        Ok(tasks
            .into_iter()
            .map(|task| task_info_summary(&task.task))
            .collect())
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
    if request.lease.worker_id != WORKFLOW_READ_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 workflow read worker",
        ));
    }
    Ok(())
}

fn user_id<T: serde::Serialize>(request: &WorkerEnvelope<T>) -> Result<i64, WorkerFailure> {
    if request.principal.kind != WorkerPrincipalKind::User {
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

fn task_info_summary(info: &a4n_queue::priority::TaskInfo) -> TaskInfoSummary {
    TaskInfoSummary {
        id: info.id.clone(),
        task_type: info.task_type.as_str().to_string(),
        priority: info.priority.as_str().to_string(),
        status: info.status.as_str().to_string(),
        root_project_id: info.root_project_id,
        source_snapshot_id: info.source_snapshot_id,
        task_scope: info.task_scope.clone(),
        progress: info.progress.as_ref().map(|progress| {
            a4n_api_contract::task::TaskProgressSummary {
                current: progress.current,
                total: progress.total,
                message: progress.message.clone(),
                meta: progress.meta.clone(),
            }
        }),
        created_at: info.created_at.and_utc().to_rfc3339(),
        started_at: info.started_at.map(|value| value.and_utc().to_rfc3339()),
        completed_at: info.completed_at.map(|value| value.and_utc().to_rfc3339()),
    }
}

async fn verify_project_editor(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(
        database.connection(),
        project_id,
        user_id,
        ProjectAccessRole::Editor,
    )
    .await
    .map(|_| ())
    .map_err(collaboration_error)
}

async fn is_active_admin(database: &DbHandle, user_id: i64) -> Result<bool, WorkerFailure> {
    let user = a4n_db::auth::user_by_id(database.connection(), user_id)
        .await
        .map_err(app_error)?;
    if !user.is_active {
        return Ok(false);
    }
    parse_user_role(&user.role)
        .map(|role| role == UserRole::Admin)
        .map_err(app_error)
}

fn workflow_error(error: a4n_db::workflow::WorkflowReadError) -> WorkerFailure {
    match error {
        a4n_db::workflow::WorkflowReadError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::workflow::WorkflowReadError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_db::workflow::WorkflowReadError::InvalidData(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::workflow::WorkflowReadError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
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

fn app_error(error: a4n_error::AppError) -> WorkerFailure {
    match error {
        a4n_error::AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        a4n_error::AppError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_error::AppError::Forbidden(message) => WorkerFailure::unauthorized(message),
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
