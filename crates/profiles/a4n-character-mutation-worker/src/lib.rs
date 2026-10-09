//! PostgreSQL-backed initial character profile mutation worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_api_contract::character::{
    AddProfileAttributeInput, AttributeCountsSummary, AttributeEventSummary, AttributeHistoryInput,
    BehaviorRecordSummary, CharacterProfileCreateResult, CharacterProfileDeleteResult,
    CharacterProfileMutationResult, CharacterProfileSummary, CheckProfileSyncConflictInput,
    DeletionMarkSummary, DiffProfileChaptersInput, ExecuteAttributeDeletionInput,
    GetProfileStateAtChapterInput, GetSnapshotInput, ListBehaviorsInput, ListDeletionMarksInput,
    ListProfilesInput, ListSnapshotsInput, ModifyProfileAttributeInput, ProfileChapterStateSummary,
    RequestAttributeDeletionInput, ResetDeletionMarksInput, SnapshotDeltaSummary,
    SnapshotReferenceSummary, SnapshotSummary,
};
use a4n_character_mutation_worker_contract::{
    AttributeEventMutationResult, CHARACTER_ATTRIBUTE_ADD_OPERATION,
    CHARACTER_ATTRIBUTE_EXECUTE_DELETION_OPERATION, CHARACTER_ATTRIBUTE_MODIFY_OPERATION,
    CHARACTER_ATTRIBUTE_REQUEST_DELETION_OPERATION, CHARACTER_ATTRIBUTE_RESET_DELETION_OPERATION,
    CHARACTER_MUTATION_CAPABILITY, CHARACTER_MUTATION_CREATE_OPERATION,
    CHARACTER_MUTATION_DELETE_OPERATION, CHARACTER_MUTATION_UPDATE_OPERATION,
    CHARACTER_MUTATION_WORKER_CONTRACT_VERSION, CHARACTER_MUTATION_WORKER_ID,
    CHARACTER_OVERVIEW_ARC_OPERATION, CHARACTER_OVERVIEW_CAPABILITY,
    CHARACTER_OVERVIEW_LIST_OPERATION, CHARACTER_OVERVIEW_PROJECT_OPERATION,
    CHARACTER_OVERVIEW_VOICE_OPERATION, CHARACTER_TIMELINE_ATTRIBUTE_OPERATION,
    CHARACTER_TIMELINE_BEHAVIOR_OPERATION, CHARACTER_TIMELINE_CAPABILITY,
    CHARACTER_TIMELINE_DELETION_OPERATION, CHARACTER_TIMELINE_DIFF_OPERATION,
    CHARACTER_TIMELINE_ROLLBACK_OPERATION, CHARACTER_TIMELINE_SNAPSHOT_BY_HASH_OPERATION,
    CHARACTER_TIMELINE_SNAPSHOTS_OPERATION, CHARACTER_TIMELINE_STATE_OPERATION,
    CHARACTER_TIMELINE_SYNC_CONFLICT_OPERATION, CharacterTimelineRollbackMutationResult,
    CreateInput, NamedProfileInput, ProfileIdInput, ProfileSyncConflictSummary,
    RollbackToChapterInput, UpdateInput,
};
use a4n_db::DbHandle;
use a4n_domain::character::{
    CharacterAttributeCategory, CharacterAttributeDeletionExecutionInput,
    CharacterAttributeDeletionRequestInput, CharacterAttributeOperation,
    CharacterAttributeValueChangeInput, CharacterTimelineSourceKind,
};
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_worker_contract::WORKER_AUTH_HEADER;
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
use serde_json::Value;
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_CHARACTER_MUTATION_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_CHARACTER_MUTATION_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47674";

#[derive(Clone)]
pub struct CharacterMutationWorker {
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

impl CharacterMutationWorker {
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
            .route("/v1/commands/character.mutation.create", post(create))
            .route("/v1/commands/character.mutation.update", post(update))
            .route(
                "/v1/commands/character.mutation.delete",
                post(delete_profile),
            )
            .route(
                "/v1/commands/character.overview.project",
                post(profile_project),
            )
            .route("/v1/commands/character.overview.list", post(list_profiles))
            .route("/v1/commands/character.overview.arc", post(character_arc))
            .route(
                "/v1/commands/character.overview.voice",
                post(character_voice),
            )
            .route(
                "/v1/commands/character.timeline.attribute_history",
                post(attribute_history),
            )
            .route(
                "/v1/commands/character.timeline.behavior_history",
                post(behavior_history),
            )
            .route(
                "/v1/commands/character.timeline.deletion_marks",
                post(deletion_marks),
            )
            .route("/v1/commands/character.timeline.snapshots", post(snapshots))
            .route(
                "/v1/commands/character.timeline.state_at_chapter",
                post(state_at_chapter),
            )
            .route(
                "/v1/commands/character.timeline.rollback",
                post(rollback_timeline),
            )
            .route(
                "/v1/commands/character.timeline.snapshot_by_hash",
                post(snapshot_by_hash),
            )
            .route(
                "/v1/commands/character.timeline.sync_conflict",
                post(sync_conflict),
            )
            .route("/v1/commands/character.timeline.diff", post(diff_timeline))
            .route("/v1/commands/character.attribute.add", post(add_attribute))
            .route(
                "/v1/commands/character.attribute.modify",
                post(modify_attribute),
            )
            .route(
                "/v1/commands/character.attribute.request_deletion",
                post(request_attribute_deletion),
            )
            .route(
                "/v1/commands/character.attribute.execute_deletion",
                post(execute_attribute_deletion),
            )
            .route(
                "/v1/commands/character.attribute.reset_deletion",
                post(reset_attribute_deletion),
            )
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
    CharacterMutationWorker::new(database, CHARACTER_MUTATION_WORKER_ID, auth_token)
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
                name: CHARACTER_MUTATION_CAPABILITY.into(),
                version: CHARACTER_MUTATION_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: CHARACTER_OVERVIEW_CAPABILITY.into(),
                version: CHARACTER_MUTATION_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: CHARACTER_TIMELINE_CAPABILITY.into(),
                version: CHARACTER_MUTATION_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

async fn create(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<CreateInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.create(request)).await
}

async fn update(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<UpdateInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.update(request)).await
}

async fn delete_profile(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProfileIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.delete_profile(request)
    })
    .await
}

async fn profile_project(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProfileIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.profile_project(request)
    })
    .await
}

async fn list_profiles(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ListProfilesInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.list_profiles(request)
    })
    .await
}

async fn character_arc(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<NamedProfileInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.character_arc(request)
    })
    .await
}

async fn character_voice(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<NamedProfileInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.character_voice(request)
    })
    .await
}

async fn attribute_history(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<AttributeHistoryInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.attribute_history(request)
    })
    .await
}

async fn behavior_history(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ListBehaviorsInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.behavior_history(request)
    })
    .await
}

async fn deletion_marks(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ListDeletionMarksInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.deletion_marks(request)
    })
    .await
}

async fn snapshots(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ListSnapshotsInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.snapshots(request)
    })
    .await
}

async fn state_at_chapter(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<GetProfileStateAtChapterInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.state_at_chapter(request)
    })
    .await
}

async fn rollback_timeline(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<RollbackToChapterInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.rollback_timeline(request)
    })
    .await
}

async fn snapshot_by_hash(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<GetSnapshotInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.snapshot_by_hash(request)
    })
    .await
}

async fn sync_conflict(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<CheckProfileSyncConflictInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.sync_conflict(request)
    })
    .await
}

async fn diff_timeline(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<DiffProfileChaptersInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.diff_timeline(request)
    })
    .await
}

async fn add_attribute(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<AddProfileAttributeInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.add_attribute(request)
    })
    .await
}

async fn modify_attribute(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ModifyProfileAttributeInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.modify_attribute(request)
    })
    .await
}

async fn request_attribute_deletion(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<RequestAttributeDeletionInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.request_attribute_deletion(request)
    })
    .await
}

async fn execute_attribute_deletion(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ExecuteAttributeDeletionInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.execute_attribute_deletion(request)
    })
    .await
}

async fn reset_attribute_deletion(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ResetDeletionMarksInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.reset_attribute_deletion(request)
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

mod access;
mod attributes;
mod overview;
mod profile_mutations;
mod projection;
mod timeline_mutations;
mod timeline_reads;

use access::{named_profile, profile_for_editor, validate_snapshot_range};
use projection::{
    attribute_event_summary, behavior_record_summary, character_profile_summary,
    character_profile_summary_with_state, deletion_mark_summary, profile_state_summary,
    snapshot_reference_summary,
};

impl WorkerState {
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
    if request.lease.worker_id != CHARACTER_MUTATION_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 character mutation worker",
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
        database.connection(),
        project_id,
        user_id,
        ProjectAccessRole::Editor,
    )
    .await
    .map(|_| ())
    .map_err(collaboration_error)
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

fn character_db_error(error: a4n_db::character::CharacterDbError) -> WorkerFailure {
    match error {
        a4n_db::character::CharacterDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::character::CharacterDbError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_db::character::CharacterDbError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}: {operation}"))
        }
        other => WorkerFailure::unavailable(other.to_string()),
    }
}

fn create_error(error: a4n_db::character::CharacterProfileCreateError) -> WorkerFailure {
    match error {
        a4n_db::character::CharacterProfileCreateError::AlreadyExists { name } => {
            WorkerFailure::conflict(format!("角色档案已存在: {name}"))
        }
        a4n_db::character::CharacterProfileCreateError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::character::CharacterProfileCreateError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
        a4n_db::character::CharacterProfileCreateError::Generation(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

fn character_profile_delete_error(
    error: a4n_db::character::CharacterProfileDeleteError,
) -> WorkerFailure {
    match error {
        a4n_db::character::CharacterProfileDeleteError::Referenced(profile_id) => {
            WorkerFailure::invalid_request(format!(
                "角色档案 {profile_id} 仍被来源快照或游戏剧本引用，不能删除"
            ))
        }
        a4n_db::character::CharacterProfileDeleteError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        other => WorkerFailure::unavailable(other.to_string()),
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
