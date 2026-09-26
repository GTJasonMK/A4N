//! PostgreSQL-backed portrait record read worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_api_contract::portrait::{PortraitSummary, PortraitType};
use a4n_db::DbHandle;
use a4n_domain::character::{CharacterPortraitProfile, profile_portrait_seed_prompt};
use a4n_portrait_worker_contract::{
    PORTRAIT_BY_ID_OPERATION, PORTRAIT_LIST_OPERATION, PORTRAIT_MISSING_SEEDS_OPERATION,
    PORTRAIT_MUTATION_CAPABILITY, PORTRAIT_PRIMARY_OPERATION, PORTRAIT_READ_CAPABILITY,
    PORTRAIT_SET_PRIMARY_OPERATION, PORTRAIT_WORKER_CONTRACT_VERSION, PORTRAIT_WORKER_ID,
    PortraitGenerationSeedWire, PortraitLookupResponse, PortraitProfileInput, PortraitProjectInput,
    PortraitRecordInput,
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
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_PORTRAIT_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_PORTRAIT_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47659";

#[derive(Clone)]
pub struct PortraitWorker {
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

impl PortraitWorker {
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
            .route("/v1/commands/portrait.records.list", post(list))
            .route("/v1/commands/portrait.records.primary", post(primary))
            .route("/v1/commands/portrait.records.by_id", post(by_id))
            .route(
                "/v1/commands/portrait.records.set_primary",
                post(set_primary),
            )
            .route(
                "/v1/commands/portrait.records.missing_seeds",
                post(missing_seeds),
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
    let database = DbHandle::connect(database_url).await?;
    PortraitWorker::new(database, PORTRAIT_WORKER_ID, auth_token)
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
        capabilities: vec![
            WorkerCapability {
                name: PORTRAIT_READ_CAPABILITY.into(),
                version: PORTRAIT_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: PORTRAIT_MUTATION_CAPABILITY.into(),
                version: PORTRAIT_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

async fn list(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<PortraitProjectInput>>,
) -> Response {
    command_response(&state, headers, request, |state, request| async move {
        state.list(request).await
    })
    .await
}

async fn primary(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<PortraitProfileInput>>,
) -> Response {
    command_response(&state, headers, request, |state, request| async move {
        state.primary(request).await
    })
    .await
}

async fn by_id(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<PortraitRecordInput>>,
) -> Response {
    command_response(&state, headers, request, |state, request| async move {
        state.by_id(request).await
    })
    .await
}

async fn set_primary(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<PortraitRecordInput>>,
) -> Response {
    command_response(&state, headers, request, |state, request| async move {
        state.set_primary(request).await
    })
    .await
}

async fn missing_seeds(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<PortraitProjectInput>>,
) -> Response {
    command_response(&state, headers, request, |state, request| async move {
        state.missing_seeds(request).await
    })
    .await
}

async fn command_response<I, O, F, Fut>(
    state: &WorkerState,
    headers: HeaderMap,
    request: WorkerEnvelope<I>,
    command: F,
) -> Response
where
    I: serde::Serialize + Clone,
    O: serde::Serialize,
    F: FnOnce(WorkerState, WorkerEnvelope<I>) -> Fut,
    Fut: std::future::Future<Output = Result<O, WorkerFailure>>,
{
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match command(state.clone(), request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

impl WorkerState {
    async fn list(
        &self,
        request: WorkerEnvelope<PortraitProjectInput>,
    ) -> Result<Vec<PortraitSummary>, WorkerFailure> {
        validate_request(&request, PORTRAIT_LIST_OPERATION)?;
        let project_id = scoped_project_id(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_permission(
            self,
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Viewer,
        )
        .await?;
        a4n_db::character::character_portrait_records_by_project(&self.database, project_id)
            .await
            .map_err(database_error)?
            .iter()
            .map(portrait_summary)
            .collect()
    }

    async fn primary(
        &self,
        request: WorkerEnvelope<PortraitProfileInput>,
    ) -> Result<PortraitLookupResponse, WorkerFailure> {
        validate_request(&request, PORTRAIT_PRIMARY_OPERATION)?;
        let profile_id = request.payload.profile_id;
        if profile_id <= 0 {
            return Err(WorkerFailure::invalid_request("profile_id 必须大于 0"));
        }
        let project_id =
            a4n_db::project::protagonist_profile_project_id(&self.database, profile_id)
                .await
                .map_err(project_error)?;
        let user_id = user_id(&request)?;
        verify_permission(
            self,
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Viewer,
        )
        .await?;
        let portrait =
            a4n_db::character::primary_character_portrait_record(&self.database, profile_id)
                .await
                .map_err(database_error)?;
        Ok(PortraitLookupResponse {
            portrait: portrait.as_ref().map(portrait_summary).transpose()?,
        })
    }

    async fn by_id(
        &self,
        request: WorkerEnvelope<PortraitRecordInput>,
    ) -> Result<PortraitLookupResponse, WorkerFailure> {
        validate_request(&request, PORTRAIT_BY_ID_OPERATION)?;
        let portrait_id = request.payload.portrait_id;
        if portrait_id <= 0 {
            return Err(WorkerFailure::invalid_request("portrait_id 必须大于 0"));
        }
        let Some(portrait) =
            a4n_db::character::character_portrait_by_id(&self.database, portrait_id)
                .await
                .map_err(database_error)?
        else {
            return Ok(PortraitLookupResponse { portrait: None });
        };
        let user_id = user_id(&request)?;
        verify_permission(
            self,
            portrait.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        Ok(PortraitLookupResponse {
            portrait: Some(portrait_summary(&portrait)?),
        })
    }

    async fn missing_seeds(
        &self,
        request: WorkerEnvelope<PortraitProjectInput>,
    ) -> Result<Vec<PortraitGenerationSeedWire>, WorkerFailure> {
        validate_request(&request, PORTRAIT_MISSING_SEEDS_OPERATION)?;
        let project_id = scoped_project_id(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_permission(
            self,
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        a4n_db::character::missing_portrait_profiles(&self.database, project_id)
            .await
            .map_err(database_error)?
            .iter()
            .map(|profile| {
                profile_portrait_seed_prompt(&portrait_profile(profile))
                    .map(|prompt| PortraitGenerationSeedWire {
                        profile_name: profile.name.clone(),
                        prompt,
                    })
                    .map_err(WorkerFailure::invalid_request)
            })
            .collect()
    }

    async fn set_primary(
        &self,
        request: WorkerEnvelope<PortraitRecordInput>,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, PORTRAIT_SET_PRIMARY_OPERATION)?;
        let portrait_id = request.payload.portrait_id;
        if portrait_id <= 0 {
            return Err(WorkerFailure::invalid_request("portrait_id 必须大于 0"));
        }
        let portrait = a4n_db::character::character_portrait_by_id(&self.database, portrait_id)
            .await
            .map_err(database_error)?
            .ok_or_else(|| WorkerFailure::not_found("立绘不存在"))?;
        let user_id = user_id(&request)?;
        verify_permission(
            self,
            portrait.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        a4n_db::character::set_primary_character_portrait_record(&self.database, portrait_id)
            .await
            .map_err(portrait_record_error)
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}

async fn verify_permission(
    state: &WorkerState,
    project_id: i64,
    user_id: i64,
    role: a4n_domain::collaboration::ProjectAccessRole,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(&state.database, project_id, user_id, role)
        .await
        .map_err(collaboration_error)
        .map(|_| ())
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
    if request.lease.worker_id != PORTRAIT_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 portrait worker",
        ));
    }
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

fn portrait_profile(
    profile: &a4n_db_models::models::character::protagonist_profile::Model,
) -> CharacterPortraitProfile {
    CharacterPortraitProfile {
        name: profile.name.clone(),
        gender: profile.gender.clone(),
        age: profile.age.clone(),
        appearance: profile.appearance.clone(),
        background: profile.background.clone(),
        personality: profile.personality.clone(),
        base_attributes: profile.base_attributes.clone(),
        current_state: profile.current_state.clone(),
        character_arc: profile.character_arc.clone(),
        voice_fingerprint: profile.voice_fingerprint.clone(),
    }
}

fn portrait_summary(
    portrait: &a4n_db_models::models::character::character_portrait::Model,
) -> Result<PortraitSummary, WorkerFailure> {
    let portrait_type =
        PortraitType::try_from(portrait.portrait_type.as_str()).map_err(|error| {
            WorkerFailure::conflict(format!("读取立绘 #{} 失败: {error}", portrait.id))
        })?;
    Ok(PortraitSummary {
        id: portrait.id,
        profile_id: portrait.profile_id,
        project_id: portrait.project_id,
        portrait_type,
        image_path: portrait.image_path.clone(),
        prompt: portrait.prompt.clone(),
        expression: portrait.expression.clone(),
        pose: portrait.pose.clone(),
        is_primary: portrait.is_primary,
        created_at: portrait.created_at.to_string(),
    })
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

fn project_error(error: a4n_db::project::ProjectFamilyDbError) -> WorkerFailure {
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

fn database_error(error: impl std::fmt::Display) -> WorkerFailure {
    WorkerFailure::unavailable(error.to_string())
}

fn portrait_record_error(error: a4n_db::character::CharacterPortraitRecordError) -> WorkerFailure {
    match error {
        a4n_db::character::CharacterPortraitRecordError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::character::CharacterPortraitRecordError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::character::CharacterPortraitRecordError::Database(error) => {
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
    (
        StatusCode::UNAUTHORIZED,
        "Portrait worker authentication required",
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
    use super::{WorkerEnvelope, validate_request};
    use a4n_portrait_worker_contract::{
        PORTRAIT_LIST_OPERATION, PORTRAIT_WORKER_ID, PortraitProjectInput,
    };
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn request_contract_rejects_unknown_operation() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: "unknown".into(),
            idempotency_key: "key".into(),
            project_id: Some(1),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: PORTRAIT_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: PortraitProjectInput { project_id: 1 },
        };
        assert!(validate_request(&request, PORTRAIT_LIST_OPERATION).is_err());
    }
}
