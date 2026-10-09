//! PostgreSQL-backed persisted outline records worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_db::DbHandle;
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_outline_worker_contract::{
    ChapterOutlineAccessInput, DeleteLatestOutlineRecordsInput, OUTLINE_MUTATION_CAPABILITY,
    OUTLINE_READ_CAPABILITY, OUTLINE_WORKER_CONTRACT_VERSION, OUTLINE_WORKER_ID,
    PartOutlineAccessInput, ProjectOutlineInput, SaveChapterOutlineInput, SavePartOutlineInput,
    UpdateChapterOutlineInput,
};
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
use subtle::ConstantTimeEq;

mod mutations;
mod projection;
mod reads;

use projection::{
    chapter_outline_summary, delete_input, generation_progress_summary, normalize_update_input,
    part_outline_summary,
};

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

impl OutlineWorker {
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
    let worker = OutlineWorker::new(database, OUTLINE_WORKER_ID, auth_token);
    tracing::info!(%address, "A4N outline worker listening");
    worker.serve(address).await
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
    a4n_worker_runtime::worker_command_response(
        state.authorized(headers),
        request,
        command,
        unauthorized,
    )
    .await
}

impl WorkerState {
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
            .get(a4n_worker_contract::WORKER_AUTH_HEADER)
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
    request.validate_worker_command(operation, OUTLINE_WORKER_ID, "outline worker")
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

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        "Outline worker authentication required",
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use a4n_db::project::OutlineGenerationProgressRows;
    use a4n_db_models::models::chapter::chapter_outline;
    use a4n_outline_worker_contract::LIST_CHAPTERS_OPERATION;

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
