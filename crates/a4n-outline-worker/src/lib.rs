//! PostgreSQL-backed persisted outline records worker.

use std::{
    collections::{HashMap, HashSet},
    env,
    net::SocketAddr,
    sync::Arc,
};

use a4n_api_contract::outline::{
    ChapterOutlineSummary, DeleteLatestOutlineRecordsInput, OutlineGenerationProgressSummary,
    OutlineMutationResult, PartOutlineSummary, SaveChapterOutlineInput, SaveChapterOutlineResult,
    SavePartOutlineInput, SavePartOutlineResult, UpdateChapterOutlineInput,
    UpdateChapterOutlineResult,
};
use a4n_db::{DbHandle, project::OutlineGenerationProgressRows};
use a4n_db_models::models::chapter::{chapter_outline, part_outline};
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_outline_worker_contract::{
    ChapterOutlineAccessInput, DELETE_CHAPTERS_OPERATION, DELETE_PARTS_OPERATION,
    ENSURE_CHAPTER_ACCESS_OPERATION, ENSURE_PART_ACCESS_OPERATION, GENERATION_PROGRESS_OPERATION,
    LIST_CHAPTERS_OPERATION, LIST_PARTS_OPERATION, OUTLINE_MUTATION_CAPABILITY,
    OUTLINE_READ_CAPABILITY, OUTLINE_WORKER_CONTRACT_VERSION, OUTLINE_WORKER_ID,
    PartOutlineAccessInput, ProjectOutlineInput, SAVE_CHAPTER_OPERATION, SAVE_PART_OPERATION,
    UPDATE_CHAPTER_OPERATION,
};
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

const WORKER_ADDR_ENV: &str = "A4N_OUTLINE_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_OUTLINE_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47653";

#[derive(Clone)]
pub struct OutlineWorker {
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

impl OutlineWorker {
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
            .route(
                "/v1/commands/outline.records.list_chapters",
                post(list_chapters),
            )
            .route("/v1/commands/outline.records.list_parts", post(list_parts))
            .route(
                "/v1/commands/outline.records.generation_progress",
                post(generation_progress),
            )
            .route(
                "/v1/commands/outline.records.ensure_chapter_access",
                post(ensure_chapter_access),
            )
            .route(
                "/v1/commands/outline.records.ensure_part_access",
                post(ensure_part_access),
            )
            .route(
                "/v1/commands/outline.records.delete_chapters",
                post(delete_chapters),
            )
            .route(
                "/v1/commands/outline.records.delete_parts",
                post(delete_parts),
            )
            .route(
                "/v1/commands/outline.records.save_chapter",
                post(save_chapter),
            )
            .route(
                "/v1/commands/outline.records.update_chapter",
                post(update_chapter),
            )
            .route("/v1/commands/outline.records.save_part", post(save_part))
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
    let worker = OutlineWorker::new(database, OUTLINE_WORKER_ID, auth_token);
    tracing::info!(%address, "A4N outline worker listening");
    worker.serve(address).await
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
        capabilities: [OUTLINE_READ_CAPABILITY, OUTLINE_MUTATION_CAPABILITY]
            .into_iter()
            .map(|name| WorkerCapability {
                name: name.into(),
                version: OUTLINE_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            })
            .collect(),
    })
    .into_response()
}

async fn list_chapters(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectOutlineInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.list_chapters(request)
    })
    .await
}

async fn list_parts(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectOutlineInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.list_parts(request)
    })
    .await
}

async fn generation_progress(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectOutlineInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.generation_progress(request)
    })
    .await
}

async fn ensure_chapter_access(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterOutlineAccessInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.ensure_chapter_access(request)
    })
    .await
}

async fn ensure_part_access(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<PartOutlineAccessInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.ensure_part_access(request)
    })
    .await
}

async fn delete_chapters(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<DeleteLatestOutlineRecordsInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.delete_chapters(request)
    })
    .await
}

async fn delete_parts(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<DeleteLatestOutlineRecordsInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.delete_parts(request)
    })
    .await
}

async fn save_chapter(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<SaveChapterOutlineInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.save_chapter(request)
    })
    .await
}

async fn update_chapter(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<UpdateChapterOutlineInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.update_chapter(request)
    })
    .await
}

async fn save_part(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<SavePartOutlineInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.save_part(request)
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
    async fn list_chapters(
        &self,
        request: WorkerEnvelope<ProjectOutlineInput>,
    ) -> Result<Vec<ChapterOutlineSummary>, WorkerFailure> {
        validate_request(&request, LIST_CHAPTERS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let outlines = a4n_db::project::chapter_outlines_for_project(
            self.database.connection(),
            request.payload.project_id,
        )
        .await
        .map_err(outline_record_error)?;
        let ids = outlines
            .iter()
            .map(|outline| outline.id)
            .collect::<Vec<_>>();
        let source_kinds =
            a4n_db::project::chapter_outline_source_kinds_for_ids(self.database.connection(), &ids)
                .await
                .map_err(outline_record_error)?;
        Ok(outlines
            .iter()
            .map(|outline| chapter_outline_summary(outline, source_kinds.get(&outline.id)))
            .collect())
    }

    async fn list_parts(
        &self,
        request: WorkerEnvelope<ProjectOutlineInput>,
    ) -> Result<Vec<PartOutlineSummary>, WorkerFailure> {
        validate_request(&request, LIST_PARTS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        Ok(a4n_db::project::part_outlines_for_project(
            self.database.connection(),
            request.payload.project_id,
        )
        .await
        .map_err(outline_record_error)?
        .iter()
        .map(part_outline_summary)
        .collect())
    }

    async fn generation_progress(
        &self,
        request: WorkerEnvelope<ProjectOutlineInput>,
    ) -> Result<OutlineGenerationProgressSummary, WorkerFailure> {
        validate_request(&request, GENERATION_PROGRESS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let rows = a4n_db::project::outline_generation_progress_rows(
            self.database.connection(),
            request.payload.project_id,
        )
        .await
        .map_err(outline_record_error)?;
        generation_progress_summary(&rows)
    }

    async fn ensure_chapter_access(
        &self,
        request: WorkerEnvelope<ChapterOutlineAccessInput>,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, ENSURE_CHAPTER_ACCESS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        positive(request.payload.outline_id, "outline_id")?;
        let user_id = user_id(&request)?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let outline = a4n_db::project::chapter_outline_by_id(
            self.database.connection(),
            request.payload.outline_id,
        )
        .await
        .map_err(outline_record_error)?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("章节大纲不存在: {}", request.payload.outline_id))
        })?;
        ensure_project_ownership(outline.project_id, request.payload.project_id, "章节大纲")
    }

    async fn ensure_part_access(
        &self,
        request: WorkerEnvelope<PartOutlineAccessInput>,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, ENSURE_PART_ACCESS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        positive(request.payload.part_id, "part_id")?;
        let user_id = user_id(&request)?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let part = a4n_db::project::part_outline_by_id(
            self.database.connection(),
            request.payload.part_id,
        )
        .await
        .map_err(outline_record_error)?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("分部大纲不存在: {}", request.payload.part_id))
        })?;
        ensure_project_ownership(part.project_id, request.payload.project_id, "分部大纲")
    }

    async fn delete_chapters(
        &self,
        request: WorkerEnvelope<DeleteLatestOutlineRecordsInput>,
    ) -> Result<OutlineMutationResult, WorkerFailure> {
        validate_request(&request, DELETE_CHAPTERS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        a4n_db::project::delete_latest_chapter_outline_records(
            self.database.connection(),
            delete_input(request.payload),
        )
        .await
        .map_err(outline_record_error)?;
        Ok(OutlineMutationResult {
            warnings: Vec::new(),
        })
    }

    async fn delete_parts(
        &self,
        request: WorkerEnvelope<DeleteLatestOutlineRecordsInput>,
    ) -> Result<OutlineMutationResult, WorkerFailure> {
        validate_request(&request, DELETE_PARTS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        a4n_db::project::delete_latest_part_outline_records(
            self.database.connection(),
            delete_input(request.payload),
        )
        .await
        .map_err(outline_record_error)?;
        Ok(OutlineMutationResult {
            warnings: Vec::new(),
        })
    }

    async fn save_chapter(
        &self,
        request: WorkerEnvelope<SaveChapterOutlineInput>,
    ) -> Result<SaveChapterOutlineResult, WorkerFailure> {
        validate_request(&request, SAVE_CHAPTER_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let outline = a4n_db::project::save_manual_chapter_outline(
            self.database.connection(),
            a4n_db::project::ManualChapterOutlineSaveInput {
                project_id: request.payload.project_id,
                outline_id: request.payload.outline_id,
                expected_updated_at: request.payload.expected_updated_at,
                chapter_number: request.payload.chapter_number,
                title: request.payload.title,
                summary: request.payload.summary,
            },
        )
        .await
        .map_err(outline_record_error)?;
        Ok(SaveChapterOutlineResult {
            outline: chapter_outline_summary(&outline, None),
            warnings: Vec::new(),
        })
    }

    async fn update_chapter(
        &self,
        request: WorkerEnvelope<UpdateChapterOutlineInput>,
    ) -> Result<UpdateChapterOutlineResult, WorkerFailure> {
        validate_request(&request, UPDATE_CHAPTER_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let input = normalize_update_input(request.payload)?;
        let outline =
            a4n_db::project::update_chapter_outline_record(self.database.connection(), input)
                .await
                .map_err(update_error)?;
        Ok(UpdateChapterOutlineResult {
            outline: chapter_outline_summary(&outline, None),
            warnings: Vec::new(),
        })
    }

    async fn save_part(
        &self,
        request: WorkerEnvelope<SavePartOutlineInput>,
    ) -> Result<SavePartOutlineResult, WorkerFailure> {
        validate_request(&request, SAVE_PART_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let part = a4n_db::project::save_manual_part_outline(
            self.database.connection(),
            a4n_db::project::ManualPartOutlineSaveInput {
                project_id: request.payload.project_id,
                part_id: request.payload.part_id,
                expected_updated_at: request.payload.expected_updated_at,
                part_number: request.payload.part_number,
                start_chapter: request.payload.start_chapter,
                end_chapter: request.payload.end_chapter,
                title: request.payload.title,
                summary: request.payload.summary,
                theme: request.payload.theme,
            },
        )
        .await
        .map_err(outline_record_error)?;
        Ok(SavePartOutlineResult {
            part: part_outline_summary(&part),
            warnings: Vec::new(),
        })
    }

    async fn verify_permission(
        &self,
        project_id: i64,
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<(), WorkerFailure> {
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            required,
        )
        .await
        .map(|_| ())
        .map_err(collaboration_error)
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(a4n_worker_client::WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}

async fn verify_project_permission(
    state: &WorkerState,
    project_id: i64,
    user_id: i64,
    required: ProjectAccessRole,
) -> Result<(), WorkerFailure> {
    state.verify_permission(project_id, user_id, required).await
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
    if request.lease.worker_id != OUTLINE_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 outline worker",
        ));
    }
    Ok(())
}

fn validate_project_scope<T>(
    request: &WorkerEnvelope<T>,
    project_id: i64,
) -> Result<(), WorkerFailure> {
    positive(project_id, "project_id")?;
    if request.project_id != Some(project_id) {
        return Err(WorkerFailure::invalid_request(format!(
            "worker scope project_id 与 payload 不一致: scope={:?}, payload={project_id}",
            request.project_id
        )));
    }
    Ok(())
}

fn positive(value: i64, field: &str) -> Result<(), WorkerFailure> {
    if value <= 0 {
        return Err(WorkerFailure::invalid_request(format!(
            "{field} 必须大于 0"
        )));
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

fn ensure_project_ownership(
    actual_project_id: i64,
    expected_project_id: i64,
    kind: &str,
) -> Result<(), WorkerFailure> {
    if actual_project_id == expected_project_id {
        Ok(())
    } else {
        Err(WorkerFailure::unauthorized(format!(
            "{kind} 不属于项目 {expected_project_id}"
        )))
    }
}

fn chapter_outline_summary(
    outline: &chapter_outline::Model,
    source_kind: Option<&String>,
) -> ChapterOutlineSummary {
    ChapterOutlineSummary {
        id: outline.id,
        project_id: outline.project_id,
        part_id: outline.part_id,
        chapter_number: outline.chapter_number,
        title: outline.title.clone(),
        summary: outline.summary.clone(),
        beats: outline.beats.clone(),
        key_scenes: outline.key_scenes.clone(),
        pacing_score: outline.pacing_score,
        foreshadowing_setup: outline.foreshadowing_setup.clone(),
        foreshadowing_payoff: outline.foreshadowing_payoff.clone(),
        characters_involved: outline.characters_involved.clone(),
        word_count_target: outline.word_count_target,
        source_kind: source_kind.cloned(),
        status: outline.status.clone(),
        created_at: Some(outline.created_at.to_string()),
        updated_at: Some(outline.updated_at.to_string()),
    }
}

fn part_outline_summary(part: &part_outline::Model) -> PartOutlineSummary {
    PartOutlineSummary {
        id: part.id,
        project_id: part.project_id,
        part_number: part.part_number,
        title: part.title.clone(),
        start_chapter: part.start_chapter,
        end_chapter: part.end_chapter,
        summary: part.summary.clone(),
        theme: part.theme.clone(),
        key_events: part.key_events.clone(),
        character_focus: part.character_focus.clone(),
        created_at: Some(part.created_at.to_string()),
        updated_at: Some(part.updated_at.to_string()),
    }
}

fn generation_progress_summary(
    rows: &OutlineGenerationProgressRows,
) -> Result<OutlineGenerationProgressSummary, WorkerFailure> {
    let mut outlines_by_chapter = HashMap::<i32, &str>::new();
    for outline in &rows.chapter_outlines {
        if !chapter_outline::is_valid_status(&outline.status) {
            return Err(WorkerFailure::conflict(format!(
                "章节大纲 {} 状态无效：{}",
                outline.id, outline.status
            )));
        }
        if outlines_by_chapter
            .insert(outline.chapter_number, outline.status.as_str())
            .is_some()
        {
            return Err(WorkerFailure::conflict(format!(
                "章节号 {} 存在重复章节大纲",
                outline.chapter_number
            )));
        }
    }

    let mut claimed_chapters = HashSet::new();
    let mut completed_parts = 0_u64;
    for part in &rows.parts {
        if part.start_chapter <= 0 || part.end_chapter < part.start_chapter {
            return Err(WorkerFailure::conflict(format!(
                "分部 {} 章节范围无效：{}..={}",
                part.id, part.start_chapter, part.end_chapter
            )));
        }
        for chapter_number in part.start_chapter..=part.end_chapter {
            if !claimed_chapters.insert(chapter_number) {
                return Err(WorkerFailure::conflict(format!(
                    "章节号 {} 被多个分部范围覆盖",
                    chapter_number
                )));
            }
        }
        if (part.start_chapter..=part.end_chapter).all(|chapter_number| {
            outlines_by_chapter
                .get(&chapter_number)
                .is_some_and(|status| *status == chapter_outline::STATUS_COMPLETED)
        }) {
            completed_parts += 1;
        }
    }

    Ok(OutlineGenerationProgressSummary {
        total_parts: rows.parts.len() as u64,
        completed_parts,
        total_chapter_outlines: rows.chapter_outlines.len() as u64,
        completed_chapter_outlines: rows
            .chapter_outlines
            .iter()
            .filter(|outline| outline.status == chapter_outline::STATUS_COMPLETED)
            .count() as u64,
    })
}

fn delete_input(
    input: DeleteLatestOutlineRecordsInput,
) -> a4n_db::project::DeleteLatestOutlineRecordsInput {
    a4n_db::project::DeleteLatestOutlineRecordsInput {
        project_id: input.project_id,
        targets: input
            .targets
            .into_iter()
            .map(|target| a4n_db::project::OutlineDeleteTarget {
                id: target.id,
                expected_updated_at: target.expected_updated_at,
            })
            .collect(),
    }
}

fn normalize_update_input(
    input: UpdateChapterOutlineInput,
) -> Result<a4n_db::project::ProjectChapterOutlineUpdateInput, WorkerFailure> {
    positive(input.project_id, "project_id")?;
    positive(input.outline_id, "outline_id")?;
    let expected_updated_at = input.expected_updated_at.trim().to_string();
    if expected_updated_at.is_empty() {
        return Err(WorkerFailure::invalid_request(
            "缺少章节大纲资料版本，请刷新后重试",
        ));
    }
    Ok(a4n_db::project::ProjectChapterOutlineUpdateInput {
        project_id: input.project_id,
        outline_id: input.outline_id,
        expected_updated_at,
        title: optional_text(input.title, "title")?,
        summary: optional_text(input.summary, "summary")?,
        beats: optional_array(input.beats, "beats")?,
        key_scenes: optional_array(input.key_scenes, "key_scenes")?,
    })
}

fn optional_text(value: Option<String>, field: &str) -> Result<Option<String>, WorkerFailure> {
    value
        .map(|value| {
            let value = value.trim().to_string();
            if value.is_empty() {
                Err(WorkerFailure::invalid_request(format!(
                    "章节大纲字段 {field} 不能为空"
                )))
            } else {
                Ok(value)
            }
        })
        .transpose()
}

fn optional_array(
    value: Option<serde_json::Value>,
    field: &str,
) -> Result<Option<serde_json::Value>, WorkerFailure> {
    value
        .map(|value| {
            if value.is_array() {
                Ok(value)
            } else {
                Err(WorkerFailure::invalid_request(format!(
                    "章节大纲字段 {field} 必须是数组"
                )))
            }
        })
        .transpose()
}

fn outline_record_error(error: a4n_db::project::ProjectOutlineRecordError) -> WorkerFailure {
    match error {
        a4n_db::project::ProjectOutlineRecordError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ProjectOutlineRecordError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}; operation={operation}"))
        }
        a4n_db::project::ProjectOutlineRecordError::ChapterSourceChanged { .. }
        | a4n_db::project::ProjectOutlineRecordError::OutlineSourceChanged { .. } => {
            WorkerFailure::conflict(error.to_string())
        }
        a4n_db::project::ProjectOutlineRecordError::Generation(error) => match error {
            a4n_db::generation::GenerationDbError::InvalidArgument(message) => {
                WorkerFailure::invalid_request(message)
            }
            a4n_db::generation::GenerationDbError::NotFound(message) => {
                WorkerFailure::not_found(message)
            }
            a4n_db::generation::GenerationDbError::Database(error) => {
                WorkerFailure::unavailable(error.to_string())
            }
        },
        a4n_db::project::ProjectOutlineRecordError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

fn update_error(error: a4n_db::project::ProjectChapterOutlineUpdateError) -> WorkerFailure {
    match error {
        a4n_db::project::ProjectChapterOutlineUpdateError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ProjectChapterOutlineUpdateError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::project::ProjectChapterOutlineUpdateError::Forbidden(message) => {
            WorkerFailure::unauthorized(message)
        }
        a4n_db::project::ProjectChapterOutlineUpdateError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}; operation={operation}"))
        }
        a4n_db::project::ProjectChapterOutlineUpdateError::SourceChanged => {
            WorkerFailure::conflict(error.to_string())
        }
        a4n_db::project::ProjectChapterOutlineUpdateError::Database(error) => {
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

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        "Outline worker authentication required",
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
    use super::*;

    #[test]
    fn progress_rejects_duplicate_ranges_and_counts_completed_parts() {
        let rows = OutlineGenerationProgressRows {
            parts: vec![
                a4n_db::project::OutlineGenerationProgressPart {
                    id: 1,
                    start_chapter: 1,
                    end_chapter: 2,
                },
                a4n_db::project::OutlineGenerationProgressPart {
                    id: 2,
                    start_chapter: 3,
                    end_chapter: 3,
                },
            ],
            chapter_outlines: vec![
                a4n_db::project::OutlineGenerationProgressChapter {
                    id: 11,
                    chapter_number: 1,
                    status: chapter_outline::STATUS_COMPLETED.into(),
                },
                a4n_db::project::OutlineGenerationProgressChapter {
                    id: 12,
                    chapter_number: 2,
                    status: chapter_outline::STATUS_COMPLETED.into(),
                },
                a4n_db::project::OutlineGenerationProgressChapter {
                    id: 13,
                    chapter_number: 3,
                    status: chapter_outline::STATUS_DRAFT.into(),
                },
            ],
        };
        let progress = generation_progress_summary(&rows).expect("progress");
        assert_eq!(progress.completed_parts, 1);
        assert_eq!(progress.completed_chapter_outlines, 2);

        let mut duplicate = rows;
        duplicate.parts[1].start_chapter = 2;
        assert!(generation_progress_summary(&duplicate).is_err());
    }

    #[test]
    fn worker_scope_rejects_mismatched_project() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: a4n_worker_contract::WorkerPrincipal {
                kind: a4n_worker_contract::WorkerPrincipalKind::User,
                subject: "1".into(),
            },
            operation_id: LIST_CHAPTERS_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(2),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: a4n_worker_contract::WorkerLease {
                worker_id: OUTLINE_WORKER_ID.into(),
                epoch: 1,
                fence_token: "fence".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ProjectOutlineInput { project_id: 1 },
        };
        assert!(validate_project_scope(&request, request.payload.project_id).is_err());
    }
}
