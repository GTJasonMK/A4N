//! PostgreSQL-backed chapter-content worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_chapter_content_core::{
    CHAPTER_CONTENT_SYNC_WORKFLOW_KIND, attach_chapter_sync_state,
    begin_chapter_content_sync_in_transaction, chapter_index_entry, chapter_metadata,
    chapter_summary_with_metadata,
};
use a4n_chapter_content_worker_contract::{
    CHAPTER_CONTENT_CAPABILITY, CHAPTER_CONTENT_IMPORT_OPERATION, CHAPTER_CONTENT_INDEX_OPERATION,
    CHAPTER_CONTENT_LIST_OPERATION, CHAPTER_CONTENT_SUMMARY_OPERATION,
    CHAPTER_CONTENT_UPDATE_OPERATION, CHAPTER_CONTENT_WORKER_CONTRACT_VERSION,
    CHAPTER_CONTENT_WORKER_ID, ChapterContentMutationResult, ChapterIndexEntry, ChapterInput,
    ChapterMetadata, ChapterSummary, ImportChapterInput, ProjectInput, UpdateChapterContentInput,
};
use a4n_db::DbHandle;
use a4n_domain::chapter::{EditChapterContentInput, ImportChapterContentInput};
use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerHealth, WorkerHealthStatus,
};
use axum::{
    Json, Router,
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Statement, TransactionTrait,
};
use subtle::ConstantTimeEq;

mod commands;
mod errors;
mod permissions;
mod responses;
mod transactions;
mod validation;

use errors::{
    WorkerFailure, app_error, chapter_error, import_error, project_error, workflow_error,
};
use permissions::{verify_project_editor, verify_project_viewer};
use responses::unauthorized;
use transactions::finish_transaction;
use validation::{positive, user_id, validate_request};

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
            .route("/v1/commands/chapter.content.list", post(commands::list))
            .route("/v1/commands/chapter.content.index", post(commands::index))
            .route(
                "/v1/commands/chapter.content.summary",
                post(commands::summary),
            )
            .route(
                "/v1/commands/chapter.content.update",
                post(commands::update),
            )
            .route(
                "/v1/commands/chapter.content.import",
                post(commands::import_chapter),
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
                    expected_revision: input.expected_revision,
                    content: input.content,
                    title: input.title,
                },
            )
            .await
            .map_err(chapter_error)?;
            let run = begin_chapter_content_sync_in_transaction(&tx, &chapter)
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
            let run = begin_chapter_content_sync_in_transaction(&tx, &chapter)
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
        let chapters = self
            .chapters_for_viewer(user_id(&request)?, project_id)
            .await?;
        self.project_summaries(project_id, chapters).await
    }

    async fn index(
        &self,
        request: WorkerEnvelope<ProjectInput>,
    ) -> Result<Vec<ChapterIndexEntry>, WorkerFailure> {
        validate_request(&request, CHAPTER_CONTENT_INDEX_OPERATION)?;
        let project_id = positive("project_id", request.payload.project_id)?;
        let chapters = self
            .chapters_for_viewer(user_id(&request)?, project_id)
            .await?;
        let metadata = self.project_metadata(project_id, &chapters).await?;
        Ok(chapters
            .iter()
            .zip(metadata)
            .map(|(chapter, metadata)| chapter_index_entry(chapter, metadata))
            .collect())
    }

    async fn chapters_for_viewer(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<a4n_db_models::models::chapter::chapter::Model>, WorkerFailure> {
        verify_project_viewer(&self.database, project_id, user_id).await?;
        a4n_db::chapter_content::chapters_for_project(self.database.connection(), project_id)
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))
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
        if request
            .project_id
            .is_some_and(|project_id| project_id != chapter.project_id)
        {
            return Err(WorkerFailure::not_found(format!(
                "章节不属于指定项目: {chapter_id}"
            )));
        }
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

    async fn project_summaries(
        &self,
        project_id: i64,
        chapters: Vec<a4n_db_models::models::chapter::chapter::Model>,
    ) -> Result<Vec<ChapterSummary>, WorkerFailure> {
        let metadata = self.project_metadata(project_id, &chapters).await?;
        Ok(chapters
            .iter()
            .zip(metadata)
            .map(|(chapter, metadata)| chapter_summary_with_metadata(chapter, metadata))
            .collect())
    }

    async fn project_metadata(
        &self,
        project_id: i64,
        chapters: &[a4n_db_models::models::chapter::chapter::Model],
    ) -> Result<Vec<ChapterMetadata>, WorkerFailure> {
        let chapter_ids = chapters
            .iter()
            .map(|chapter| chapter.id)
            .collect::<Vec<_>>();
        let (analysis_runs, content_runs) = tokio::try_join!(
            a4n_db::workflow::latest_workflow_runs_for_chapters(
                self.database.connection(),
                project_id,
                "chapter_analysis",
                &chapter_ids,
            ),
            a4n_db::workflow::latest_workflow_runs_for_chapters(
                self.database.connection(),
                project_id,
                CHAPTER_CONTENT_SYNC_WORKFLOW_KIND,
                &chapter_ids,
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
        let mut summaries = chapters.iter().map(chapter_metadata).collect::<Vec<_>>();
        attach_chapter_sync_state(
            chapters,
            &mut summaries,
            &analysis_runs,
            &content_runs,
            checkpoints,
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
