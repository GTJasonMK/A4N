//! PostgreSQL-backed chapter-scene read and mutation worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_api_contract::chapter::ChapterSceneSummary;
use a4n_chapter_scene_worker_contract::{
    CHAPTER_SCENE_CAPABILITY, CHAPTER_SCENE_WORKER_CONTRACT_VERSION, CHAPTER_SCENE_WORKER_ID,
    ChapterInput, DeleteSceneInput, ReorderScenesInput, UpdateSceneInput,
};
use a4n_db::DbHandle;
use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerFailure, WorkerHealth, WorkerHealthStatus,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::Utc;

mod mutations;
mod reads;

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
    ChapterSceneWorker::new(database, CHAPTER_SCENE_WORKER_ID, auth_token)
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
    a4n_worker_runtime::worker_command_response(
        state.authorized(headers),
        request,
        command,
        unauthorized,
    )
    .await
}

impl WorkerState {
    async fn authorized_chapter(
        &self,
        chapter_id: i64,
        user_id: i64,
    ) -> Result<a4n_domain::chapter::ChapterRecord, WorkerFailure> {
        let chapter = a4n_db::project::chapter_by_id(self.database.connection(), chapter_id)
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
            .ok_or_else(|| {
                WorkerFailure::not_found(format!("章节不存在: chapter_id={chapter_id}"))
            })?;
        verify_project_editor(&self.database, chapter.project_id, user_id).await?;
        Ok(chapter)
    }

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
    operation: &str,
) -> Result<(), WorkerFailure> {
    request.validate_worker_command(operation, CHAPTER_SCENE_WORKER_ID, "chapter scene worker")
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
        a4n_domain::collaboration::ProjectAccessRole::Editor,
    )
    .await
    .map(|_| ())
    .map_err(collaboration_error)
}

fn scene_summary(scene: a4n_domain::chapter::ChapterSceneRecord) -> ChapterSceneSummary {
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

fn unauthorized() -> Response {
    StatusCode::UNAUTHORIZED.into_response()
}
