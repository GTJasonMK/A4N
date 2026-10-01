//! PostgreSQL-backed persisted Blueprint read worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_api_contract::blueprint::{
    BlueprintDiffSummary, BlueprintSummary, BlueprintVersionSummary, DiffBlueprintVersionsInput,
    ListBlueprintVersionsInput,
};
use a4n_blueprint_worker_contract::{
    BLUEPRINT_DIFF_OPERATION, BLUEPRINT_READ_CAPABILITY, BLUEPRINT_SUMMARY_OPERATION,
    BLUEPRINT_VERSIONS_OPERATION, BLUEPRINT_WORKER_CONTRACT_VERSION, BLUEPRINT_WORKER_ID,
    BlueprintProjectInput,
};
use a4n_db::DbHandle;
use a4n_db_models::models::project::blueprint_version;
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_domain::project::BlueprintRecord;
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
use serde_json::{Map, Value};
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_BLUEPRINT_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_BLUEPRINT_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47654";

#[derive(Clone)]
pub struct BlueprintWorker {
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

impl BlueprintWorker {
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
            .route("/v1/commands/blueprint.records.summary", post(summary))
            .route("/v1/commands/blueprint.records.versions", post(versions))
            .route("/v1/commands/blueprint.records.diff", post(diff))
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
    BlueprintWorker::new(database, BLUEPRINT_WORKER_ID, auth_token)
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
            name: BLUEPRINT_READ_CAPABILITY.into(),
            version: BLUEPRINT_WORKER_CONTRACT_VERSION.into(),
            enabled: true,
        }],
    })
    .into_response()
}

async fn summary(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<BlueprintProjectInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.summary(request)).await
}

async fn versions(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ListBlueprintVersionsInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.versions(request)).await
}

async fn diff(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<DiffBlueprintVersionsInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.diff(request)).await
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
        request: WorkerEnvelope<BlueprintProjectInput>,
    ) -> Result<Option<BlueprintSummary>, WorkerFailure> {
        validate_request(&request, BLUEPRINT_SUMMARY_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        self.verify_permission(
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let Some(blueprint) = a4n_db::project::novel_blueprint_for_project(
            &self.database,
            request.payload.project_id,
        )
        .await
        .map_err(WorkerFailure::database)?
        else {
            return Ok(None);
        };
        let needs_review = a4n_db::project::blueprint_sources_need_review(
            &self.database,
            blueprint.project_id,
            blueprint.updated_at,
        )
        .await
        .map_err(WorkerFailure::database)?;
        Ok(Some(blueprint_summary(blueprint, needs_review)))
    }

    async fn versions(
        &self,
        request: WorkerEnvelope<ListBlueprintVersionsInput>,
    ) -> Result<Vec<BlueprintVersionSummary>, WorkerFailure> {
        validate_request(&request, BLUEPRINT_VERSIONS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        self.verify_permission(
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        a4n_db::project::blueprint_versions_by_project(&self.database, request.payload.project_id)
            .await
            .map_err(project_mutation_error)?
            .into_iter()
            .map(blueprint_version_summary)
            .collect()
    }

    async fn diff(
        &self,
        request: WorkerEnvelope<DiffBlueprintVersionsInput>,
    ) -> Result<BlueprintDiffSummary, WorkerFailure> {
        validate_request(&request, BLUEPRINT_DIFF_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        self.verify_permission(
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let left = a4n_db::project::blueprint_version_by_project_version(
            &self.database,
            request.payload.project_id,
            &request.payload.version_a,
        )
        .await
        .map_err(project_mutation_error)?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("版本 {} 不存在", request.payload.version_a))
        })?;
        let right = a4n_db::project::blueprint_version_by_project_version(
            &self.database,
            request.payload.project_id,
            &request.payload.version_b,
        )
        .await
        .map_err(project_mutation_error)?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("版本 {} 不存在", request.payload.version_b))
        })?;
        Ok(BlueprintDiffSummary {
            diff: blueprint_json_diff(&left.blueprint_data, &right.blueprint_data),
        })
    }

    async fn verify_permission(
        &self,
        project_id: i64,
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<(), WorkerFailure> {
        a4n_db::collaboration::verify_project_permission(
            &self.database,
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
    if request.lease.worker_id != BLUEPRINT_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 blueprint worker",
        ));
    }
    Ok(())
}

fn validate_project_scope<T>(
    request: &WorkerEnvelope<T>,
    project_id: i64,
) -> Result<(), WorkerFailure> {
    if project_id <= 0 || request.project_id != Some(project_id) {
        return Err(WorkerFailure::invalid_request(
            "worker scope project_id 与 payload 不一致或无效",
        ));
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

fn blueprint_summary(blueprint: BlueprintRecord, sources_need_review: bool) -> BlueprintSummary {
    BlueprintSummary {
        id: blueprint.id,
        project_id: blueprint.project_id,
        title: blueprint.title,
        genre: blueprint.genre,
        style: blueprint.style,
        tone: blueprint.tone,
        target_audience: blueprint.target_audience,
        one_sentence_summary: blueprint.one_sentence_summary,
        full_synopsis: blueprint.full_synopsis,
        world_setting: blueprint.world_setting,
        current_version: blueprint.current_version,
        total_chapters: blueprint.total_chapters,
        chapters_per_part: blueprint.chapters_per_part,
        source_review_status: if sources_need_review {
            "needs_review"
        } else {
            "not_required"
        }
        .to_string(),
    }
}

fn blueprint_version_summary(
    version: blueprint_version::Model,
) -> Result<BlueprintVersionSummary, WorkerFailure> {
    Ok(BlueprintVersionSummary {
        id: version.id,
        version: version.version,
        change_description: persisted_version_text(
            version.change_description.as_deref(),
            version.id,
            "change_description",
        )?,
        created_at: version.created_at.to_string(),
        created_by: persisted_version_text(
            version.created_by.as_deref(),
            version.id,
            "created_by",
        )?,
    })
}

fn persisted_version_text(
    value: Option<&str>,
    version_id: i64,
    field: &str,
) -> Result<String, WorkerFailure> {
    let value = value.ok_or_else(|| {
        WorkerFailure::conflict(format!("blueprint_version {version_id} 缺少 {field}"))
    })?;
    if value.is_empty() || value.trim() != value {
        return Err(WorkerFailure::conflict(format!(
            "blueprint_version {version_id} 的 {field} 非规范"
        )));
    }
    Ok(value.to_string())
}

fn blueprint_json_diff(left: &Value, right: &Value) -> Value {
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            let mut changes = Map::new();
            for (key, right_value) in right {
                match left.get(key) {
                    Some(left_value) if left_value != right_value => {
                        changes.insert(key.clone(), blueprint_json_diff(left_value, right_value));
                    }
                    None => {
                        changes.insert(key.clone(), serde_json::json!({"added": right_value}));
                    }
                    _ => {}
                }
            }
            for (key, left_value) in left {
                if !right.contains_key(key) {
                    changes.insert(key.clone(), serde_json::json!({"removed": left_value}));
                }
            }
            Value::Object(changes)
        }
        _ if left != right => serde_json::json!({"old": left, "new": right}),
        _ => Value::Null,
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
    fn database(error: impl std::fmt::Display) -> Self {
        Self::unavailable(error.to_string())
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
    (
        StatusCode::UNAUTHORIZED,
        "Blueprint worker authentication required",
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
    use super::blueprint_json_diff;
    use serde_json::json;

    #[test]
    fn diff_preserves_nested_additions_and_removals() {
        assert_eq!(
            blueprint_json_diff(
                &json!({"world":{"tone":"old","removed":true}}),
                &json!({"world":{"tone":"new","added":1}}),
            ),
            json!({"world":{"tone":{"old":"old","new":"new"},"added":{"added":1},"removed":{"removed":true}}})
        );
    }
}
