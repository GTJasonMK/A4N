//! PostgreSQL-backed initial character profile mutation worker.

use std::{env, net::SocketAddr, sync::Arc};

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
    CharacterMutationWorker::new(database, CHARACTER_MUTATION_WORKER_ID, auth_token)
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

impl WorkerState {
    async fn create(
        &self,
        request: WorkerEnvelope<CreateInput>,
    ) -> Result<CharacterProfileCreateResult, WorkerFailure> {
        validate_request(&request, CHARACTER_MUTATION_CREATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload.0;
        verify_project_editor(&self.database, input.project_id, user_id).await?;
        let profile = a4n_db::character::create_profile(
            self.database.connection(),
            input.project_id,
            input.name.trim(),
            input.base_attributes,
        )
        .await
        .map_err(create_error)?;
        Ok(CharacterProfileCreateResult {
            profile_id: profile.id,
            warnings: Vec::new(),
        })
    }

    async fn update(
        &self,
        request: WorkerEnvelope<UpdateInput>,
    ) -> Result<CharacterProfileMutationResult, WorkerFailure> {
        validate_request(&request, CHARACTER_MUTATION_UPDATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload.0;
        verify_project_editor(&self.database, input.project_id, user_id).await?;
        let profile = a4n_db::character::update_initial_profile(
            self.database.connection(),
            input.project_id,
            input.profile_id,
            input.expected_current_state,
            input.base_attributes,
        )
        .await
        .map_err(character_db_error)?;
        Ok(CharacterProfileMutationResult {
            profile: character_profile_summary(&profile)?,
            warnings: Vec::new(),
        })
    }

    async fn delete_profile(
        &self,
        request: WorkerEnvelope<ProfileIdInput>,
    ) -> Result<CharacterProfileDeleteResult, WorkerFailure> {
        validate_request(&request, CHARACTER_MUTATION_DELETE_OPERATION)?;
        let user_id = user_id(&request)?;
        let profile =
            profile_for_editor(&self.database, user_id, request.payload.profile_id).await?;
        a4n_db::character::delete_profile_cascade_transaction(
            self.database.connection(),
            profile.id,
        )
        .await
        .map_err(character_profile_delete_error)?;
        Ok(CharacterProfileDeleteResult {
            warnings: Vec::new(),
        })
    }

    async fn profile_project(
        &self,
        request: WorkerEnvelope<ProfileIdInput>,
    ) -> Result<i64, WorkerFailure> {
        validate_request(&request, CHARACTER_OVERVIEW_PROJECT_OPERATION)?;
        let user_id = user_id(&request)?;
        let profile_id = positive(request.payload.profile_id, "profile_id")?;
        let profile = a4n_db::character::profile_by_id(self.database.connection(), profile_id)
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
            .ok_or_else(|| WorkerFailure::not_found(format!("角色档案不存在: {profile_id}")))?;
        verify_project_editor(&self.database, profile.project_id, user_id).await?;
        Ok(profile.project_id)
    }

    async fn list_profiles(
        &self,
        request: WorkerEnvelope<ListProfilesInput>,
    ) -> Result<Vec<CharacterProfileSummary>, WorkerFailure> {
        validate_request(&request, CHARACTER_OVERVIEW_LIST_OPERATION)?;
        let user_id = user_id(&request)?;
        let project_id = positive(request.payload.project_id, "project_id")?;
        verify_project_editor(&self.database, project_id, user_id).await?;
        a4n_db::character::profiles_with_timeline_state(self.database.connection(), project_id)
            .await
            .map_err(character_db_error)?
            .into_iter()
            .map(|(profile, timeline_started, editable)| {
                character_profile_summary_with_state(&profile, timeline_started, editable)
            })
            .collect()
    }

    async fn character_arc(
        &self,
        request: WorkerEnvelope<NamedProfileInput>,
    ) -> Result<Option<Value>, WorkerFailure> {
        validate_request(&request, CHARACTER_OVERVIEW_ARC_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        verify_project_editor(&self.database, input.project_id, user_id).await?;
        let profile = named_profile(&self.database, input.project_id, &input.name).await?;
        Ok(profile.character_arc)
    }

    async fn character_voice(
        &self,
        request: WorkerEnvelope<NamedProfileInput>,
    ) -> Result<Option<Value>, WorkerFailure> {
        validate_request(&request, CHARACTER_OVERVIEW_VOICE_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        verify_project_editor(&self.database, input.project_id, user_id).await?;
        let profile = named_profile(&self.database, input.project_id, &input.name).await?;
        Ok(profile.voice_fingerprint)
    }

    async fn attribute_history(
        &self,
        request: WorkerEnvelope<AttributeHistoryInput>,
    ) -> Result<Vec<AttributeEventSummary>, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_ATTRIBUTE_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let events = a4n_db::character::attribute_history(
            self.database.connection(),
            input.profile_id,
            a4n_db::character::CharacterAttributeEventHistoryFilter {
                chapter_from: input.chapter_from,
                chapter_to: input.chapter_to,
                category: input.category.map(|value| value.as_str().to_string()),
                key: input.key,
                operation: input.operation.map(|value| value.as_str().to_string()),
                limit: input.limit,
            },
        )
        .await
        .map_err(character_db_error)?;
        events.iter().map(attribute_event_summary).collect()
    }

    async fn behavior_history(
        &self,
        request: WorkerEnvelope<ListBehaviorsInput>,
    ) -> Result<Vec<BehaviorRecordSummary>, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_BEHAVIOR_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        if input.limit.is_some_and(|limit| !(1..=100).contains(&limit)) {
            return Err(WorkerFailure::invalid_request(
                "行为记录 limit 必须在 1..=100 之间",
            ));
        }
        profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let records = a4n_db::character::behavior_records_by_profile(
            self.database.connection(),
            input.profile_id,
            input.chapter_from,
            input.chapter_to,
            input.limit,
        )
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        records.iter().map(behavior_record_summary).collect()
    }

    async fn deletion_marks(
        &self,
        request: WorkerEnvelope<ListDeletionMarksInput>,
    ) -> Result<Vec<DeletionMarkSummary>, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_DELETION_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let marks = a4n_db::character::deletion_marks_by_profile(
            self.database.connection(),
            input.profile_id,
            input.category.as_ref().map(|value| value.as_str()),
            input.status,
        )
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        marks.iter().map(deletion_mark_summary).collect()
    }

    async fn snapshots(
        &self,
        request: WorkerEnvelope<ListSnapshotsInput>,
    ) -> Result<Vec<SnapshotReferenceSummary>, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_SNAPSHOTS_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        validate_snapshot_range(input.start_chapter, input.end_chapter)?;
        profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let snapshots = a4n_db::character::snapshot_references_by_profile(
            self.database.connection(),
            input.profile_id,
            input.start_chapter,
            input.end_chapter,
        )
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        snapshots.iter().map(snapshot_reference_summary).collect()
    }

    async fn state_at_chapter(
        &self,
        request: WorkerEnvelope<GetProfileStateAtChapterInput>,
    ) -> Result<ProfileChapterStateSummary, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_STATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        if input.chapter_number <= 0 {
            return Err(WorkerFailure::invalid_request("章节号必须大于 0"));
        }
        profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let snapshot = a4n_db::character::snapshot_reference_by_chapter(
            self.database.connection(),
            input.profile_id,
            input.chapter_number,
        )
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("第 {} 章没有角色快照", input.chapter_number))
        })?;
        profile_state_summary(&snapshot)
    }

    async fn rollback_timeline(
        &self,
        request: WorkerEnvelope<RollbackToChapterInput>,
    ) -> Result<CharacterTimelineRollbackMutationResult, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_ROLLBACK_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let profile = profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let result = a4n_db::character::rollback_profile_to_chapter(
            self.database.connection(),
            input.profile_id,
            input.chapter_number,
        )
        .await
        .map_err(character_db_error)?;
        Ok(CharacterTimelineRollbackMutationResult {
            project_id: profile.project_id,
            deleted_events: result.deleted_timeline.deleted_attribute_events,
            deleted_snapshots: result.deleted_timeline.deleted_snapshots,
            deleted_behaviors: result.deleted_timeline.deleted_behaviors,
            deleted_state_indexes: result.deleted_timeline.deleted_state_indexes,
            deleted_deletion_marks: result.deletion_mark_rewind.deleted,
            rewound_deletion_marks: result.deletion_mark_rewind.rewound,
            state_index_ids: result.deleted_timeline.state_index_ids,
            attribute_event_ids: result.deleted_timeline.attribute_event_ids,
        })
    }

    async fn snapshot_by_hash(
        &self,
        request: WorkerEnvelope<GetSnapshotInput>,
    ) -> Result<SnapshotSummary, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_SNAPSHOT_BY_HASH_OPERATION)?;
        let user_id = user_id(&request)?;
        let content_hash = request.payload.content_hash.trim();
        if content_hash.is_empty() {
            return Err(WorkerFailure::invalid_request("快照哈希不能为空"));
        }
        let snapshot_store =
            a4n_db::character::Sha256SnapshotStore::new(self.database.shared_connection());
        let snapshot = snapshot_store
            .load_snapshot(content_hash)
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
            .ok_or_else(|| WorkerFailure::not_found(format!("快照不存在: {content_hash}")))?;
        profile_for_editor(&self.database, user_id, snapshot.entity_id).await?;
        if snapshot.entity_type != "character_profile" {
            return Err(WorkerFailure::invalid_request(format!(
                "快照类型不匹配: expected character_profile, got {}",
                snapshot.entity_type
            )));
        }
        Ok(SnapshotSummary {
            data: snapshot.data,
            created_at: Some(snapshot.created_at.to_string()),
        })
    }

    async fn sync_conflict(
        &self,
        request: WorkerEnvelope<CheckProfileSyncConflictInput>,
    ) -> Result<ProfileSyncConflictSummary, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_SYNC_CONFLICT_OPERATION)?;
        let user_id = user_id(&request)?;
        let profile =
            profile_for_editor(&self.database, user_id, request.payload.profile_id).await?;
        let chapters = a4n_db::project::character_sync_chapters(
            self.database.connection(),
            profile.project_id,
        )
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        let max_available_chapter = chapters
            .iter()
            .map(|chapter| chapter.chapter_number)
            .max()
            .unwrap_or_default();
        let snapshots = a4n_db::character::snapshot_references_by_profile(
            self.database.connection(),
            profile.id,
            None,
            Some(max_available_chapter),
        )
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        Ok(ProfileSyncConflictSummary {
            has_conflict: profile.last_synced_chapter > max_available_chapter,
            last_synced_chapter: profile.last_synced_chapter,
            max_available_chapter,
            available_snapshot_chapters: snapshots
                .iter()
                .map(|snapshot| snapshot.chapter_number)
                .collect(),
        })
    }

    async fn diff_timeline(
        &self,
        request: WorkerEnvelope<DiffProfileChaptersInput>,
    ) -> Result<SnapshotDeltaSummary, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_DIFF_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        if input.from_chapter < 0 {
            return Err(WorkerFailure::invalid_request("起始章节号不能小于 0"));
        }
        if input.to_chapter <= 0 {
            return Err(WorkerFailure::invalid_request("目标章节号必须大于 0"));
        }
        if input.from_chapter > input.to_chapter {
            return Err(WorkerFailure::invalid_request(
                "起始章节号不能大于目标章节号",
            ));
        }
        let profile = profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let to_ref = a4n_db::character::snapshot_reference_by_chapter(
            self.database.connection(),
            profile.id,
            input.to_chapter,
        )
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("第 {} 章没有角色快照", input.to_chapter))
        })?;
        let from_ref = if input.from_chapter == 0 {
            None
        } else {
            a4n_db::character::snapshot_reference_by_chapter(
                self.database.connection(),
                profile.id,
                input.from_chapter,
            )
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
        };
        let store = a4n_db::character::Sha256SnapshotStore::new(self.database.shared_connection());
        let to_snapshot = store
            .load_snapshot(&to_ref.snapshot_hash)
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
            .ok_or_else(|| {
                WorkerFailure::not_found(format!("快照不存在: {}", to_ref.snapshot_hash))
            })?;
        let from_data = match from_ref {
            Some(from_ref) => {
                store
                    .load_snapshot(&from_ref.snapshot_hash)
                    .await
                    .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
                    .ok_or_else(|| {
                        WorkerFailure::not_found(format!("快照不存在: {}", from_ref.snapshot_hash))
                    })?
                    .data
            }
            None => a4n_domain::character::initial_snapshot_data(),
        };
        let delta = a4n_db::character::Sha256SnapshotStore::diff_snapshot_data(
            &from_data,
            &to_snapshot.data,
        )
        .map_err(|error| WorkerFailure::conflict(error.to_string()))?;
        Ok(SnapshotDeltaSummary { delta })
    }

    async fn add_attribute(
        &self,
        request: WorkerEnvelope<AddProfileAttributeInput>,
    ) -> Result<AttributeEventMutationResult, WorkerFailure> {
        validate_request(&request, CHARACTER_ATTRIBUTE_ADD_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let profile = profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let result = a4n_db::character::add_profile_attribute(
            self.database.connection(),
            CharacterAttributeValueChangeInput {
                profile_id: input.profile_id,
                chapter_number: input.chapter_number,
                category: input.category.as_str().to_string(),
                key: input.key,
                value: input.value,
                change_description: None,
                cause: input.cause,
                evidence: input.evidence,
            },
        )
        .await
        .map_err(character_db_error)?;
        Ok(AttributeEventMutationResult {
            project_id: profile.project_id,
            event_id: result.event.id,
        })
    }

    async fn modify_attribute(
        &self,
        request: WorkerEnvelope<ModifyProfileAttributeInput>,
    ) -> Result<AttributeEventMutationResult, WorkerFailure> {
        validate_request(&request, CHARACTER_ATTRIBUTE_MODIFY_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let profile = profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let result = a4n_db::character::modify_profile_attribute(
            self.database.connection(),
            CharacterAttributeValueChangeInput {
                profile_id: input.profile_id,
                chapter_number: input.chapter_number,
                category: input.category.as_str().to_string(),
                key: input.key,
                value: input.new_value,
                change_description: None,
                cause: input.cause,
                evidence: input.evidence,
            },
        )
        .await
        .map_err(character_db_error)?;
        Ok(AttributeEventMutationResult {
            project_id: profile.project_id,
            event_id: result.event.id,
        })
    }

    async fn request_attribute_deletion(
        &self,
        request: WorkerEnvelope<RequestAttributeDeletionInput>,
    ) -> Result<DeletionMarkSummary, WorkerFailure> {
        validate_request(&request, CHARACTER_ATTRIBUTE_REQUEST_DELETION_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let mark = a4n_db::character::request_attribute_deletion(
            self.database.connection(),
            CharacterAttributeDeletionRequestInput {
                profile_id: input.profile_id,
                category: input.category.as_str().to_string(),
                key: input.key,
                chapter_number: input.chapter_number,
                reason: input.reason,
                evidence: input.evidence,
            },
        )
        .await
        .map_err(character_db_error)?;
        deletion_mark_summary(&mark)
    }

    async fn execute_attribute_deletion(
        &self,
        request: WorkerEnvelope<ExecuteAttributeDeletionInput>,
    ) -> Result<AttributeEventMutationResult, WorkerFailure> {
        validate_request(&request, CHARACTER_ATTRIBUTE_EXECUTE_DELETION_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let profile = profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let result = a4n_db::character::execute_ready_attribute_deletion(
            self.database.connection(),
            CharacterAttributeDeletionExecutionInput {
                profile_id: input.profile_id,
                category: input.category.as_str().to_string(),
                key: input.key,
                event_cause: input.cause,
                evidence: input.evidence,
                chapter_number: input.chapter_number,
            },
        )
        .await
        .map_err(character_db_error)?;
        Ok(AttributeEventMutationResult {
            project_id: profile.project_id,
            event_id: result.event.id,
        })
    }

    async fn reset_attribute_deletion(
        &self,
        request: WorkerEnvelope<ResetDeletionMarksInput>,
    ) -> Result<u64, WorkerFailure> {
        validate_request(&request, CHARACTER_ATTRIBUTE_RESET_DELETION_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        profile_for_editor(&self.database, user_id, input.profile_id).await?;
        a4n_db::character::reset_deletion_marks_in(
            self.database.connection(),
            input.profile_id,
            input.category.as_str(),
            &input.key,
        )
        .await
        .map_err(character_db_error)
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

fn attribute_counts(value: &Value) -> Result<AttributeCountsSummary, WorkerFailure> {
    let count = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_object)
            .map(|attributes| attributes.len())
            .ok_or_else(|| WorkerFailure::invalid_request(format!("角色当前状态缺少字段 {key}")))
    };
    Ok(AttributeCountsSummary {
        explicit: count("explicit_attributes")?,
        implicit: count("implicit_attributes")?,
        social: count("social_attributes")?,
    })
}

fn character_profile_summary(
    profile: &a4n_db_models::models::character::protagonist_profile::Model,
) -> Result<CharacterProfileSummary, WorkerFailure> {
    character_profile_summary_with_state(profile, false, true)
}

fn character_profile_summary_with_state(
    profile: &a4n_db_models::models::character::protagonist_profile::Model,
    timeline_started: bool,
    initial_profile_editable: bool,
) -> Result<CharacterProfileSummary, WorkerFailure> {
    Ok(CharacterProfileSummary {
        id: profile.id,
        project_id: profile.project_id,
        name: profile.name.clone(),
        gender: profile.gender.clone(),
        age: profile.age.clone(),
        base_attributes: profile.base_attributes.clone(),
        current_state: profile.current_state.clone(),
        character_arc: profile.character_arc.clone(),
        voice_fingerprint: profile.voice_fingerprint.clone(),
        appearance: profile.appearance.clone(),
        background: profile.background.clone(),
        personality: profile.personality.clone(),
        last_synced_chapter: profile.last_synced_chapter,
        timeline_started,
        initial_profile_editable,
        attribute_counts: attribute_counts(&profile.current_state)?,
        created_at: Some(profile.created_at.to_string()),
        updated_at: Some(profile.updated_at.to_string()),
    })
}

async fn named_profile(
    database: &DbHandle,
    project_id: i64,
    name: &str,
) -> Result<a4n_db_models::models::character::protagonist_profile::Model, WorkerFailure> {
    let name = name.trim();
    if name.is_empty() {
        return Err(WorkerFailure::invalid_request("角色名称不能为空"));
    }
    a4n_db::character::profile_by_project_name(database.connection(), project_id, name)
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("角色不存在: project_id={project_id}, name={name}"))
        })
}

async fn profile_for_editor(
    database: &DbHandle,
    user_id: i64,
    profile_id: i64,
) -> Result<a4n_db_models::models::character::protagonist_profile::Model, WorkerFailure> {
    let profile_id = positive(profile_id, "profile_id")?;
    let profile = a4n_db::character::profile_by_id(database.connection(), profile_id)
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
        .ok_or_else(|| WorkerFailure::not_found(format!("角色档案不存在: {profile_id}")))?;
    verify_project_editor(database, profile.project_id, user_id).await?;
    Ok(profile)
}

fn validate_snapshot_range(
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
) -> Result<(), WorkerFailure> {
    if start_chapter.is_some_and(|chapter| chapter <= 0) {
        return Err(WorkerFailure::invalid_request("快照起始章节必须大于 0"));
    }
    if end_chapter.is_some_and(|chapter| chapter <= 0) {
        return Err(WorkerFailure::invalid_request("快照结束章节必须大于 0"));
    }
    if let (Some(start), Some(end)) = (start_chapter, end_chapter)
        && start > end
    {
        return Err(WorkerFailure::invalid_request(
            "快照起始章节不能大于结束章节",
        ));
    }
    Ok(())
}

fn attribute_event_summary(
    event: &a4n_db_models::models::character::attribute_event::Model,
) -> Result<AttributeEventSummary, WorkerFailure> {
    let source_kind = CharacterTimelineSourceKind::parse(&event.source_kind).ok_or_else(|| {
        WorkerFailure::conflict(format!(
            "角色时间线 source_kind 无效: {}",
            event.source_kind
        ))
    })?;
    let category = CharacterAttributeCategory::parse(&event.category)
        .ok_or_else(|| WorkerFailure::conflict(format!("角色属性分类无效: {}", event.category)))?;
    let operation = CharacterAttributeOperation::parse(&event.operation)
        .ok_or_else(|| WorkerFailure::conflict(format!("角色属性操作无效: {}", event.operation)))?;
    Ok(AttributeEventSummary {
        id: event.id,
        chapter_number: event.chapter_number,
        source_kind,
        timestamp: Some(event.timestamp.to_string()),
        category,
        key: event.key.clone(),
        operation,
        change_description: event.change_description.clone(),
        old_value: event.old_value.clone(),
        new_value: event.new_value.clone(),
        cause: event.cause.clone(),
        evidence: event.evidence.clone(),
    })
}

fn behavior_record_summary(
    record: &a4n_db_models::models::character::protagonist_behavior_record::Model,
) -> Result<BehaviorRecordSummary, WorkerFailure> {
    let source_kind = CharacterTimelineSourceKind::parse(&record.source_kind).ok_or_else(|| {
        WorkerFailure::conflict(format!(
            "角色时间线 source_kind 无效: {}",
            record.source_kind
        ))
    })?;
    Ok(BehaviorRecordSummary {
        id: record.id,
        chapter_number: record.chapter_number,
        source_kind,
        behavior_description: record.behavior_description.clone(),
        behavior_type: record.behavior_type.clone(),
        original_text: record.original_text.clone(),
        behavior_tags: record.behavior_tags.clone(),
        classification_results: record.classification_results.clone(),
        related_characters: record.related_characters.clone(),
        impact_analysis: record.impact_analysis.clone(),
    })
}

fn deletion_mark_summary(
    mark: &a4n_db_models::models::character::protagonist_deletion_mark::Model,
) -> Result<DeletionMarkSummary, WorkerFailure> {
    let source_kind = CharacterTimelineSourceKind::parse(&mark.source_kind).ok_or_else(|| {
        WorkerFailure::conflict(format!("角色时间线 source_kind 无效: {}", mark.source_kind))
    })?;
    let category = CharacterAttributeCategory::parse(&mark.category)
        .ok_or_else(|| WorkerFailure::conflict(format!("角色属性分类无效: {}", mark.category)))?;
    Ok(DeletionMarkSummary {
        id: mark.id,
        source_kind,
        category,
        key: mark.key.clone(),
        reason: mark.reason.clone(),
        evidence: mark.evidence.clone(),
        consecutive_count: mark.consecutive_count,
        last_marked_chapter: mark.last_marked_chapter,
        is_executed: mark.is_executed,
        created_at: mark.created_at.to_string(),
    })
}

fn snapshot_data(
    snapshot: &a4n_db_models::models::character::snapshot_reference::Model,
) -> Result<
    (
        Value,
        AttributeCountsSummary,
        Option<String>,
        Option<String>,
        usize,
        usize,
    ),
    WorkerFailure,
> {
    let attributes =
        a4n_db::character::snapshot_attributes_from_data(&snapshot.data, snapshot.chapter_number)
            .map_err(character_db_error)?;
    let counts = attribute_counts(&attributes)?;
    let label = format!("第 {} 章角色快照", snapshot.chapter_number);
    let count = |name: &str| {
        let value = snapshot
            .data
            .get(name)
            .and_then(Value::as_u64)
            .ok_or_else(|| WorkerFailure::conflict(format!("{label}字段 {name} 必须是非负整数")))?;
        usize::try_from(value)
            .map_err(|_| WorkerFailure::conflict(format!("{label}字段 {name} 超出 usize 范围")))
    };
    Ok((
        attributes,
        counts,
        snapshot
            .data
            .get("state_summary")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        snapshot
            .data
            .get("current_location")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        count("changes_in_chapter")?,
        count("behaviors_in_chapter")?,
    ))
}

fn snapshot_reference_summary(
    snapshot: &a4n_db_models::models::character::snapshot_reference::Model,
) -> Result<SnapshotReferenceSummary, WorkerFailure> {
    let (_, counts, state_summary, _, changes, behaviors) = snapshot_data(snapshot)?;
    Ok(SnapshotReferenceSummary {
        id: snapshot.id,
        chapter_number: snapshot.chapter_number,
        snapshot_hash: snapshot.snapshot_hash.clone(),
        attribute_counts: counts,
        state_summary,
        changes_in_chapter: changes,
        behaviors_in_chapter: behaviors,
    })
}

fn profile_state_summary(
    snapshot: &a4n_db_models::models::character::snapshot_reference::Model,
) -> Result<ProfileChapterStateSummary, WorkerFailure> {
    let (attributes, counts, state_summary, current_location, changes, behaviors) =
        snapshot_data(snapshot)?;
    let category = |name: &str| {
        attributes
            .get(name)
            .and_then(Value::as_object)
            .map(|object| Value::Object(object.clone()))
            .ok_or_else(|| WorkerFailure::conflict(format!("角色快照属性缺少分类 {name}")))
    };
    Ok(ProfileChapterStateSummary {
        chapter_number: snapshot.chapter_number,
        attribute_counts: counts,
        explicit_attributes: category("explicit_attributes")?,
        implicit_attributes: category("implicit_attributes")?,
        social_attributes: category("social_attributes")?,
        state_summary,
        current_location,
        changes_in_chapter: changes,
        behaviors_in_chapter: behaviors,
        created_at: Some(snapshot.created_at.to_string()),
    })
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
