//! PostgreSQL-backed scene profile worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_api_contract::profile::{
    SceneProfileDeleteResult, SceneProfileMutationResult, SceneProfileSummary,
};
use a4n_db::DbHandle;
use a4n_domain::{
    image::IMAGE_TYPE_SCENE_MATERIAL, profile_material::resolve_downstream_image_ids,
};
use a4n_scene_profile_worker_contract::{
    CreateSceneProfileInput, DeleteSceneProfileInput, SCENE_PROFILE_CREATE_OPERATION,
    SCENE_PROFILE_DELETE_OPERATION, SCENE_PROFILE_LIST_OPERATION,
    SCENE_PROFILE_MUTATION_CAPABILITY, SCENE_PROFILE_READ_CAPABILITY,
    SCENE_PROFILE_SYNC_CAPABILITY, SCENE_PROFILE_SYNC_RECORDS_OPERATION,
    SCENE_PROFILE_UPDATE_OPERATION, SCENE_PROFILE_WORKER_CONTRACT_VERSION, SCENE_PROFILE_WORKER_ID,
    SceneProfileDeleteWorkerResult, SceneProfileMutationWorkerResult, SceneProfileProjectInput,
    SceneProfileSyncRecordsInput, SceneSyncRecordResult, UpdateSceneProfileInput,
};
use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerFailure, WorkerHealth, WorkerHealthStatus,
    WorkerResponse,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};

const WORKER_ADDR_ENV: &str = "A4N_SCENE_PROFILE_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_SCENE_PROFILE_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47658";

#[derive(Clone)]
pub struct SceneProfileWorker {
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

impl SceneProfileWorker {
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
            .route("/v1/commands/profile.scene.list", post(list))
            .route("/v1/commands/profile.scene.create", post(create))
            .route("/v1/commands/profile.scene.update", post(update))
            .route("/v1/commands/profile.scene.delete", post(delete))
            .route(
                "/v1/commands/profile.scene.sync_records",
                post(sync_records),
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
    SceneProfileWorker::new(database, SCENE_PROFILE_WORKER_ID, auth_token)
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
                name: SCENE_PROFILE_READ_CAPABILITY.into(),
                version: SCENE_PROFILE_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: SCENE_PROFILE_MUTATION_CAPABILITY.into(),
                version: SCENE_PROFILE_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: SCENE_PROFILE_SYNC_CAPABILITY.into(),
                version: SCENE_PROFILE_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

async fn list(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<SceneProfileProjectInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.list(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

async fn create(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<CreateSceneProfileInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.create(request)).await
}

async fn update(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<UpdateSceneProfileInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.update(request)).await
}

async fn delete(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<DeleteSceneProfileInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.delete(request)).await
}

async fn sync_records(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<SceneProfileSyncRecordsInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.sync_records(request)
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

fn validate_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation_id: &str,
) -> Result<(), WorkerFailure> {
    request.validate_worker_command(
        operation_id,
        SCENE_PROFILE_WORKER_ID,
        "scene profile worker",
    )
}

fn validate_project_scope<T>(
    request: &WorkerEnvelope<T>,
    project_id: i64,
) -> Result<(), WorkerFailure> {
    if project_id <= 0 || request.project_id.is_some_and(|scope| scope != project_id) {
        return Err(WorkerFailure::invalid_request(
            "worker scope project_id 与资源 project_id 不一致或无效",
        ));
    }
    Ok(())
}

fn user_id<T>(request: &WorkerEnvelope<T>) -> Result<i64, WorkerFailure> {
    request.principal.require_user_id()
}

fn scene_profile_summary(
    profile: &a4n_domain::scene_profile::SceneProfileRecord,
    occurrence_count: u64,
    generated_image_count: i64,
) -> SceneProfileSummary {
    SceneProfileSummary {
        id: profile.id,
        project_id: profile.project_id,
        ownership_kind: profile.ownership_kind.clone(),
        name: profile.name.clone(),
        normalized_name: profile.normalized_name.clone(),
        aliases_json: profile.aliases_json.clone(),
        scene_type: profile.scene_type.clone(),
        importance: profile.importance.clone(),
        parent_scene_id: profile.parent_scene_id,
        base_description: profile.base_description.clone(),
        visual_description: profile.visual_description.clone(),
        spatial_layout: profile.spatial_layout.clone(),
        atmosphere: profile.atmosphere.clone(),
        entry_points_json: profile.entry_points_json.clone(),
        rules_json: profile.rules_json.clone(),
        current_state_json: profile.current_state_json.clone(),
        first_seen_chapter: profile.first_seen_chapter,
        last_seen_chapter: profile.last_seen_chapter,
        evidence_json: profile.evidence_json.clone(),
        image_prompt_seed: profile.image_prompt_seed.clone(),
        occurrence_count,
        generated_image_count,
        created_at: Some(profile.created_at.to_string()),
        updated_at: Some(profile.updated_at.to_string()),
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

fn scene_profile_error(error: a4n_db::scene_profile::SceneProfileDbError) -> WorkerFailure {
    match error {
        a4n_db::scene_profile::SceneProfileDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::scene_profile::SceneProfileDbError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::scene_profile::SceneProfileDbError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{operation}: {current}"))
        }
        a4n_db::scene_profile::SceneProfileDbError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

fn profile_material_error(
    error: a4n_db::profile_material::ProfileMaterialDbError,
) -> WorkerFailure {
    match error {
        a4n_db::profile_material::ProfileMaterialDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::profile_material::ProfileMaterialDbError::Database { source, .. } => {
            WorkerFailure::unavailable(source.to_string())
        }
    }
}

fn completed<T>(request: &WorkerEnvelope<impl serde::Serialize>, payload: T) -> WorkerResponse<T> {
    WorkerResponse::completed_for(request, payload)
}

fn failed<T>(
    request: &WorkerEnvelope<impl serde::Serialize>,
    failure: WorkerFailure,
) -> WorkerResponse<T> {
    WorkerResponse::failed_for(request, failure.into())
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        "Scene profile worker authentication required",
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::{WorkerEnvelope, validate_request};
    use a4n_scene_profile_worker_contract::{
        SCENE_PROFILE_LIST_OPERATION, SCENE_PROFILE_WORKER_ID, SceneProfileProjectInput,
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
                worker_id: SCENE_PROFILE_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: SceneProfileProjectInput { project_id: 1 },
        };
        assert!(validate_request(&request, SCENE_PROFILE_LIST_OPERATION).is_err());
        assert_eq!(SCENE_PROFILE_LIST_OPERATION, "profile.scene.list");
    }
}
