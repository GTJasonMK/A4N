//! PostgreSQL-backed portrait record read worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_api_contract::portrait::{PortraitSummary, PortraitType};
use a4n_db::DbHandle;
use a4n_domain::character::{
    CharacterPortraitProfile, CharacterPortraitRecord, CharacterProfileRecord,
    profile_portrait_seed_prompt,
};
use a4n_portrait_worker_contract::{
    PORTRAIT_BY_ID_OPERATION, PORTRAIT_LIST_OPERATION, PORTRAIT_MISSING_SEEDS_OPERATION,
    PORTRAIT_MUTATION_CAPABILITY, PORTRAIT_PRIMARY_OPERATION, PORTRAIT_READ_CAPABILITY,
    PORTRAIT_SET_PRIMARY_OPERATION, PORTRAIT_WORKER_CONTRACT_VERSION, PORTRAIT_WORKER_ID,
    PortraitGenerationSeedWire, PortraitLookupResponse, PortraitProfileInput, PortraitProjectInput,
    PortraitRecordInput,
};
use a4n_worker_contract::WORKER_AUTH_HEADER;
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

mod mutations;
mod reads;

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
    PortraitWorker::new(database, PORTRAIT_WORKER_ID, auth_token)
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
    a4n_worker_runtime::worker_command_response(
        state.authorized(&headers),
        request,
        |request| command(state.clone(), request),
        unauthorized,
    )
    .await
}

impl WorkerState {
    fn authorized(&self, headers: &HeaderMap) -> bool {
        a4n_worker_runtime::worker_token_authorized(
            headers,
            WORKER_AUTH_HEADER,
            self.auth_token.as_bytes(),
        )
    }
}

async fn verify_permission(
    state: &WorkerState,
    project_id: i64,
    user_id: i64,
    role: a4n_domain::collaboration::ProjectAccessRole,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(
        state.database.connection(),
        project_id,
        user_id,
        role,
    )
    .await
    .map_err(a4n_worker_db_adapter::map_collaboration_db_error)
    .map(|_| ())
}

fn validate_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    request.validate_worker_command(operation, PORTRAIT_WORKER_ID, "portrait worker")
}

fn portrait_profile(profile: &CharacterProfileRecord) -> CharacterPortraitProfile {
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

fn portrait_summary(portrait: &CharacterPortraitRecord) -> Result<PortraitSummary, WorkerFailure> {
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

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        "Portrait worker authentication required",
    )
        .into_response()
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
