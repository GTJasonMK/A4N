//! PostgreSQL-backed generated-video record read worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_api_contract::image::GeneratedImageSummary;
use a4n_api_contract::video::GeneratedVideoSummary;
use a4n_db::DbHandle;
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_video_record_worker_contract::{
    ChapterVideosInput, LoadVideoSegmentPlanDraftInput, ReferenceImagesInput,
    VIDEO_PROMPT_CAPABILITY, VIDEO_PROMPT_DRAFT_CAPABILITY, VIDEO_RECORD_CAPABILITY,
    VIDEO_RECORD_LIST_OPERATION, VIDEO_RECORD_MUTATION_CAPABILITY, VIDEO_RECORD_PLAYBACK_OPERATION,
    VIDEO_RECORD_REFERENCE_IMAGES_OPERATION, VIDEO_RECORD_SELECT_SEGMENT_OPERATION,
    VIDEO_RECORD_UPDATE_OPERATION, VIDEO_RECORD_WORKER_CONTRACT_VERSION, VIDEO_RECORD_WORKER_ID,
    VideoIdInput, VideoSegmentSelectionInput, VideoSourceInput, VideoUpdateInput,
};
use a4n_worker_contract::WORKER_AUTH_HEADER;
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
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Statement};
use subtle::ConstantTimeEq;

mod mutations;
mod prompts;
mod reads;
mod summaries;

use summaries::{image_summary, video_summary};

const WORKER_ADDR_ENV: &str = "A4N_VIDEO_RECORD_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_VIDEO_RECORD_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47675";

#[derive(Clone)]
pub struct VideoRecordWorker {
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

impl VideoRecordWorker {
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
            .route("/v1/commands/video.record.playback", post(playback_source))
            .route(
                "/v1/commands/video.record.list_by_chapter",
                post(list_by_chapter),
            )
            .route(
                "/v1/commands/video.record.reference_images",
                post(validated_reference_images),
            )
            .route("/v1/commands/video.record.update", post(update_record))
            .route(
                "/v1/commands/video.record.select_segment",
                post(select_segment),
            )
            .route(
                "/v1/commands/video.prompt.draft.load",
                post(load_video_segment_plan_draft),
            )
            .route(
                "/v1/commands/video.prompt.draft.update",
                post(update_video_segment_plan_draft_recipe),
            )
            .route(
                "/v1/commands/video.prompt.list_by_chapter",
                post(list_prompts),
            )
            .route(
                "/v1/commands/video.prompt.validate_source",
                post(validate_source),
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
    VideoRecordWorker::new(database, VIDEO_RECORD_WORKER_ID, auth_token)
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
                name: VIDEO_RECORD_CAPABILITY.into(),
                version: VIDEO_RECORD_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: VIDEO_PROMPT_CAPABILITY.into(),
                version: VIDEO_RECORD_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: VIDEO_RECORD_MUTATION_CAPABILITY.into(),
                version: VIDEO_RECORD_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: VIDEO_PROMPT_DRAFT_CAPABILITY.into(),
                version: VIDEO_RECORD_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

async fn playback_source(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<VideoIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.playback_source(request)
    })
    .await
}

async fn list_by_chapter(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterVideosInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.list_by_chapter(request)
    })
    .await
}

async fn validated_reference_images(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ReferenceImagesInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.validated_reference_images(request)
    })
    .await
}

async fn update_record(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<VideoUpdateInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.update_record(request)
    })
    .await
}

async fn select_segment(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<VideoSegmentSelectionInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.select_segment(request)
    })
    .await
}

async fn load_video_segment_plan_draft(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<LoadVideoSegmentPlanDraftInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.load_video_segment_plan_draft(request)
    })
    .await
}

async fn update_video_segment_plan_draft_recipe(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<
        WorkerEnvelope<a4n_video_record_worker_contract::UpdateVideoSegmentPlanDraftRecipeInput>,
    >,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.update_video_segment_plan_draft_recipe(request)
    })
    .await
}

async fn list_prompts(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterVideosInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.list_prompts(request)
    })
    .await
}

async fn validate_source(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<VideoSourceInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.validate_source(request)
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
    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}

async fn verify_project_permission(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
    role: ProjectAccessRole,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(
        database.connection(),
        project_id,
        user_id,
        role,
    )
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
    if request.lease.worker_id != VIDEO_RECORD_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 video record worker",
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

fn validate_required_project_scope<T>(
    request: &WorkerEnvelope<T>,
    project_id: i64,
) -> Result<(), WorkerFailure> {
    if project_id <= 0 || request.project_id != Some(project_id) {
        return Err(WorkerFailure::invalid_request(
            "worker scope project_id 与资源 project_id 不一致或无效",
        ));
    }
    Ok(())
}

fn validate_optional_project_scope(
    request_project_id: Option<i64>,
    project_id: i64,
) -> Result<(), WorkerFailure> {
    if project_id <= 0 || request_project_id.is_some_and(|scope| scope != project_id) {
        return Err(WorkerFailure::invalid_request(
            "worker scope project_id 与资源 project_id 不一致或无效",
        ));
    }
    Ok(())
}

fn positive(value: i64, field: &str) -> Result<i64, WorkerFailure> {
    if value > 0 {
        Ok(value)
    } else {
        Err(WorkerFailure::invalid_request(format!(
            "{field} 必须大于 0"
        )))
    }
}

async fn validate_video_source(
    database: &DbHandle,
    project_id: i64,
    chapter_id: i64,
    operation: &str,
    writable: bool,
) -> Result<String, WorkerFailure> {
    let (project, chapter) = a4n_db::video::project_and_chapter_for_video_source(
        database.connection(),
        project_id,
        chapter_id,
    )
    .await
    .map_err(video_error)?;
    match project.project_kind.as_str() {
        "text" => {
            if chapter.project_id != project.id {
                return Err(WorkerFailure::invalid_request(format!(
                    "{operation}失败：章节不属于文本项目 {project_id}"
                )));
            }
            if chapter.status == "deleted" || (writable && chapter.status == "generating") {
                return Err(WorkerFailure::conflict(format!(
                    "{}: {operation}",
                    chapter.status
                )));
            }
            Ok(chapter.content)
        }
        "video" => {
            if project.root_project_id != Some(chapter.project_id) {
                return Err(WorkerFailure::invalid_request(format!(
                    "{operation}失败：章节不属于视频项目 {project_id} 绑定的根文本项目"
                )));
            }
            let snapshot_id = project.source_snapshot_id.ok_or_else(|| {
                WorkerFailure::conflict(format!("missing_source_snapshot: {operation}"))
            })?;
            let snapshot_chapter = a4n_db::video::source_snapshot_chapter_for_video_source(
                database.connection(),
                snapshot_id,
                chapter_id,
            )
            .await
            .map_err(video_error)?
            .ok_or_else(|| {
                WorkerFailure::not_found(format!(
                    "{operation}失败：视频项目 {project_id} 的源快照不包含章节 {chapter_id}"
                ))
            })?;
            Ok(snapshot_chapter.content)
        }
        other => {
            return Err(WorkerFailure::invalid_request(format!(
                "{operation}失败：视频链路不支持项目类型 {other}: project_id={project_id}"
            )));
        }
    }
}

fn database_error(error: sea_orm::DbErr) -> WorkerFailure {
    WorkerFailure::unavailable(error.to_string())
}

fn worker_app_error(error: a4n_error::AppError) -> WorkerFailure {
    match error {
        a4n_error::AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        a4n_error::AppError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}: {operation}"))
        }
        a4n_error::AppError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_error::AppError::Forbidden(message) => WorkerFailure::unauthorized(message),
        other => WorkerFailure::unavailable(other.to_string()),
    }
}

#[cfg(test)]
mod tests;

fn workflow_error(error: a4n_db::workflow::WorkflowReadError) -> WorkerFailure {
    match error {
        a4n_db::workflow::WorkflowReadError::InvalidArgument(message)
        | a4n_db::workflow::WorkflowReadError::InvalidData(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::workflow::WorkflowReadError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_db::workflow::WorkflowReadError::Database(error) => database_error(error),
    }
}

fn video_error(error: a4n_db::video::VideoDbError) -> WorkerFailure {
    match error {
        a4n_db::video::VideoDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::video::VideoDbError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_db::video::VideoDbError::Forbidden(message) => WorkerFailure::unauthorized(message),
        a4n_db::video::VideoDbError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}: {operation}"))
        }
        a4n_db::video::VideoDbError::Database(error) => database_error(error),
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
        a4n_db::collaboration::CollaborationDbError::Database(error) => database_error(error),
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
