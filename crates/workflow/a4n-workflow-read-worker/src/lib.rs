//! PostgreSQL-backed workflow read worker.

use std::{collections::HashSet, env, net::SocketAddr, sync::Arc};

use a4n_api_contract::task::{GetTaskHistoryInput, TaskInfoSummary, TaskProgressSummary};
use a4n_api_contract::workflow::{
    ActiveTaskLookupRecord, GetWorkflowRunInput, ListWorkflowRunsInput, TaskLookupRecord,
    WorkflowProgressEvent, WorkflowProgressEventsInput, WorkflowResumeRecord, WorkflowRunSummary,
    WorkflowTaskAccessRecord,
};
use a4n_db::DbHandle;
use a4n_db_models::models::user::role::{UserRole, parse_user_role};
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerHealth,
    WorkerHealthStatus, WorkerOperationRef, WorkerOperationStatus, WorkerPrincipal,
    WorkerPrincipalKind, WorkerResponse,
};
use a4n_workflow_core::{workflow_run_detail_to_api, workflow_run_summary_to_api};
use a4n_workflow_read_worker_contract::{
    EmptyInput, TASK_LOOKUP_ACTIVE_OPERATION, TASK_LOOKUP_BY_ID_OPERATION,
    TASK_READ_HISTORY_CAPABILITY, TASK_READ_HISTORY_OPERATION, TaskIdInput,
    WORKFLOW_ACCESS_ACTIVE_TASK_OPERATION, WORKFLOW_READ_CAPABILITY,
    WORKFLOW_READ_DETAIL_OPERATION, WORKFLOW_READ_LIST_OPERATION,
    WORKFLOW_READ_PROGRESS_EVENTS_OPERATION, WORKFLOW_READ_RESUME_RECORD_OPERATION,
    WORKFLOW_READ_SUMMARY_OPERATION, WORKFLOW_READ_WORKER_CONTRACT_VERSION,
    WORKFLOW_READ_WORKER_ID, WORKFLOW_TASK_LOOKUP_CAPABILITY, WorkflowRunIdInput,
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
            .route("/v1/commands/workflow.read.summary", post(summary))
            .route(
                "/v1/commands/workflow.read.resume_record",
                post(resume_record),
            )
            .route(
                "/v1/commands/workflow.read.progress_events",
                post(progress_events),
            )
            .route("/v1/commands/task.read.history", post(task_history))
            .route("/v1/commands/task.lookup.by_id", post(task_by_id))
            .route("/v1/commands/task.lookup.active", post(active_tasks))
            .route(
                "/v1/commands/workflow.access.active_task",
                post(workflow_access_by_active_task),
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
        capabilities: [
            WORKFLOW_READ_CAPABILITY,
            WORKFLOW_TASK_LOOKUP_CAPABILITY,
            TASK_READ_HISTORY_CAPABILITY,
        ]
        .into_iter()
        .map(|name| WorkerCapability {
            name: name.into(),
            version: WORKFLOW_READ_WORKER_CONTRACT_VERSION.into(),
            enabled: true,
        })
        .collect(),
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

async fn task_by_id(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<TaskIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.task_by_id(request)
    })
    .await
}

async fn active_tasks(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<EmptyInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.active_tasks(request)
    })
    .await
}

async fn workflow_access_by_active_task(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<TaskIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.workflow_access_by_active_task(request)
    })
    .await
}

async fn resume_record(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<WorkflowRunIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.resume_record(request)
    })
    .await
}

async fn summary(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<WorkflowRunIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.summary(request)).await
}

async fn progress_events(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<WorkflowProgressEventsInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.progress_events(request)
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
        a4n_domain::workflow::validate_optional_project_id(input.project_id)
            .map_err(WorkerFailure::invalid_request)?;
        if let Some(project_id) = input.project_id {
            verify_project_editor(&self.database, project_id, user_id).await?;
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
        let visible = if input.project_id.is_some() {
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
        require_service_principal(&request.principal)?;
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

    async fn task_by_id(
        &self,
        request: WorkerEnvelope<TaskIdInput>,
    ) -> Result<Option<TaskLookupRecord>, WorkerFailure> {
        validate_request(&request, TASK_LOOKUP_BY_ID_OPERATION)?;
        require_service_principal(&request.principal)?;
        let persisted = a4n_db::workflow::persisted_task_by_id(
            self.database.connection(),
            &request.payload.task_id,
        )
        .await
        .map_err(workflow_error)?;
        Ok(persisted.map(|persisted| TaskLookupRecord {
            task_id: persisted.task.id,
            project_id: persisted.task.project_id,
            root_project_id: persisted.task.root_project_id,
            workflow_run_id: persisted.workflow_run_id,
            status: persisted.task.status.as_str().to_string(),
        }))
    }

    async fn active_tasks(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<Vec<ActiveTaskLookupRecord>, WorkerFailure> {
        validate_request(&request, TASK_LOOKUP_ACTIVE_OPERATION)?;
        require_service_principal(&request.principal)?;
        let persisted =
            a4n_db::workflow::persisted_task_history(self.database.connection(), None, 0, 100)
                .await
                .map_err(workflow_error)?;
        Ok(persisted
            .into_iter()
            .map(active_task_lookup_record)
            .collect())
    }

    async fn workflow_access_by_active_task(
        &self,
        request: WorkerEnvelope<TaskIdInput>,
    ) -> Result<Option<WorkflowTaskAccessRecord>, WorkerFailure> {
        validate_request(&request, WORKFLOW_ACCESS_ACTIVE_TASK_OPERATION)?;
        require_service_principal(&request.principal)?;
        Ok(
            a4n_db::workflow::workflow_run_access_record_by_active_task_id(
                self.database.connection(),
                &request.payload.task_id,
            )
            .await
            .map_err(workflow_error)?
            .map(|record| WorkflowTaskAccessRecord {
                workflow_run_id: record.id,
                project_id: record.project_id,
                root_project_id: record.root_project_id,
            }),
        )
    }

    async fn resume_record(
        &self,
        request: WorkerEnvelope<WorkflowRunIdInput>,
    ) -> Result<WorkflowResumeRecord, WorkerFailure> {
        validate_request(&request, WORKFLOW_READ_RESUME_RECORD_OPERATION)?;
        require_service_principal(&request.principal)?;
        let run = a4n_db::workflow::workflow_run_model_by_id(
            self.database.connection(),
            request.payload.workflow_run_id,
        )
        .await
        .map_err(workflow_error)?;
        Ok(WorkflowResumeRecord {
            id: run.id,
            workflow_kind: run.workflow_kind,
            project_id: run.project_id,
            root_project_id: run.root_project_id,
            chapter_id: run.chapter_id,
            scope_table: run.scope_table,
            scope_id: run.scope_id,
            input_fingerprint_json: run.input_fingerprint_json,
            status: run.status,
            can_resume: run.can_resume,
            resume_block_reason: run.resume_block_reason,
        })
    }

    async fn summary(
        &self,
        request: WorkerEnvelope<WorkflowRunIdInput>,
    ) -> Result<WorkflowRunSummary, WorkerFailure> {
        validate_request(&request, WORKFLOW_READ_SUMMARY_OPERATION)?;
        require_service_principal(&request.principal)?;
        a4n_db::workflow::workflow_run_summary_by_id(
            self.database.connection(),
            request.payload.workflow_run_id,
        )
        .await
        .map(workflow_run_summary_to_api)
        .map_err(workflow_error)
    }

    async fn progress_events(
        &self,
        request: WorkerEnvelope<WorkflowProgressEventsInput>,
    ) -> Result<Vec<WorkflowProgressEvent>, WorkerFailure> {
        validate_request(&request, WORKFLOW_READ_PROGRESS_EVENTS_OPERATION)?;
        require_service_principal(&request.principal)?;
        let Some(workflow_run_id) = request.payload.workflow_run_id else {
            return Err(WorkerFailure::invalid_request("workflow_run_id 必须存在"));
        };
        if request.payload.task_id.is_some() {
            return Err(WorkerFailure::invalid_request(
                "progress events 不接受 task_id",
            ));
        }
        a4n_db::workflow::workflow_progress_events(
            self.database.connection(),
            workflow_run_id,
            request.payload.limit,
        )
        .await
        .map(|events| {
            events
                .into_iter()
                .map(|event| WorkflowProgressEvent {
                    id: event.id,
                    current: event.current,
                    total: event.total,
                    message: event.message,
                    meta: event.meta,
                    timestamp: event.timestamp,
                })
                .collect()
        })
        .map_err(workflow_error)
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}

fn require_service_principal(principal: &WorkerPrincipal) -> Result<(), WorkerFailure> {
    if principal.kind != WorkerPrincipalKind::Service {
        return Err(WorkerFailure::unauthorized(
            "workflow task lookup 仅允许 service principal",
        ));
    }
    Ok(())
}

fn active_task_lookup_record(
    persisted: a4n_db::workflow::PersistedTaskInfo,
) -> ActiveTaskLookupRecord {
    let task = persisted.task;
    let progress = task.progress.as_ref().map(|progress| TaskProgressSummary {
        current: progress.current,
        total: progress.total,
        message: progress.message.clone(),
        meta: progress.meta.clone(),
    });
    let progress_events = task
        .progress
        .as_ref()
        .map(|progress| WorkflowProgressEvent {
            id: format!("{}-snapshot", task.id),
            current: progress.current,
            total: progress.total,
            message: progress.message.clone(),
            meta: progress.meta.clone(),
            timestamp: progress
                .updated_at
                .clone()
                .unwrap_or_else(|| task.started_at.unwrap_or(task.created_at).to_string()),
        })
        .into_iter()
        .collect();
    ActiveTaskLookupRecord {
        task_id: task.id.clone(),
        project_id: task.project_id,
        root_project_id: task.root_project_id,
        status: task.status.as_str().to_string(),
        workflow: a4n_api_contract::workflow::WorkflowStateSummary {
            task_id: task.id,
            root_project_id: task.root_project_id,
            source_snapshot_id: task.source_snapshot_id,
            task_scope: task.task_scope,
            progress,
            progress_events,
        },
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

#[cfg(test)]
mod tests {
    use a4n_progress::{ProgressMeta, ProgressPhaseKind};
    use a4n_queue::priority::{TaskInfo, TaskKind, TaskPriority, TaskProgress, TaskStatus};
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn task_lookup_projection_preserves_progress_and_snapshot_timing() {
        let created_at = NaiveDate::from_ymd_opt(2026, 9, 10)
            .unwrap()
            .and_hms_opt(1, 2, 3)
            .unwrap();
        let started_at = NaiveDate::from_ymd_opt(2026, 9, 10)
            .unwrap()
            .and_hms_opt(1, 2, 4)
            .unwrap();
        let task = TaskInfo {
            id: "task-1".into(),
            task_type: TaskKind::MangaGeneration,
            priority: TaskPriority::Medium,
            status: TaskStatus::Running,
            project_id: Some(11),
            root_project_id: Some(7),
            source_snapshot_id: Some(13),
            task_scope: Some("chapter".into()),
            progress: Some(TaskProgress {
                current: 2,
                total: 4,
                message: "drawing".into(),
                meta: Some(ProgressMeta::new(
                    "manga_generation",
                    "draw",
                    "draw",
                    ProgressPhaseKind::AssetGeneration,
                )),
                updated_at: None,
            }),
            created_at,
            started_at: Some(started_at),
            completed_at: None,
        };

        let summary = task_info_summary(&task);
        assert_eq!(summary.status, "running");
        assert_eq!(summary.root_project_id, Some(7));
        assert_eq!(summary.progress.unwrap().current, 2);

        let active = active_task_lookup_record(a4n_db::workflow::PersistedTaskInfo {
            task,
            workflow_run_id: Some(19),
        });
        assert_eq!(active.workflow.task_id, "task-1");
        assert_eq!(active.workflow.progress_events[0].id, "task-1-snapshot");
        assert_eq!(
            active.workflow.progress_events[0].timestamp,
            started_at.to_string()
        );
    }

    #[test]
    fn task_lookup_rejects_user_principals() {
        let user = WorkerPrincipal {
            kind: WorkerPrincipalKind::User,
            subject: "7".into(),
        };
        let service = WorkerPrincipal {
            kind: WorkerPrincipalKind::Service,
            subject: "runtime".into(),
        };

        assert!(require_service_principal(&user).is_err());
        assert!(require_service_principal(&service).is_ok());
    }
}
