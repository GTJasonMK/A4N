//! PostgreSQL-backed chapter-scene read and mutation worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_api_contract::chapter::ChapterSceneSummary;
use a4n_chapter_scene_worker_contract::{
    CHAPTER_SCENE_CAPABILITY, CHAPTER_SCENE_DELETE_OPERATION, CHAPTER_SCENE_LIST_OPERATION,
    CHAPTER_SCENE_REORDER_OPERATION, CHAPTER_SCENE_UPDATE_OPERATION,
    CHAPTER_SCENE_WORKER_CONTRACT_VERSION, CHAPTER_SCENE_WORKER_ID, ChapterInput, DeleteSceneInput,
    ReorderScenesInput, UpdateSceneInput,
};
use a4n_db::DbHandle;
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
use chrono::Utc;
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_CHAPTER_SCENE_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_CHAPTER_SCENE_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47670";

#[derive(Clone)]
pub struct ChapterSceneWorker {
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

impl ChapterSceneWorker {
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
            .route("/v1/commands/chapter.scene.list", post(list))
            .route("/v1/commands/chapter.scene.update", post(update))
            .route("/v1/commands/chapter.scene.delete", post(delete))
            .route("/v1/commands/chapter.scene.reorder", post(reorder))
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
    ChapterSceneWorker::new(database, CHAPTER_SCENE_WORKER_ID, auth_token)
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
            name: CHAPTER_SCENE_CAPABILITY.into(),
            version: CHAPTER_SCENE_WORKER_CONTRACT_VERSION.into(),
            enabled: true,
        }],
    })
    .into_response()
}

async fn list(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.list(request)).await
}

async fn update(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<UpdateSceneInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.update(request)).await
}

async fn delete(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<DeleteSceneInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.delete(request)).await
}

async fn reorder(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ReorderScenesInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.reorder(request)).await
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
        request: WorkerEnvelope<ChapterInput>,
    ) -> Result<Vec<ChapterSceneSummary>, WorkerFailure> {
        validate_request(&request, CHAPTER_SCENE_LIST_OPERATION)?;
        let chapter = self
            .authorized_chapter(request.payload.chapter_id, user_id(&request)?)
            .await?;
        let scenes = a4n_db::project::chapter_scenes_by_chapter(&self.database, chapter.id)
            .await
            .map_err(record_error)?;
        Ok(scenes.into_iter().map(scene_summary).collect())
    }

    async fn update(
        &self,
        request: WorkerEnvelope<UpdateSceneInput>,
    ) -> Result<ChapterSceneSummary, WorkerFailure> {
        validate_request(&request, CHAPTER_SCENE_UPDATE_OPERATION)?;
        let actor_user_id = user_id(&request)?;
        let input = request.payload;
        let scene = a4n_db::project::chapter_scene_by_id(&self.database, input.scene_id)
            .await
            .map_err(record_error)?
            .ok_or_else(|| {
                WorkerFailure::not_found(format!("场景不存在: scene_id={}", input.scene_id))
            })?;
        self.authorized_chapter(scene.chapter_id, actor_user_id)
            .await?;
        let content = input
            .content
            .map(|value| {
                let value = value.trim().to_string();
                if value.is_empty() {
                    Err(WorkerFailure::invalid_request("场景正文不能为空"))
                } else {
                    Ok(value)
                }
            })
            .transpose()?;
        let scene = a4n_db::project::update_chapter_scene_record(
            &self.database,
            a4n_db::project::ProjectChapterSceneUpdateInput {
                scene,
                content,
                title: input.title.map(|value| Some(trim_optional(value))),
                summary: input.summary.map(|value| Some(trim_optional(value))),
                scene_type: input
                    .scene_type
                    .map(|value| required_text(value, "场景类型"))
                    .transpose()?,
                location: input.location.map(|value| Some(trim_optional(value))),
                timeline_position: input
                    .timeline_position
                    .map(|value| Some(trim_optional(value))),
                updated_at: Utc::now().naive_utc(),
            },
        )
        .await
        .map_err(record_error)?;
        Ok(scene_summary(scene))
    }

    async fn delete(
        &self,
        request: WorkerEnvelope<DeleteSceneInput>,
    ) -> Result<bool, WorkerFailure> {
        validate_request(&request, CHAPTER_SCENE_DELETE_OPERATION)?;
        let scene = a4n_db::project::chapter_scene_by_id(&self.database, request.payload.scene_id)
            .await
            .map_err(record_error)?
            .ok_or_else(|| {
                WorkerFailure::not_found(format!(
                    "场景不存在: scene_id={}",
                    request.payload.scene_id
                ))
            })?;
        self.authorized_chapter(scene.chapter_id, user_id(&request)?)
            .await?;
        let rows = a4n_db::project::delete_chapter_scene_record(&self.database, scene.id)
            .await
            .map_err(record_error)?;
        if rows == 0 {
            return Err(WorkerFailure::not_found(format!(
                "场景不存在: scene_id={}",
                scene.id
            )));
        }
        Ok(true)
    }

    async fn reorder(
        &self,
        request: WorkerEnvelope<ReorderScenesInput>,
    ) -> Result<Vec<ChapterSceneSummary>, WorkerFailure> {
        validate_request(&request, CHAPTER_SCENE_REORDER_OPERATION)?;
        if request.payload.order.is_empty() {
            return Err(WorkerFailure::invalid_request("order 不能为空"));
        }
        let project_id = a4n_db::project::chapter_scene_reorder_project_id(
            &self.database,
            &request.payload.order,
        )
        .await
        .map_err(write_error)?;
        verify_project_editor(&self.database, project_id, user_id(&request)?).await?;
        let scenes =
            a4n_db::project::reorder_chapter_scene_records(&self.database, &request.payload.order)
                .await
                .map_err(write_error)?;
        Ok(scenes.into_iter().map(scene_summary).collect())
    }

    async fn authorized_chapter(
        &self,
        chapter_id: i64,
        user_id: i64,
    ) -> Result<a4n_db_models::models::chapter::chapter::Model, WorkerFailure> {
        let chapter = a4n_db::project::chapter_by_id(&self.database, chapter_id)
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
            .ok_or_else(|| {
                WorkerFailure::not_found(format!("章节不存在: chapter_id={chapter_id}"))
            })?;
        verify_project_editor(&self.database, chapter.project_id, user_id).await?;
        Ok(chapter)
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
    if request.lease.worker_id != CHAPTER_SCENE_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 chapter scene worker",
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

async fn verify_project_editor(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(
        &database,
        project_id,
        user_id,
        a4n_domain::collaboration::ProjectAccessRole::Editor,
    )
    .await
    .map(|_| ())
    .map_err(collaboration_error)
}

fn scene_summary(
    scene: a4n_db_models::models::chapter::chapter_scene::Model,
) -> ChapterSceneSummary {
    ChapterSceneSummary {
        id: scene.id,
        chapter_id: scene.chapter_id,
        scene_number: scene.scene_number,
        title: scene.title,
        content: scene.content,
        word_count: scene.word_count,
        summary: scene.summary,
        scene_type: scene.scene_type,
        characters_present: scene.characters_present,
        location: scene.location,
        timeline_position: scene.timeline_position,
        transition_from_previous: scene.transition_from_previous,
        created_at: scene.created_at.to_string(),
        updated_at: scene.updated_at.to_string(),
    }
}

fn trim_optional(value: String) -> String {
    value.trim().to_string()
}

fn required_text(value: String, field: &str) -> Result<String, WorkerFailure> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(WorkerFailure::invalid_request(format!("{field}不能为空")))
    } else {
        Ok(value)
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

fn record_error(error: a4n_db::project::ProjectChapterSceneRecordError) -> WorkerFailure {
    match error {
        a4n_db::project::ProjectChapterSceneRecordError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ProjectChapterSceneRecordError::Database { source, .. } => {
            WorkerFailure::unavailable(source.to_string())
        }
    }
}

fn write_error(error: a4n_db::project::ProjectChapterSceneWriteError) -> WorkerFailure {
    match error {
        a4n_db::project::ProjectChapterSceneWriteError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ProjectChapterSceneWriteError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::project::ProjectChapterSceneWriteError::Database(error) => {
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
