//! PostgreSQL-backed workflow read worker.

use std::{collections::HashSet, net::SocketAddr, sync::Arc};

use a4n_api_contract::task::{GetTaskHistoryInput, TaskInfoSummary};
use a4n_api_contract::workflow::{
    GetWorkflowRunInput, ListWorkflowRunsInput, TaskLookupRecord, WorkflowProgressEventsInput,
    WorkflowResumeRecord, WorkflowRunSummary, WorkflowTaskAccessRecord,
};
use a4n_db::DbHandle;
use a4n_domain::auth::{UserRole, parse_user_role};
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerFailure, WorkerHealth, WorkerHealthStatus,
    WorkerPrincipal, WorkerPrincipalKind,
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

mod reads;

impl WorkflowReadWorker {
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
    let database = DbHandle::connect_default(database_url).await?;
    WorkflowReadWorker::new(database, WORKFLOW_READ_WORKER_ID, auth_token)
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
    a4n_worker_runtime::worker_command_response(
        state.authorized(headers),
        request,
        command,
        unauthorized,
    )
    .await
}

fn require_service_principal(principal: &WorkerPrincipal) -> Result<(), WorkerFailure> {
    if principal.kind != WorkerPrincipalKind::Service {
        return Err(WorkerFailure::unauthorized(
            "workflow task lookup 仅允许 service principal",
        ));
    }
    Ok(())
}

fn validate_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    request.validate_worker_command(operation, WORKFLOW_READ_WORKER_ID, "workflow read worker")
}

async fn verify_project_editor(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(
        database.executor(),
        project_id,
        user_id,
        ProjectAccessRole::Editor,
    )
    .await
    .map(|_| ())
    .map_err(collaboration_error)
}

async fn is_active_admin(database: &DbHandle, user_id: i64) -> Result<bool, WorkerFailure> {
    let user = a4n_db::auth::user_summary_by_id(database.executor(), user_id)
        .await
        .map_err(app_error)?;
    if !user.is_active {
        return Ok(false);
    }
    parse_user_role(&user.role)
        .map(|role| role == UserRole::Admin)
        .map_err(|error| app_error(error.into()))
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
    a4n_worker_db_adapter::map_collaboration_db_error(error)
}

fn app_error(error: a4n_error::AppError) -> WorkerFailure {
    match error {
        a4n_error::AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        a4n_error::AppError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_error::AppError::Forbidden(message) => WorkerFailure::unauthorized(message),
        other => WorkerFailure::unavailable(other.to_string()),
    }
}

fn unauthorized() -> Response {
    StatusCode::UNAUTHORIZED.into_response()
}

#[cfg(test)]
mod tests {
    use a4n_progress::{ProgressMeta, ProgressPhaseKind};
    use a4n_queue::priority::{TaskInfo, TaskKind, TaskPriority, TaskProgress, TaskStatus};
    use chrono::NaiveDate;

    use super::reads::{active_task_lookup_record, task_info_summary};
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
