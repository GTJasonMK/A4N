//! PostgreSQL-backed chapter-content worker.

use std::{collections::BTreeSet, env, net::SocketAddr, sync::Arc};

use a4n_chapter_content_core::{
    CHAPTER_CONTENT_SYNC_WORKFLOW_KIND, attach_chapter_sync_state,
    begin_chapter_content_sync_in_transaction, chapter_cleanup_requires_snapshot, chapter_summary,
    validate_chapter_content_snapshot_preflight,
};
use a4n_chapter_content_worker_contract::{
    CHAPTER_CONTENT_CAPABILITY, CHAPTER_CONTENT_IMPORT_OPERATION, CHAPTER_CONTENT_LIST_OPERATION,
    CHAPTER_CONTENT_SUMMARY_OPERATION, CHAPTER_CONTENT_UPDATE_OPERATION,
    CHAPTER_CONTENT_WORKER_CONTRACT_VERSION, CHAPTER_CONTENT_WORKER_ID,
    ChapterContentMutationResult, ChapterInput, ChapterSummary, ImportChapterInput, ProjectInput,
    UpdateChapterContentInput,
};
use a4n_db::DbHandle;
use a4n_domain::chapter::{EditChapterContentInput, ImportChapterContentInput};
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
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DatabaseTransaction, Statement,
    TransactionTrait,
};
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_CHAPTER_CONTENT_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_CHAPTER_CONTENT_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47671";

#[derive(Clone)]
pub struct ChapterContentWorker {
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

impl ChapterContentWorker {
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
            .route("/v1/commands/chapter.content.list", post(list))
            .route("/v1/commands/chapter.content.summary", post(summary))
            .route("/v1/commands/chapter.content.update", post(update))
            .route("/v1/commands/chapter.content.import", post(import_chapter))
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
    ChapterContentWorker::new(database, CHAPTER_CONTENT_WORKER_ID, auth_token)
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
            name: CHAPTER_CONTENT_CAPABILITY.into(),
            version: CHAPTER_CONTENT_WORKER_CONTRACT_VERSION.into(),
            enabled: true,
        }],
    })
    .into_response()
}

async fn list(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.list(request)).await
}

async fn summary(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.summary(request)).await
}

async fn update(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<UpdateChapterContentInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.update(request)).await
}

async fn import_chapter(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ImportChapterInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.import_chapter(request)
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
    async fn update(
        &self,
        request: WorkerEnvelope<UpdateChapterContentInput>,
    ) -> Result<ChapterContentMutationResult, WorkerFailure> {
        validate_request(&request, CHAPTER_CONTENT_UPDATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let chapter_id = positive("chapter_id", request.payload.chapter_id)?;
        let existing = a4n_db::chapter_content::chapter_by_id_for_analysis(
            self.database.connection(),
            chapter_id,
        )
        .await
        .map_err(chapter_error)?;
        self.verify_text_project_editor(existing.project_id, user_id, "更新章节正文")
            .await?;
        self.validate_snapshot_preflight(existing.project_id, existing.chapter_number)
            .await?;

        let input = request.payload;
        let tx = self
            .database
            .connection()
            .begin()
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        let result = async {
            let chapter = a4n_db::chapter_content::save_chapter_content_edit_record(
                &tx,
                EditChapterContentInput {
                    chapter_id: input.chapter_id,
                    content: input.content,
                    title: input.title,
                },
            )
            .await
            .map_err(chapter_error)?;
            let run = begin_chapter_content_sync_in_transaction(&tx, &chapter, |_| Ok(()))
                .await
                .map_err(app_error)?;
            Ok(ChapterContentMutationResult {
                chapter_id: chapter.id,
                workflow_run_id: run.id,
            })
        }
        .await;
        finish_transaction(tx, result, "保存章节正文并登记资料同步").await
    }

    async fn import_chapter(
        &self,
        request: WorkerEnvelope<ImportChapterInput>,
    ) -> Result<ChapterContentMutationResult, WorkerFailure> {
        validate_request(&request, CHAPTER_CONTENT_IMPORT_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let project_id = positive("project_id", input.project_id)?;
        if input.chapter_number <= 0 {
            return Err(WorkerFailure::invalid_request("章节序号必须大于 0"));
        }
        self.verify_text_project_editor(project_id, user_id, "导入章节正文")
            .await?;
        self.validate_snapshot_preflight(project_id, input.chapter_number)
            .await?;

        let tx = self
            .database
            .connection()
            .begin()
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        let result = async {
            let chapter = a4n_db::chapter_content::import_chapter_content_record(
                &tx,
                ImportChapterContentInput {
                    project_id: input.project_id,
                    chapter_number: input.chapter_number,
                    content: input.content,
                    title: input.title,
                },
                "导入覆盖正文",
            )
            .await
            .map_err(import_error)?;
            let run = begin_chapter_content_sync_in_transaction(&tx, &chapter, |_| Ok(()))
                .await
                .map_err(app_error)?;
            Ok(ChapterContentMutationResult {
                chapter_id: chapter.id,
                workflow_run_id: run.id,
            })
        }
        .await;
        finish_transaction(tx, result, "导入章节正文并登记资料同步").await
    }

    async fn list(
        &self,
        request: WorkerEnvelope<ProjectInput>,
    ) -> Result<Vec<ChapterSummary>, WorkerFailure> {
        validate_request(&request, CHAPTER_CONTENT_LIST_OPERATION)?;
        let project_id = positive("project_id", request.payload.project_id)?;
        verify_project_viewer(&self.database, project_id, user_id(&request)?).await?;
        let chapters =
            a4n_db::chapter_content::chapters_for_project(self.database.connection(), project_id)
                .await
                .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        self.project_summaries(project_id, chapters).await
    }

    async fn summary(
        &self,
        request: WorkerEnvelope<ChapterInput>,
    ) -> Result<ChapterSummary, WorkerFailure> {
        validate_request(&request, CHAPTER_CONTENT_SUMMARY_OPERATION)?;
        let chapter_id = positive("chapter_id", request.payload.chapter_id)?;
        let chapter = a4n_db::chapter_content::chapter_by_id_for_analysis(
            self.database.connection(),
            chapter_id,
        )
        .await
        .map_err(chapter_error)?;
        verify_project_viewer(&self.database, chapter.project_id, user_id(&request)?).await?;
        let mut summaries = self
            .project_summaries(chapter.project_id, vec![chapter.clone()])
            .await?;
        summaries
            .pop()
            .ok_or_else(|| WorkerFailure::unavailable("章节摘要投影为空"))
    }

    async fn verify_text_project_editor(
        &self,
        project_id: i64,
        user_id: i64,
        operation: &str,
    ) -> Result<(), WorkerFailure> {
        verify_project_editor(&self.database, project_id, user_id).await?;
        let project = a4n_db::project::project_by_id_record(self.database.connection(), project_id)
            .await
            .map_err(project_error)?
            .ok_or_else(|| WorkerFailure::not_found(format!("项目不存在: {project_id}")))?;
        if project.project_kind != a4n_db::project::PROJECT_KIND_TEXT {
            return Err(WorkerFailure::invalid_request(format!(
                "{operation}只能作用于文本项目"
            )));
        }
        Ok(())
    }

    async fn validate_snapshot_preflight(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<(), WorkerFailure> {
        let profiles = a4n_db::project::protagonist_profiles_for_project(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(project_chapter_context_error)?;
        let profile_ids = profiles
            .iter()
            .filter(|profile| chapter_cleanup_requires_snapshot(profile, chapter_number))
            .map(|profile| profile.id)
            .collect::<Vec<_>>();
        let existing_snapshot_profile_ids = a4n_db::project::snapshot_profile_ids_for_chapter(
            self.database.connection(),
            &profile_ids,
            chapter_number - 1,
        )
        .await
        .map_err(project_chapter_context_error)?
        .into_iter()
        .collect::<BTreeSet<_>>();
        validate_chapter_content_snapshot_preflight(
            &profiles,
            &existing_snapshot_profile_ids,
            chapter_number,
        )
        .map_err(app_error)
    }

    async fn project_summaries(
        &self,
        project_id: i64,
        chapters: Vec<a4n_db_models::models::chapter::chapter::Model>,
    ) -> Result<Vec<ChapterSummary>, WorkerFailure> {
        let (analysis_runs, content_runs) = tokio::try_join!(
            a4n_db::workflow::workflow_runs_by_project_kind(
                self.database.connection(),
                project_id,
                "chapter_analysis",
            ),
            a4n_db::workflow::workflow_runs_by_project_kind(
                self.database.connection(),
                project_id,
                CHAPTER_CONTENT_SYNC_WORKFLOW_KIND,
            ),
        )
        .map_err(workflow_error)?;
        let run_ids = analysis_runs
            .iter()
            .chain(&content_runs)
            .map(|run| run.id)
            .collect::<Vec<_>>();
        let checkpoints = a4n_db::workflow::workflow_checkpoint_models_for_runs(
            self.database.connection(),
            &run_ids,
        )
        .await
        .map_err(workflow_error)?;
        let mut summaries = chapters.iter().map(chapter_summary).collect::<Vec<_>>();
        attach_chapter_sync_state(
            &chapters,
            &mut summaries,
            &analysis_runs,
            &content_runs,
            &checkpoints,
        )
        .map_err(app_error)?;
        Ok(summaries)
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
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
        a4n_domain::collaboration::ProjectAccessRole::Editor,
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
    if request.lease.worker_id != CHAPTER_CONTENT_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 chapter content worker",
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

fn positive(field: &str, value: i64) -> Result<i64, WorkerFailure> {
    if value <= 0 {
        return Err(WorkerFailure::invalid_request(format!(
            "{field} 必须大于 0"
        )));
    }
    Ok(value)
}

async fn verify_project_viewer(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(
        database.connection(),
        project_id,
        user_id,
        a4n_domain::collaboration::ProjectAccessRole::Viewer,
    )
    .await
    .map(|_| ())
    .map_err(collaboration_error)
}

fn chapter_error(error: a4n_db::chapter_content::ChapterContentEditError) -> WorkerFailure {
    match error {
        a4n_db::chapter_content::ChapterContentEditError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::chapter_content::ChapterContentEditError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::chapter_content::ChapterContentEditError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}: {operation}"))
        }
        a4n_db::chapter_content::ChapterContentEditError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

fn import_error(error: a4n_db::chapter_content::ChapterContentImportError) -> WorkerFailure {
    match error {
        a4n_db::chapter_content::ChapterContentImportError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::chapter_content::ChapterContentImportError::TargetChapterGenerating {
            chapter_number,
            status,
            operation,
        } => WorkerFailure::conflict(format!(
            "第 {chapter_number} 章正在生成中，请等待当前生成完成后再{operation}（当前状态：{status}）"
        )),
        a4n_db::chapter_content::ChapterContentImportError::Database(error) => {
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

fn project_chapter_context_error(
    error: a4n_db::project::ProjectChapterContextError,
) -> WorkerFailure {
    match error {
        a4n_db::project::ProjectChapterContextError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ProjectChapterContextError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::project::ProjectChapterContextError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
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

fn app_error(error: a4n_error::AppError) -> WorkerFailure {
    match error {
        a4n_error::AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        a4n_error::AppError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_error::AppError::Forbidden(message) => WorkerFailure::unauthorized(message),
        a4n_error::AppError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}: {operation}"))
        }
        other => WorkerFailure::unavailable(other.to_string()),
    }
}

async fn finish_transaction<T>(
    tx: DatabaseTransaction,
    result: Result<T, WorkerFailure>,
    operation: &str,
) -> Result<T, WorkerFailure> {
    match result {
        Ok(value) => {
            tx.commit()
                .await
                .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
            Ok(value)
        }
        Err(error) => {
            tx.rollback().await.map_err(|rollback_error| {
                WorkerFailure::unavailable(format!(
                    "{operation}失败：{}；事务回滚失败：{rollback_error}",
                    error.message
                ))
            })?;
            Err(error)
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
