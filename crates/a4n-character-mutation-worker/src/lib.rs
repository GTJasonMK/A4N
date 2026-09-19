//! PostgreSQL-backed initial character profile mutation worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_api_contract::character::{
    AttributeCountsSummary, CharacterProfileCreateResult, CharacterProfileMutationResult,
    CharacterProfileSummary, ListProfilesInput,
};
use a4n_character_mutation_worker_contract::{
    CHARACTER_MUTATION_CAPABILITY, CHARACTER_MUTATION_CREATE_OPERATION,
    CHARACTER_MUTATION_UPDATE_OPERATION, CHARACTER_MUTATION_WORKER_CONTRACT_VERSION,
    CHARACTER_MUTATION_WORKER_ID, CHARACTER_OVERVIEW_ARC_OPERATION, CHARACTER_OVERVIEW_CAPABILITY,
    CHARACTER_OVERVIEW_LIST_OPERATION, CHARACTER_OVERVIEW_PROJECT_OPERATION,
    CHARACTER_OVERVIEW_VOICE_OPERATION, CreateInput, NamedProfileInput, ProfileIdInput,
    UpdateInput,
};
use a4n_db::DbHandle;
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
                "/v1/commands/character.overview.project",
                post(profile_project),
            )
            .route("/v1/commands/character.overview.list", post(list_profiles))
            .route("/v1/commands/character.overview.arc", post(character_arc))
            .route(
                "/v1/commands/character.overview.voice",
                post(character_voice),
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
        .map_err(character_error)?;
        Ok(CharacterProfileMutationResult {
            profile: character_profile_summary(&profile)?,
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

fn character_error(error: a4n_db::character::CharacterDbError) -> WorkerFailure {
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
