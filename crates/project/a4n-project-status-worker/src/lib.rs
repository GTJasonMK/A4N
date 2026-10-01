//! PostgreSQL-backed project-status read worker.

use std::{env, net::SocketAddr, sync::Arc};

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
use a4n_worker_client::WORKER_AUTH_HEADER;
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
use subtle::ConstantTimeEq;

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

#[derive(Debug)]
struct WorkerFailure {
    code: WorkerErrorCode,
    message: String,
    retryable: bool,
}

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
    let database = DbHandle::connect(database_url).await?;
    ProjectStatusWorker::new(database, PROJECT_STATUS_WORKER_ID, auth_token)
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
    async fn status_machine(
        &self,
        request: WorkerEnvelope<ProjectStatusProjectInput>,
    ) -> Result<ProjectStatusMachineSummary, WorkerFailure> {
        validate_request(&request, PROJECT_STATUS_MACHINE_OPERATION)?;
        let project_id = scoped_project_id(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_viewer(&self.database, project_id, user_id).await?;
        let (_, current_status) = self
            .project_with_status(project_id, "读取项目状态机")
            .await?;
        let (artifacts, history) = tokio::join!(
            async {
                a4n_db::project::project_status_artifacts(&self.database, project_id)
                    .await
                    .map_err(status_read_error)
            },
            async {
                a4n_db::project::project_status_history(&self.database, project_id)
                    .await
                    .map_err(status_read_error)
            },
        );
        let artifacts = artifacts?;
        Ok(ProjectStatusMachineSummary {
            current_status,
            forward_targets: ProjectStatus::ALL
                .into_iter()
                .filter(|target| current_status < *target && artifacts.supports(*target))
                .collect(),
            rollback_targets: ProjectStatus::ALL
                .into_iter()
                .filter(|target| current_status > *target)
                .collect(),
            history: history?,
        })
    }

    async fn preview_rollback(
        &self,
        request: WorkerEnvelope<ProjectStatusRollbackInput>,
    ) -> Result<ProjectRollbackPreview, WorkerFailure> {
        validate_request(&request, PROJECT_STATUS_PREVIEW_ROLLBACK_OPERATION)?;
        scoped_project_id(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let project_id = input.project_id;
        verify_viewer(&self.database, project_id, user_id).await?;
        let (_, current_status) = self
            .project_with_status(project_id, "预览项目状态回退")
            .await?;
        if current_status <= input.target {
            return Err(WorkerFailure::conflict(format!(
                "当前状态 {} 不允许回退到 {}",
                current_status.as_str(),
                input.target.as_str()
            )));
        }
        let actions = rollback_actions(current_status, input.target);
        let impacts =
            a4n_db::project::project_rollback_impacts(&self.database, project_id, &actions)
                .await
                .map_err(rollback_impact_error)?;
        Ok(ProjectRollbackPreview {
            path: ProjectStatus::ALL
                .into_iter()
                .rev()
                .filter(|status| *status <= current_status && *status >= input.target)
                .collect(),
            impacts,
        })
    }

    async fn forward(
        &self,
        request: WorkerEnvelope<ProjectStatusRollbackInput>,
    ) -> Result<ProjectStatusMutationResult, WorkerFailure> {
        validate_request(&request, PROJECT_STATUS_FORWARD_OPERATION)?;
        let project_id = scoped_project_id(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_permission(
            &self.database,
            project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let (project, from) = self.project_with_status(project_id, "推进项目状态").await?;
        let artifacts = a4n_db::project::project_status_artifacts(&self.database, project_id)
            .await
            .map_err(status_read_error)?;
        let target = request.payload.target;
        if from >= target {
            return Err(WorkerFailure::conflict(format!(
                "当前状态 {} 不允许前进到 {}",
                from.as_str(),
                target.as_str()
            )));
        }
        if !artifacts.supports(target) {
            return Err(WorkerFailure::conflict(format!(
                "当前状态 {} 缺少前进到 {} 所需产物",
                from.as_str(),
                target.as_str()
            )));
        }
        let changed_at = chrono::Utc::now().naive_utc();
        let project = a4n_db::project::update_project_status_record(
            &self.database,
            project,
            target,
            changed_at,
        )
        .await
        .map_err(status_write_error)?;
        let summary = self.summary_for_user(user_id, project.id).await?;
        Ok(ProjectStatusMutationResult {
            project: summary,
            from,
            to: target,
            changed_at,
            needs_rag_clear: false,
        })
    }

    async fn rollback(
        &self,
        request: WorkerEnvelope<ProjectStatusRollbackInput>,
    ) -> Result<ProjectStatusMutationResult, WorkerFailure> {
        validate_request(&request, PROJECT_STATUS_ROLLBACK_OPERATION)?;
        let project_id = scoped_project_id(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_permission(
            &self.database,
            project_id,
            user_id,
            ProjectAccessRole::Owner,
        )
        .await?;
        let (_, from) = self.project_with_status(project_id, "回退项目状态").await?;
        let target = request.payload.target;
        if from <= target {
            return Err(WorkerFailure::conflict(format!(
                "当前状态 {} 不允许回退到 {}",
                from.as_str(),
                target.as_str()
            )));
        }
        let changed_at = chrono::Utc::now().naive_utc();
        let outcome = a4n_db::project::reset::rollback_project_status_record(
            &self.database,
            project_id,
            from,
            target,
            changed_at,
        )
        .await
        .map_err(project_reset_error)?;
        let summary = self.summary_for_user(user_id, outcome.project.id).await?;
        Ok(ProjectStatusMutationResult {
            project: summary,
            from,
            to: target,
            changed_at,
            needs_rag_clear: outcome.needs_rag_clear,
        })
    }

    async fn summary_for_user(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<a4n_api_contract::project::ProjectSummary, WorkerFailure> {
        let project = a4n_db::project::project_summary_by_id_record(&self.database, project_id)
            .await
            .map_err(project_mutation_error)?
            .ok_or_else(|| WorkerFailure::not_found(format!("项目不存在: {project_id}")))?;
        a4n_project_read_core::project_summary_for_user(&self.database, user_id, &project)
            .await
            .map_err(summary_error)
    }

    async fn project_with_status(
        &self,
        project_id: i64,
        operation: &'static str,
    ) -> Result<(a4n_domain::project::ProjectRecord, ProjectStatus), WorkerFailure> {
        let project = a4n_db::project::project_by_id_record(&self.database, project_id)
            .await
            .map_err(project_mutation_error)?
            .ok_or_else(|| WorkerFailure::not_found(format!("项目不存在: {project_id}")))?;
        if project.project_kind != a4n_db::project::PROJECT_KIND_TEXT {
            return Err(WorkerFailure::invalid_request(
                "非文本项目不能使用文本写作状态机",
            ));
        }
        let status = ProjectStatus::parse_storage(&project.status).map_err(|_| {
            WorkerFailure::conflict(format!(
                "当前状态 {} 不允许执行 {operation}",
                project.status
            ))
        })?;
        Ok((project, status))
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}

async fn verify_viewer(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(
        &database,
        project_id,
        user_id,
        a4n_domain::collaboration::ProjectAccessRole::Viewer,
    )
    .await
    .map(|_| ())
    .map_err(collaboration_error)
}

async fn verify_permission(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
    required: ProjectAccessRole,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(&database, project_id, user_id, required)
        .await
        .map(|_| ())
        .map_err(collaboration_error)
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
    if request.lease.worker_id != PROJECT_STATUS_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 project status worker",
        ));
    }
    Ok(())
}

fn scoped_project_id<T>(
    request: &WorkerEnvelope<T>,
    project_id: i64,
) -> Result<i64, WorkerFailure> {
    if project_id <= 0 || request.project_id != Some(project_id) {
        return Err(WorkerFailure::invalid_request(
            "worker scope project_id 与 payload 不一致或无效",
        ));
    }
    Ok(project_id)
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

fn project_mutation_error(error: a4n_db::project::ProjectMutationError) -> WorkerFailure {
    match error {
        a4n_db::project::ProjectMutationError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ProjectMutationError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::project::ProjectMutationError::Forbidden(message) => {
            WorkerFailure::unauthorized(message)
        }
        a4n_db::project::ProjectMutationError::Internal(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::project::ProjectMutationError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
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
        a4n_project_read_core::ProjectSummaryError::Progress(error) => match error {
            a4n_db::project::ProjectProgressReadError::InvalidArgument(message) => {
                WorkerFailure::invalid_request(message)
            }
            a4n_db::project::ProjectProgressReadError::Database { source, .. } => {
                WorkerFailure::unavailable(source.to_string())
            }
            error => WorkerFailure::conflict(error.to_string()),
        },
        a4n_project_read_core::ProjectSummaryError::Family(error) => match error {
            a4n_db::project::ProjectFamilyMetadataReadError::InvalidArgument(message) => {
                WorkerFailure::invalid_request(message)
            }
            a4n_db::project::ProjectFamilyMetadataReadError::Database { source, .. } => {
                WorkerFailure::unavailable(source.to_string())
            }
            error => WorkerFailure::conflict(error.to_string()),
        },
        a4n_project_read_core::ProjectSummaryError::Collaboration(error) => {
            collaboration_error(error)
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
