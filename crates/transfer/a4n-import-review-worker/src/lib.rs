//! PostgreSQL-backed import-review read worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_db::DbHandle;
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_domain::import_review::{ImportReviewCreateInput, ImportReviewRecord};
use a4n_import_review_core::{calculate_dedup, summary_from_parts};
use a4n_import_review_worker_contract::{
    CheckImportDedupInput, IMPORT_REVIEW_DEDUP_OPERATION, IMPORT_REVIEW_DELETE_OPERATION,
    IMPORT_REVIEW_LIST_OPERATION, IMPORT_REVIEW_MUTATION_CAPABILITY, IMPORT_REVIEW_READ_CAPABILITY,
    IMPORT_REVIEW_READ_WORKER_CONTRACT_VERSION, IMPORT_REVIEW_READ_WORKER_ID,
    IMPORT_REVIEW_REJECT_OPERATION, ImportProjectReviewsInput, ImportReviewIdInput,
    ImportReviewSummary,
};
use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerFailure, WorkerHealth, WorkerHealthStatus,
    WorkerOperationRef, WorkerOperationStatus, WorkerPrincipalKind, WorkerResponse,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_IMPORT_REVIEW_READ_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_IMPORT_REVIEW_READ_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47665";

#[derive(Clone)]
pub struct ImportReviewWorker {
    state: WorkerState,
}

#[derive(Clone)]
struct WorkerState {
    database: DbHandle,
    worker_id: Arc<str>,
    auth_token: Arc<str>,
}

mod mutations;
mod reads;

impl ImportReviewWorker {
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
            .route("/v1/commands/import_review.read.list", post(list))
            .route("/v1/commands/import_review.read.dedup", post(dedup))
            .route("/v1/commands/import_review.mutation.delete", post(delete))
            .route("/v1/commands/import_review.mutation.reject", post(reject))
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
    ImportReviewWorker::new(database, IMPORT_REVIEW_READ_WORKER_ID, auth_token)
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
        capabilities: vec![
            WorkerCapability {
                name: IMPORT_REVIEW_READ_CAPABILITY.into(),
                version: IMPORT_REVIEW_READ_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: IMPORT_REVIEW_MUTATION_CAPABILITY.into(),
                version: IMPORT_REVIEW_READ_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

async fn list(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ImportProjectReviewsInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.list(request)).await
}

async fn dedup(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<CheckImportDedupInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.dedup(request)).await
}

async fn delete(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ImportReviewIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.delete(request)).await
}

async fn reject(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ImportReviewIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.reject(request)).await
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

async fn verify_editor(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(
        database.connection(),
        project_id,
        user_id,
        ProjectAccessRole::Editor,
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
    if request.lease.worker_id != IMPORT_REVIEW_READ_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 import review read worker",
        ));
    }
    if request.principal.kind != WorkerPrincipalKind::User {
        return Err(WorkerFailure::unauthorized("仅允许用户 principal"));
    }
    Ok(())
}

fn validate_mutation_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    validate_request(request, operation)?;
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

async fn review_for_editor(
    database: &DbHandle,
    user_id: i64,
    review_id: i64,
) -> Result<ImportReviewRecord, WorkerFailure> {
    if review_id <= 0 {
        return Err(WorkerFailure::invalid_request("review_id 必须大于 0"));
    }
    let review = a4n_db::import_review::import_review_by_id(database.connection(), review_id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("导入审核不存在: review_id={review_id}"))
        })?;
    verify_editor(database, review.project_id, user_id).await?;
    Ok(review)
}

fn summary_from_record(
    review: ImportReviewRecord,
) -> Result<ImportReviewSummary, a4n_error::AppError> {
    summary_from_parts(
        review.id,
        ImportReviewCreateInput {
            project_id: review.project_id,
            filename: review.filename,
            analysis_result: review.analysis_result,
        },
        &review.review_status,
        review.created_at.to_string(),
        review.updated_at.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_summary_preserves_persisted_fields() {
        let review = ImportReviewRecord {
            id: 4,
            project_id: 8,
            filename: "chapters.md".into(),
            analysis_result: Default::default(),
            review_status: "pending".into(),
            created_at: Default::default(),
            updated_at: Default::default(),
        };
        let created_at = review.created_at.to_string();
        let summary = summary_from_record(review).unwrap();
        assert_eq!(summary.id, 4);
        assert_eq!(summary.project_id, 8);
        assert_eq!(summary.filename, "chapters.md");
        assert_eq!(summary.created_at, created_at);
    }
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

fn database_error(error: impl std::fmt::Display) -> WorkerFailure {
    WorkerFailure::unavailable(error.to_string())
}

fn mutation_error(error: a4n_db::import_review::ImportReviewMutationError) -> WorkerFailure {
    match error {
        a4n_db::import_review::ImportReviewMutationError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
        a4n_db::import_review::ImportReviewMutationError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::import_review::ImportReviewMutationError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::import_review::ImportReviewMutationError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}: {operation}"))
        }
    }
}

fn app_error(error: a4n_error::AppError) -> WorkerFailure {
    match error {
        a4n_error::AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        a4n_error::AppError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_error::AppError::Forbidden(message) | a4n_error::AppError::Unauthorized(message) => {
            WorkerFailure::unauthorized(message)
        }
        a4n_error::AppError::InvalidState { current, .. }
        | a4n_error::AppError::ConfigError(current) => WorkerFailure::conflict(current),
        error => WorkerFailure::unavailable(error.to_string()),
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
        error: Some(failure.into()),
    }
}

fn unauthorized() -> Response {
    StatusCode::UNAUTHORIZED.into_response()
}
