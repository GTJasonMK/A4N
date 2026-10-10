//! PostgreSQL-backed profile-material worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_api_contract::profile::{
    ListProfileMaterialsInput, ProfileMaterialGroup, SetProfilePrimaryMaterialInput,
};
use a4n_db::DbHandle;
use a4n_domain::profile_material::{ProfileMaterialKind, profile_material_identity};
use a4n_profile_material_core::{
    apply_profile_material_query, image_type_for_profile_material_kind, object_source_from_record,
    profile_material_group_projections, requested_profile_material_kinds, scene_source_from_record,
};
use a4n_profile_material_worker_contract::{
    PROFILE_MATERIAL_LIST_OPERATION, PROFILE_MATERIAL_MUTATION_CAPABILITY,
    PROFILE_MATERIAL_READ_CAPABILITY, PROFILE_MATERIAL_SET_PRIMARY_OPERATION,
    PROFILE_MATERIAL_WORKER_CONTRACT_VERSION, PROFILE_MATERIAL_WORKER_ID,
    ProfileMaterialSetPrimaryResponse,
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

const WORKER_ADDR_ENV: &str = "A4N_PROFILE_MATERIAL_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_PROFILE_MATERIAL_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47661";
const MAX_IMAGE_LIMIT: usize = 32;

#[derive(Clone)]
pub struct ProfileMaterialWorker {
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

impl ProfileMaterialWorker {
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
            .route("/v1/commands/profile.material.list", post(list))
            .route(
                "/v1/commands/profile.material.set_primary",
                post(set_primary),
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
    ProfileMaterialWorker::new(database, PROFILE_MATERIAL_WORKER_ID, auth_token)
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
                name: PROFILE_MATERIAL_READ_CAPABILITY.into(),
                version: PROFILE_MATERIAL_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: PROFILE_MATERIAL_MUTATION_CAPABILITY.into(),
                version: PROFILE_MATERIAL_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

async fn list(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ListProfileMaterialsInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.list(request)).await
}

async fn set_primary(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<SetProfilePrimaryMaterialInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.set_primary(request)
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
    fn authorized(&self, headers: &HeaderMap) -> bool {
        a4n_worker_runtime::worker_token_authorized(
            headers,
            WORKER_AUTH_HEADER,
            self.auth_token.as_bytes(),
        )
    }
}

async fn verify_permission(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
    role: a4n_domain::collaboration::ProjectAccessRole,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(
        database.connection(),
        project_id,
        user_id,
        role,
    )
    .await
    .map(|_| ())
    .map_err(collaboration_error)
}

fn validate_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    expected_operation: &str,
) -> Result<(), WorkerFailure> {
    request.validate_worker_command(
        expected_operation,
        PROFILE_MATERIAL_WORKER_ID,
        "profile material worker",
    )
}

fn collaboration_error(error: a4n_db::collaboration::CollaborationDbError) -> WorkerFailure {
    a4n_worker_db_adapter::map_collaboration_db_error(error)
}

fn unauthorized() -> Response {
    StatusCode::UNAUTHORIZED.into_response()
}
