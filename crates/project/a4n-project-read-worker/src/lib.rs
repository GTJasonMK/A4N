//! PostgreSQL-backed project summary/list read worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_db::DbHandle;
use a4n_project_read_core::project_summary_from_parts;
use a4n_project_read_worker_contract::{
    PROJECT_READ_CAPABILITY, PROJECT_READ_EXISTS_OPERATION, PROJECT_READ_IDS_OPERATION,
    PROJECT_READ_LIST_OPERATION, PROJECT_READ_SUMMARY_OPERATION,
    PROJECT_READ_WORKER_CONTRACT_VERSION, PROJECT_READ_WORKER_ID, PaginatedProjects,
    ProjectIdInput, ProjectListInput, ProjectServiceInput, ProjectSummary,
};
use a4n_worker_client::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerHealth,
    WorkerHealthStatus, WorkerOperationRef, WorkerOperationStatus, WorkerPrincipalKind,
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

#[derive(Debug)]
struct WorkerFailure {
    code: WorkerErrorCode,
    message: String,
    retryable: bool,
}

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
    ProjectReadWorker::new(database, PROJECT_READ_WORKER_ID, auth_token)
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
    async fn summary(
        &self,
        request: WorkerEnvelope<ProjectIdInput>,
    ) -> Result<ProjectSummary, WorkerFailure> {
        validate_request(&request, PROJECT_READ_SUMMARY_OPERATION, false)?;
        let user_id = user_id(&request)?;
        let project_id = scoped_project_id(&request, request.payload.project_id)?;
        let project = a4n_db::project::project_summary_by_id_record(&self.database, project_id)
            .await
            .map_err(project_error)?
            .ok_or_else(|| WorkerFailure::not_found(format!("项目不存在: {project_id}")))?;
        self.summary_for_project(user_id, project).await
    }

    async fn list(
        &self,
        request: WorkerEnvelope<ProjectListInput>,
    ) -> Result<PaginatedProjects, WorkerFailure> {
        validate_request(&request, PROJECT_READ_LIST_OPERATION, false)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let offset = a4n_domain::project::project_list_offset(input.page, input.page_size)
            .map_err(WorkerFailure::invalid_request)?;
        let page = a4n_db::collaboration::list_accessible_projects(
            &self.database,
            a4n_db::collaboration::AccessibleProjectListInput {
                user_id,
                offset,
                limit: input.page_size,
                project_kind: input.project_kind,
                root_project_id: input.root_project_id,
                access_scope: input
                    .access_scope
                    .unwrap_or(a4n_db::collaboration::ProjectAccessScope::All),
            },
        )
        .await
        .map_err(collaboration_error)?;
        let projects = page
            .items
            .into_iter()
            .map(|project| project.into_summary_record())
            .collect::<Vec<_>>();
        let items = self.summary_for_projects(user_id, &projects).await?;
        Ok(PaginatedProjects {
            items,
            total: page.total,
            page: input.page,
            page_size: input.page_size,
        })
    }

    async fn ids(
        &self,
        request: WorkerEnvelope<ProjectServiceInput>,
    ) -> Result<Vec<i64>, WorkerFailure> {
        validate_request(&request, PROJECT_READ_IDS_OPERATION, true)?;
        service_principal(&request)?;
        a4n_db::project::project_ids(&self.database)
            .await
            .map_err(project_error)
    }

    async fn exists(&self, request: WorkerEnvelope<ProjectIdInput>) -> Result<bool, WorkerFailure> {
        validate_request(&request, PROJECT_READ_EXISTS_OPERATION, true)?;
        service_principal(&request)?;
        let project_id = scoped_project_id(&request, request.payload.project_id)?;
        a4n_db::project::project_by_id_record(&self.database, project_id)
            .await
            .map(|project| project.is_some())
            .map_err(project_error)
    }

    async fn summary_for_project(
        &self,
        user_id: i64,
        project: a4n_domain::project::ProjectSummaryRecord,
    ) -> Result<ProjectSummary, WorkerFailure> {
        a4n_project_read_core::project_summary_for_user(&self.database, user_id, &project)
            .await
            .map_err(summary_error)
    }

    async fn summary_for_projects(
        &self,
        user_id: i64,
        projects: &[a4n_domain::project::ProjectSummaryRecord],
    ) -> Result<Vec<ProjectSummary>, WorkerFailure> {
        let family_queries = projects
            .iter()
            .map(a4n_db::project::ProjectFamilyMetadataQuery::from_project)
            .collect::<Vec<_>>();
        let access_queries = projects
            .iter()
            .map(a4n_db::collaboration::ProjectAccessInfoQuery::from_project)
            .collect::<Vec<_>>();
        let progress_queries = projects
            .iter()
            .map(a4n_db::project::ProjectProgressQuery::from_project)
            .collect::<Result<Vec<_>, _>>()
            .map_err(progress_error)?;
        let (metadata_by_id, access_by_id, progress_by_id, intervals) = tokio::try_join!(
            async {
                a4n_db::project::project_family_metadata_batch(&self.database, &family_queries)
                    .await
                    .map_err(family_error)
            },
            async {
                a4n_db::collaboration::project_access_info_batch(
                    &self.database,
                    &access_queries,
                    user_id,
                )
                .await
                .map_err(collaboration_error)
            },
            async {
                a4n_db::project::project_progress_summaries(&self.database, &progress_queries)
                    .await
                    .map_err(progress_error)
            },
            async {
                a4n_db::fact_feedback::feedback_intervals(
                    &self.database,
                    &projects
                        .iter()
                        .map(|project| project.id)
                        .collect::<Vec<_>>(),
                )
                .await
                .map_err(|error| WorkerFailure::unavailable(error.to_string()))
            },
        )?;
        projects
            .iter()
            .zip(&progress_queries)
            .map(|(project, query)| {
                let metadata = metadata_by_id.get(&project.id).ok_or_else(|| {
                    WorkerFailure::conflict(format!("项目缺少族元数据: {}", project.id))
                })?;
                let access = access_by_id.get(&project.id).ok_or_else(|| {
                    WorkerFailure::conflict(format!("项目缺少协作访问信息: {}", project.id))
                })?;
                let progress = progress_by_id.get(&project.id).cloned().ok_or_else(|| {
                    WorkerFailure::conflict(format!("项目缺少进度摘要: {}", project.id))
                })?;
                Ok(project_summary_from_parts(
                    project,
                    metadata,
                    access,
                    query.project_kind,
                    query.source_mode,
                    query.status,
                    progress,
                    intervals
                        .get(&project.id)
                        .copied()
                        .unwrap_or(a4n_domain::fact_feedback::DEFAULT_FEEDBACK_INTERVAL_CHAPTERS),
                ))
            })
            .collect()
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
    service: bool,
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
    if request.lease.worker_id != PROJECT_READ_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 project read worker",
        ));
    }
    if service && request.principal.kind != WorkerPrincipalKind::Service {
        return Err(WorkerFailure::unauthorized(
            "该操作仅允许 service principal",
        ));
    }
    if !service && request.principal.kind != WorkerPrincipalKind::User {
        return Err(WorkerFailure::unauthorized("该操作仅允许 user principal"));
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
    request
        .principal
        .subject
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| WorkerFailure::unauthorized("用户 principal subject 无效"))
}

fn service_principal<T>(request: &WorkerEnvelope<T>) -> Result<(), WorkerFailure> {
    if request.principal.subject != "runtime" {
        return Err(WorkerFailure::unauthorized(
            "service principal subject 无效",
        ));
    }
    Ok(())
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

fn summary_error(error: a4n_project_read_core::ProjectSummaryError) -> WorkerFailure {
    match error {
        a4n_project_read_core::ProjectSummaryError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
        a4n_project_read_core::ProjectSummaryError::Progress(error) => progress_error(error),
        a4n_project_read_core::ProjectSummaryError::Family(error) => family_error(error),
        a4n_project_read_core::ProjectSummaryError::Collaboration(error) => {
            collaboration_error(error)
        }
    }
}

fn family_error(error: a4n_db::project::ProjectFamilyMetadataReadError) -> WorkerFailure {
    match error {
        a4n_db::project::ProjectFamilyMetadataReadError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ProjectFamilyMetadataReadError::RootProjectMissing { .. }
        | a4n_db::project::ProjectFamilyMetadataReadError::SourceSnapshotMissing { .. } => {
            WorkerFailure::conflict(error.to_string())
        }
        a4n_db::project::ProjectFamilyMetadataReadError::Database { source, .. } => {
            WorkerFailure::unavailable(source.to_string())
        }
    }
}

fn progress_error(error: a4n_db::project::ProjectProgressReadError) -> WorkerFailure {
    match error {
        a4n_db::project::ProjectProgressReadError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ProjectProgressReadError::Database { source, .. } => {
            WorkerFailure::unavailable(source.to_string())
        }
        error => WorkerFailure::conflict(error.to_string()),
    }
}

fn project_error(error: a4n_db::project::ProjectMutationError) -> WorkerFailure {
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

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .try_init();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
