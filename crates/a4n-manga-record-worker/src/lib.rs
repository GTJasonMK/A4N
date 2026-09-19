//! PostgreSQL-backed Manga page prompt read worker.

use std::{
    collections::{HashMap, HashSet},
    env,
    net::SocketAddr,
    sync::Arc,
};

use a4n_api_contract::manga::{MangaPagePromptSummary, MangaPipelineStatusSummary};
use a4n_db::DbHandle;
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_manga_record_worker_contract::{
    MANGA_PIPELINE_CANCEL_OPERATION, MANGA_PIPELINE_CAPABILITY, MANGA_PIPELINE_STATUS_OPERATION,
    MANGA_PIPELINE_STATUSES_OPERATION, MANGA_RECORD_CAPABILITY, MANGA_RECORD_LIST_OPERATION,
    MANGA_RECORD_WORKER_CONTRACT_VERSION, MANGA_RECORD_WORKER_ID, MangaChapterInput,
    MangaPipelineStatusesInput,
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
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Statement};
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_MANGA_RECORD_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_MANGA_RECORD_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47676";

#[derive(Clone)]
pub struct MangaRecordWorker {
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

impl MangaRecordWorker {
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
                "/v1/commands/manga.record.list_page_prompts",
                post(list_page_prompts),
            )
            .route("/v1/commands/manga.pipeline.status", post(pipeline_status))
            .route(
                "/v1/commands/manga.pipeline.statuses",
                post(pipeline_statuses),
            )
            .route("/v1/commands/manga.pipeline.cancel", post(pipeline_cancel))
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
    MangaRecordWorker::new(database, MANGA_RECORD_WORKER_ID, auth_token)
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
                name: MANGA_RECORD_CAPABILITY.into(),
                version: MANGA_RECORD_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: MANGA_PIPELINE_CAPABILITY.into(),
                version: MANGA_RECORD_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

async fn list_page_prompts(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<MangaChapterInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.list_page_prompts(request)
    })
    .await
}

async fn pipeline_status(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<MangaChapterInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.pipeline_status(request)
    })
    .await
}

async fn pipeline_statuses(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<MangaPipelineStatusesInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.pipeline_statuses(request)
    })
    .await
}

async fn pipeline_cancel(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<MangaChapterInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.pipeline_cancel(request)
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
    async fn list_page_prompts(
        &self,
        request: WorkerEnvelope<MangaChapterInput>,
    ) -> Result<Vec<MangaPagePromptSummary>, WorkerFailure> {
        validate_request(&request, MANGA_RECORD_LIST_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        positive(input.project_id, "project_id")?;
        positive(input.chapter_id, "chapter_id")?;
        verify_manga_viewer(&self.database, input.project_id, user_id).await?;
        let prompts = a4n_db::manga::manga_page_prompts_by_chapter(
            self.database.connection(),
            input.project_id,
            input.chapter_id,
        )
        .await
        .map_err(manga_error)?;
        prompts
            .iter()
            .map(|prompt| {
                a4n_manga_core::manga_page_prompt_summary(prompt)
                    .map_err(WorkerFailure::invalid_request)
            })
            .collect()
    }

    async fn pipeline_status(
        &self,
        request: WorkerEnvelope<MangaChapterInput>,
    ) -> Result<MangaPipelineStatusSummary, WorkerFailure> {
        validate_request(&request, MANGA_PIPELINE_STATUS_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let mut statuses = self
            .pipeline_statuses_for(user_id, input.project_id, &[input.chapter_id])
            .await?;
        statuses
            .pop()
            .ok_or_else(|| WorkerFailure::invalid_request("章节状态列表为空"))
    }

    async fn pipeline_statuses(
        &self,
        request: WorkerEnvelope<MangaPipelineStatusesInput>,
    ) -> Result<Vec<MangaPipelineStatusSummary>, WorkerFailure> {
        validate_request(&request, MANGA_PIPELINE_STATUSES_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        self.pipeline_statuses_for(user_id, input.project_id, &input.chapter_ids)
            .await
    }

    async fn pipeline_cancel(
        &self,
        request: WorkerEnvelope<MangaChapterInput>,
    ) -> Result<i64, WorkerFailure> {
        validate_request(&request, MANGA_PIPELINE_CANCEL_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        positive(input.project_id, "project_id")?;
        positive(input.chapter_id, "chapter_id")?;
        verify_project_permission(
            &self.database,
            input.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let source = a4n_db::project::chapter_manga_source_by_id(
            self.database.connection(),
            input.chapter_id,
            input.project_id,
            false,
        )
        .await
        .map_err(project_family_error)?;
        if source.status == "deleted" {
            return Err(WorkerFailure::invalid_request(format!(
                "第 {} 章已删除，无法取消漫画流水线",
                source.chapter_number
            )));
        }
        if source.status == "generating" {
            return Err(WorkerFailure::invalid_request(format!(
                "第 {} 章正在生成中，请等待完成后再取消漫画流水线",
                source.chapter_number
            )));
        }
        let run = a4n_db::workflow::list_workflow_runs(
            self.database.connection(),
            a4n_db::workflow::ListWorkflowRunsInput {
                project_id: Some(input.project_id),
                chapter_id: Some(input.chapter_id),
                chapter_ids: None,
                workflow_kind: Some("manga_pipeline".to_string()),
                status: Some("running".to_string()),
                resumable_only: None,
                limit: Some(1),
            },
        )
        .await
        .map_err(workflow_error)?
        .into_iter()
        .next()
        .ok_or_else(|| WorkerFailure::conflict("no_running_manga_pipeline: 取消漫画流水线"))?;
        a4n_db::workflow::mark_workflow_status(
            self.database.connection(),
            run.id,
            "cancelled",
            true,
            Some("用户请求取消漫画流水线"),
        )
        .await
        .map_err(workflow_write_error)?;
        a4n_db::manga::cancel_manga_page_prompts(
            self.database.connection(),
            input.project_id,
            input.chapter_id,
        )
        .await
        .map_err(manga_pipeline_error)
    }

    async fn pipeline_statuses_for(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_ids: &[i64],
    ) -> Result<Vec<MangaPipelineStatusSummary>, WorkerFailure> {
        positive(project_id, "project_id")?;
        verify_manga_viewer(&self.database, project_id, user_id).await?;
        if chapter_ids.is_empty() {
            return Err(WorkerFailure::invalid_request("chapter_ids 不能为空"));
        }
        if chapter_ids.len() > 200 {
            return Err(WorkerFailure::invalid_request(
                "chapter_ids 最多支持 200 个章节",
            ));
        }
        let mut seen = HashSet::with_capacity(chapter_ids.len());
        for chapter_id in chapter_ids {
            positive(*chapter_id, "chapter_id")?;
            if !seen.insert(*chapter_id) {
                return Err(WorkerFailure::invalid_request(format!(
                    "chapter_id 重复: {chapter_id}"
                )));
            }
        }

        let mut summaries = chapter_ids
            .iter()
            .copied()
            .map(|chapter_id| (chapter_id, empty_pipeline_status(chapter_id)))
            .collect::<HashMap<_, _>>();
        let (prompt_statuses, runs) = tokio::try_join!(
            async {
                a4n_db::manga::manga_page_prompt_statuses_by_chapters(
                    self.database.connection(),
                    project_id,
                    chapter_ids,
                )
                .await
                .map_err(manga_error)
            },
            async {
                a4n_db::workflow::list_workflow_runs(
                    self.database.connection(),
                    a4n_db::workflow::ListWorkflowRunsInput {
                        project_id: Some(project_id),
                        chapter_id: None,
                        chapter_ids: Some(chapter_ids.to_vec()),
                        workflow_kind: Some("manga_pipeline".to_string()),
                        status: None,
                        resumable_only: None,
                        limit: None,
                    },
                )
                .await
                .map_err(workflow_error)
            },
        )?;
        for (chapter_id, prompt_id, status) in prompt_statuses {
            let summary = summaries.get_mut(&chapter_id).ok_or_else(|| {
                WorkerFailure::invalid_request(format!("批量状态返回未知章节: {chapter_id}"))
            })?;
            match status.as_str() {
                "pending" => summary.pending += 1,
                "generating" => summary.generating += 1,
                "completed" => summary.completed += 1,
                "failed" => summary.failed += 1,
                "cancelled" => summary.cancelled += 1,
                status => {
                    return Err(WorkerFailure::invalid_request(format!(
                        "漫画页面提示词 #{} 包含未知状态: {status}",
                        prompt_id
                    )));
                }
            }
        }

        let mut latest_runs = HashMap::new();
        for run in runs {
            let chapter_id = run.chapter_id.ok_or_else(|| {
                WorkerFailure::invalid_request("漫画流水线 workflow 缺少 chapter_id")
            })?;
            latest_runs.entry(chapter_id).or_insert(run);
        }
        let run_ids = latest_runs.values().map(|run| run.id).collect::<Vec<_>>();
        let checkpoints = a4n_db::workflow::workflow_checkpoint_statuses_for_runs(
            self.database.connection(),
            &run_ids,
        )
        .await
        .map_err(workflow_error)?;
        let completed_checkpoint_runs = checkpoints
            .into_iter()
            .filter_map(|(run_id, status)| (status == "completed").then_some(run_id))
            .collect::<HashSet<_>>();
        let checkpoint_chapter_ids = latest_runs
            .iter()
            .filter_map(|(chapter_id, run)| {
                completed_checkpoint_runs
                    .contains(&run.id)
                    .then_some(*chapter_id)
            })
            .collect::<Vec<_>>();
        let (sources, character_portraits) = tokio::try_join!(
            async {
                a4n_db::project::chapter_manga_sources_by_ids(
                    self.database.connection(),
                    project_id,
                    &checkpoint_chapter_ids,
                    false,
                )
                .await
                .map_err(project_family_error)
            },
            async {
                if checkpoint_chapter_ids.is_empty() {
                    Ok(Vec::new())
                } else {
                    a4n_db::project::manga_character_portrait_refs(
                        self.database.connection(),
                        project_id,
                    )
                    .await
                    .map_err(project_family_error)
                }
            },
        )?;
        let sources_by_chapter = sources
            .into_iter()
            .map(|source| (source.chapter_id, source))
            .collect::<HashMap<_, _>>();
        for chapter_id in chapter_ids {
            let Some(run) = latest_runs.get(chapter_id) else {
                continue;
            };
            let summary = summaries.get_mut(chapter_id).ok_or_else(|| {
                WorkerFailure::invalid_request(format!("批量状态返回未知章节: {chapter_id}"))
            })?;
            summary.pipeline_status = Some(run.status.clone());
            summary.stage = run.current_step_key.clone();
            summary.task_id = run.active_task_id.clone();
            summary.can_cancel = run.status == "running";
            summary.message = run.error_message.clone();
            if let Some(progress) = run.progress.as_ref() {
                summary.current = Some(i32::try_from(progress.current).map_err(|_| {
                    WorkerFailure::invalid_request("漫画流水线 current 超出 i32 范围")
                })?);
                summary.total = Some(i32::try_from(progress.total).map_err(|_| {
                    WorkerFailure::invalid_request("漫画流水线 total 超出 i32 范围")
                })?);
                if summary.message.is_none() {
                    summary.message = Some(progress.message.clone());
                }
            }
            summary.has_checkpoint = completed_checkpoint_runs.contains(&run.id);
            if summary.has_checkpoint {
                let source = sources_by_chapter
                    .get(chapter_id)
                    .ok_or_else(|| WorkerFailure::not_found(format!("章节不存在: {chapter_id}")))?;
                let current_fingerprint = a4n_manga_core::manga_pipeline_fingerprint(
                    &source.content,
                    &character_portraits,
                );
                summary.checkpoint_valid =
                    run.input_fingerprint_json.as_ref() == Some(&current_fingerprint);
            }
            summary.can_resume = run.can_resume
                && summary.checkpoint_valid
                && matches!(run.status.as_str(), "failed" | "cancelled" | "interrupted");
        }
        chapter_ids
            .iter()
            .map(|chapter_id| {
                summaries.remove(chapter_id).ok_or_else(|| {
                    WorkerFailure::invalid_request(format!("批量状态缺少章节: {chapter_id}"))
                })
            })
            .collect()
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}

async fn verify_manga_viewer(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
) -> Result<(), WorkerFailure> {
    let (project_id, root_project_id) =
        a4n_db::project::project_or_derived_family_access_ids(database.connection(), project_id)
            .await
            .map_err(project_family_error)?;
    let project_ids = root_project_id
        .into_iter()
        .chain([project_id])
        .collect::<Vec<_>>();
    a4n_db::collaboration::verify_project_permissions(
        database.connection(),
        &project_ids,
        user_id,
        ProjectAccessRole::Viewer,
    )
    .await
    .map(|_| ())
    .map_err(collaboration_error)
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
    if request.lease.worker_id != MANGA_RECORD_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 manga record worker",
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

fn positive(value: i64, field: &str) -> Result<i64, WorkerFailure> {
    if value > 0 {
        Ok(value)
    } else {
        Err(WorkerFailure::invalid_request(format!(
            "{field} 必须大于 0"
        )))
    }
}

fn empty_pipeline_status(chapter_id: i64) -> MangaPipelineStatusSummary {
    MangaPipelineStatusSummary {
        chapter_id,
        pending: 0,
        generating: 0,
        completed: 0,
        failed: 0,
        cancelled: 0,
        pipeline_status: None,
        stage: None,
        current: None,
        total: None,
        message: None,
        task_id: None,
        can_cancel: false,
        can_resume: false,
        has_checkpoint: false,
        checkpoint_valid: false,
    }
}

fn manga_error(error: a4n_db::manga::MangaPromptRecordError) -> WorkerFailure {
    match error {
        a4n_db::manga::MangaPromptRecordError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::manga::MangaPromptRecordError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::manga::MangaPromptRecordError::Conflict(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::manga::MangaPromptRecordError::Database { source, .. } => {
            WorkerFailure::unavailable(source.to_string())
        }
        a4n_db::manga::MangaPromptRecordError::Generation(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

fn workflow_error(error: a4n_db::workflow::WorkflowReadError) -> WorkerFailure {
    match error {
        a4n_db::workflow::WorkflowReadError::InvalidArgument(message)
        | a4n_db::workflow::WorkflowReadError::InvalidData(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::workflow::WorkflowReadError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_db::workflow::WorkflowReadError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

fn workflow_write_error(error: a4n_db::workflow::WorkflowWriteError) -> WorkerFailure {
    match error {
        a4n_db::workflow::WorkflowWriteError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::workflow::WorkflowWriteError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::workflow::WorkflowWriteError::InvalidData(message) => {
            WorkerFailure::unavailable(message)
        }
        a4n_db::workflow::WorkflowWriteError::StatusConflict { current, operation } => {
            WorkerFailure::conflict(format!("{current}: {operation}"))
        }
        a4n_db::workflow::WorkflowWriteError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

fn manga_pipeline_error(error: a4n_db::manga::MangaPipelineDbError) -> WorkerFailure {
    match error {
        a4n_db::manga::MangaPipelineDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::manga::MangaPipelineDbError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
        a4n_db::manga::MangaPipelineDbError::Generation(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

fn project_family_error(error: a4n_db::project::ProjectFamilyDbError) -> WorkerFailure {
    match error {
        a4n_db::project::ProjectFamilyDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ProjectFamilyDbError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::project::ProjectFamilyDbError::Database(error) => {
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
