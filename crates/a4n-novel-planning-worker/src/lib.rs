//! PostgreSQL-backed shooting-script CRUD worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_api_contract::shooting_script::{
    ChapterShootingScriptRecordSummary, ChapterShootingScriptSourceInput,
    ChapterShootingScriptView, DeleteChapterShootingScriptInput, EditChapterShootingScriptInput,
    EditedChapterShootingScriptView,
};
use a4n_db::{DbHandle, chapter_visual_plan::ChapterVisualPlanDbError};
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_novel_planning_worker_contract::{
    DELETE_SHOOTING_SCRIPT_OPERATION, EDIT_SHOOTING_SCRIPT_OPERATION,
    GET_CURRENT_SHOOTING_SCRIPT_OPERATION, NOVEL_PLANNING_WORKER_CONTRACT_VERSION,
    NOVEL_PLANNING_WORKER_ID, SHOOTING_SCRIPT_CURRENT_CAPABILITY,
    SHOOTING_SCRIPT_DELETE_CAPABILITY, SHOOTING_SCRIPT_EDIT_CAPABILITY,
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
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Statement};
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_NOVEL_PLANNING_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_NOVEL_PLANNING_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47651";

#[derive(Clone)]
pub struct NovelPlanningWorker {
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

impl NovelPlanningWorker {
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
                "/v1/commands/novel_planning.shooting_script.current",
                post(current),
            )
            .route(
                "/v1/commands/novel_planning.shooting_script.edit",
                post(edit),
            )
            .route(
                "/v1/commands/novel_planning.shooting_script.delete",
                post(delete),
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
    let worker = NovelPlanningWorker::new(database, NOVEL_PLANNING_WORKER_ID, auth_token);
    tracing::info!(%address, "A4N novel-planning worker listening");
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
        capabilities: vec![
            WorkerCapability {
                name: SHOOTING_SCRIPT_CURRENT_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: SHOOTING_SCRIPT_EDIT_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: SHOOTING_SCRIPT_DELETE_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

async fn current(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterShootingScriptSourceInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.current(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

async fn edit(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<EditChapterShootingScriptInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.edit(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

async fn delete(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<DeleteChapterShootingScriptInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.delete(request.clone()).await {
        Ok(()) => completed(&request, ()),
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

    async fn current(
        &self,
        request: WorkerEnvelope<ChapterShootingScriptSourceInput>,
    ) -> Result<Option<ChapterShootingScriptView>, WorkerFailure> {
        validate_request(&request, GET_CURRENT_SHOOTING_SCRIPT_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_family_permission(
            self.database.connection(),
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let current = self.current_record(&request.payload).await?;
        Ok(current.map(|record| ChapterShootingScriptView {
            record: record_summary(record.record),
            script: record.script,
        }))
    }

    async fn edit(
        &self,
        request: WorkerEnvelope<EditChapterShootingScriptInput>,
    ) -> Result<EditedChapterShootingScriptView, WorkerFailure> {
        validate_request(&request, EDIT_SHOOTING_SCRIPT_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        validate_target_revision(&request, request.payload.expected_revision)?;
        let user_id = user_id(&request)?;
        verify_family_permission(
            self.database.connection(),
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let current = self
            .current_record(&ChapterShootingScriptSourceInput {
                project_id: request.payload.project_id,
                source_chapter_id: request.payload.source_chapter_id,
                visual_plan_id: None,
            })
            .await?
            .ok_or_else(|| {
                WorkerFailure::not_found(format!(
                    "项目 {} 的章节 {} 未找到与当前正文版本匹配的结构化拍摄剧本；请先生成",
                    request.payload.project_id, request.payload.source_chapter_id
                ))
            })?;
        ensure_record_id(current.record.id, request.payload.record_id)?;
        let outcome = a4n_db::shooting_script::edit_chapter_shooting_script_and_invalidate(
            self.database.connection(),
            a4n_domain::shooting_script::EditStoredChapterShootingScriptInput {
                record_id: request.payload.record_id,
                expected_revision: request.payload.expected_revision,
                script: request.payload.script,
            },
        )
        .await
        .map_err(shooting_script_error)?;
        Ok(EditedChapterShootingScriptView {
            shooting_script: ChapterShootingScriptView {
                record: record_summary(outcome.shooting_script.record),
                script: outcome.shooting_script.script,
            },
            invalidated_project_ids: outcome.invalidated_project_ids,
        })
    }

    async fn delete(
        &self,
        request: WorkerEnvelope<DeleteChapterShootingScriptInput>,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, DELETE_SHOOTING_SCRIPT_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        validate_target_revision(&request, request.payload.expected_revision)?;
        let user_id = user_id(&request)?;
        verify_family_permission(
            self.database.connection(),
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let current = self
            .current_record(&ChapterShootingScriptSourceInput {
                project_id: request.payload.project_id,
                source_chapter_id: request.payload.source_chapter_id,
                visual_plan_id: None,
            })
            .await?
            .ok_or_else(|| {
                WorkerFailure::not_found(format!(
                    "项目 {} 的章节 {} 未找到与当前正文版本匹配的结构化拍摄剧本；请先生成",
                    request.payload.project_id, request.payload.source_chapter_id
                ))
            })?;
        ensure_record_id(current.record.id, request.payload.record_id)?;
        a4n_db::shooting_script::delete_chapter_shooting_script_and_invalidate(
            self.database.connection(),
            a4n_db::shooting_script::DeleteStoredChapterShootingScriptInput {
                record_id: request.payload.record_id,
                expected_revision: request.payload.expected_revision,
            },
        )
        .await
        .map_err(shooting_script_error)
    }

    async fn current_record(
        &self,
        input: &ChapterShootingScriptSourceInput,
    ) -> Result<Option<a4n_db::shooting_script::ParsedChapterShootingScript>, WorkerFailure> {
        let source = a4n_db::project::resolve_chapter_source(
            self.database.connection(),
            input.project_id,
            input.source_chapter_id,
        )
        .await
        .map_err(source_error)?;
        let existing = a4n_db::shooting_script::chapter_shooting_script_for_source(
            self.database.connection(),
            &source,
        )
        .await
        .map_err(shooting_script_error)?;
        let confirmed = a4n_db::chapter_visual_plan::confirmed_chapter_visual_plan_for_source(
            self.database.connection(),
            &source,
        )
        .await
        .map_err(visual_plan_error)?;
        let Some(confirmed) = confirmed else {
            return Ok(None);
        };
        if input
            .visual_plan_id
            .is_some_and(|requested| requested != confirmed.record.id)
        {
            return Err(WorkerFailure::invalid_request(format!(
                "visual_plan_id 不是当前正文的 confirmed 方案: requested={:?}, current={}",
                input.visual_plan_id, confirmed.record.id
            )));
        }
        Ok(existing.filter(|record| {
            record.record.visual_plan_id == Some(confirmed.record.id)
                && record.record.visual_plan_version == Some(confirmed.record.plan_version)
                && record.record.visual_plan_sha256.as_deref()
                    == Some(confirmed.record.plan_sha256.as_str())
        }))
    }
}

async fn verify_family_permission(
    db: &DatabaseConnection,
    project_id: i64,
    user_id: i64,
    required: ProjectAccessRole,
) -> Result<(), WorkerFailure> {
    let (project_id, root_project_id) =
        a4n_db::project::project_or_derived_family_access_ids(db, project_id)
            .await
            .map_err(project_family_error)?;
    if let Some(root_project_id) = root_project_id {
        a4n_db::collaboration::verify_project_permissions(
            db,
            &[project_id, root_project_id],
            user_id,
            required,
        )
        .await
        .map_err(collaboration_error)?;
    } else {
        a4n_db::collaboration::verify_project_permission(db, project_id, user_id, required)
            .await
            .map_err(collaboration_error)?;
    }
    Ok(())
}

fn validate_request<T>(request: &WorkerEnvelope<T>, operation: &str) -> Result<(), WorkerFailure> {
    request
        .validate()
        .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
    if request.operation_id != operation {
        return Err(WorkerFailure::invalid_request(format!(
            "operation_id 不匹配: expected={operation}, actual={}",
            request.operation_id
        )));
    }
    if request.lease.worker_id != NOVEL_PLANNING_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 novel-planning worker",
        ));
    }
    Ok(())
}

fn validate_project_scope<T>(
    request: &WorkerEnvelope<T>,
    project_id: i64,
) -> Result<(), WorkerFailure> {
    if request.project_id != Some(project_id) {
        return Err(WorkerFailure::invalid_request(format!(
            "worker scope project_id 与 payload 不一致: scope={:?}, payload={project_id}",
            request.project_id
        )));
    }
    Ok(())
}

fn validate_target_revision<T>(
    request: &WorkerEnvelope<T>,
    expected_revision: i32,
) -> Result<(), WorkerFailure> {
    if request.target_revision != Some(expected_revision as u64) {
        return Err(WorkerFailure::invalid_request(format!(
            "worker target_revision 与 payload 不一致: target={:?}, payload={expected_revision}",
            request.target_revision
        )));
    }
    Ok(())
}

fn user_id<T>(request: &WorkerEnvelope<T>) -> Result<i64, WorkerFailure> {
    if request.principal.kind != a4n_worker_contract::WorkerPrincipalKind::User {
        return Err(WorkerFailure::unauthorized("仅允许用户 principal"));
    }
    let user_id = request
        .principal
        .subject
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| WorkerFailure::unauthorized("用户 principal subject 无效"))?;
    Ok(user_id)
}

fn ensure_record_id(expected: i64, actual: i64) -> Result<(), WorkerFailure> {
    if expected != actual {
        return Err(WorkerFailure::invalid_request(format!(
            "拍摄剧本与当前章节来源不匹配: expected_record_id={expected}, actual_record_id={actual}"
        )));
    }
    Ok(())
}

fn record_summary(
    record: a4n_db_models::models::chapter::chapter_shooting_script::Model,
) -> ChapterShootingScriptRecordSummary {
    ChapterShootingScriptRecordSummary {
        id: record.id,
        revision: record.revision,
        visual_plan_id: record.visual_plan_id,
        visual_plan_version: record.visual_plan_version,
        visual_plan_sha256: record.visual_plan_sha256,
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

fn source_error(error: a4n_db::project::ResolveChapterSourceError) -> WorkerFailure {
    match error {
        a4n_db::project::ResolveChapterSourceError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ResolveChapterSourceError::InvalidState(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::project::ResolveChapterSourceError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::project::ResolveChapterSourceError::Database(error) => {
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

fn visual_plan_error(error: ChapterVisualPlanDbError) -> WorkerFailure {
    match error {
        ChapterVisualPlanDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        ChapterVisualPlanDbError::InvalidState(message) => WorkerFailure::conflict(message),
        ChapterVisualPlanDbError::NotFound(message) => WorkerFailure::not_found(message),
        ChapterVisualPlanDbError::RevisionConflict {
            record_id,
            expected_revision,
        } => WorkerFailure::conflict(format!(
            "chapter_visual_plan_revision_conflict:id={record_id},expected={expected_revision}"
        )),
        ChapterVisualPlanDbError::Database(error) => WorkerFailure::unavailable(error.to_string()),
    }
}

fn shooting_script_error(error: a4n_db::shooting_script::ShootingScriptDbError) -> WorkerFailure {
    match error {
        a4n_db::shooting_script::ShootingScriptDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::shooting_script::ShootingScriptDbError::InvalidState(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::shooting_script::ShootingScriptDbError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::shooting_script::ShootingScriptDbError::RevisionConflict {
            record_id,
            expected_revision,
        } => WorkerFailure::conflict(format!(
            "shooting_script_revision_conflict:id={record_id},expected={expected_revision}"
        )),
        a4n_db::shooting_script::ShootingScriptDbError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        "novel-planning worker authentication required",
    )
        .into_response()
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    fn worker() -> NovelPlanningWorker {
        NovelPlanningWorker::new(
            DatabaseConnection::Disconnected,
            NOVEL_PLANNING_WORKER_ID,
            "worker-token",
        )
    }

    #[tokio::test]
    async fn health_and_capabilities_require_authentication() {
        let router = worker().router();
        for path in [WORKER_HEALTH_PATH, WORKER_CAPABILITIES_PATH] {
            let response = router
                .clone()
                .oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
    }

    #[test]
    fn worker_address_rejects_public_bindings() {
        assert!(worker_addr_from_value("0.0.0.0:47651").is_err());
        assert!(worker_addr_from_value("127.0.0.1:0").is_err());
        assert!(worker_addr_from_value("127.0.0.1:47651").is_ok());
    }

    fn worker_addr_from_value(value: &str) -> anyhow::Result<SocketAddr> {
        let address: SocketAddr = value.parse()?;
        if !address.ip().is_loopback() || address.port() == 0 {
            anyhow::bail!("invalid worker address")
        }
        Ok(address)
    }

    #[test]
    fn revision_conflict_preserves_explicit_error_code() {
        let failure = shooting_script_error(
            a4n_db::shooting_script::ShootingScriptDbError::RevisionConflict {
                record_id: 4,
                expected_revision: 2,
            },
        );
        assert_eq!(failure.code, WorkerErrorCode::Conflict);
        assert!(
            failure
                .message
                .contains("shooting_script_revision_conflict")
        );
    }
}
