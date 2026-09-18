//! PostgreSQL-backed object profile read worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_api_contract::profile::ObjectProfileSummary;
use a4n_db::DbHandle;
use a4n_domain::image::IMAGE_TYPE_OBJECT_MATERIAL;
use a4n_object_profile_worker_contract::{
    OBJECT_PROFILE_LIST_OPERATION, OBJECT_PROFILE_READ_CAPABILITY,
    OBJECT_PROFILE_WORKER_CONTRACT_VERSION, OBJECT_PROFILE_WORKER_ID, ObjectProfileProjectInput,
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
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Statement};
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_OBJECT_PROFILE_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_OBJECT_PROFILE_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47657";

#[derive(Clone)]
pub struct ObjectProfileWorker {
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

impl ObjectProfileWorker {
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
            .route("/v1/commands/profile.object.list", post(list))
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
    ObjectProfileWorker::new(database, OBJECT_PROFILE_WORKER_ID, auth_token)
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
        capabilities: vec![WorkerCapability {
            name: OBJECT_PROFILE_READ_CAPABILITY.into(),
            version: OBJECT_PROFILE_WORKER_CONTRACT_VERSION.into(),
            enabled: true,
        }],
    })
    .into_response()
}

async fn list(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ObjectProfileProjectInput>>,
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

impl WorkerState {
    async fn list(
        &self,
        request: WorkerEnvelope<ObjectProfileProjectInput>,
    ) -> Result<Vec<ObjectProfileSummary>, WorkerFailure> {
        validate_request(&request)?;
        let project_id = request.payload.project_id;
        if request.project_id != Some(project_id) || project_id <= 0 {
            return Err(WorkerFailure::invalid_request(
                "worker scope project_id 与 payload 不一致或无效",
            ));
        }
        let user_id = user_id(&request)?;
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Viewer,
        )
        .await
        .map_err(collaboration_error)?;
        let profiles =
            a4n_db::asset::object_profiles_by_project(self.database.connection(), project_id)
                .await
                .map_err(asset_error)?;
        let profile_ids = profiles
            .iter()
            .map(|profile| profile.id)
            .collect::<Vec<_>>();
        let image_counts = a4n_db::profile_material::profile_material_image_counts_by_profile(
            self.database.connection(),
            project_id,
            IMAGE_TYPE_OBJECT_MATERIAL,
            &profile_ids,
        )
        .await
        .map_err(profile_material_error)?;
        Ok(profiles
            .iter()
            .map(|profile| {
                object_profile_summary(profile, image_counts.get(&profile.id).copied().unwrap_or(0))
            })
            .collect())
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}

fn validate_request<T: serde::Serialize>(request: &WorkerEnvelope<T>) -> Result<(), WorkerFailure> {
    request
        .validate()
        .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
    if request.operation_id != OBJECT_PROFILE_LIST_OPERATION {
        return Err(WorkerFailure::invalid_request(format!(
            "operation_id 不匹配: expected={}, actual={}",
            OBJECT_PROFILE_LIST_OPERATION, request.operation_id
        )));
    }
    if request.lease.worker_id != OBJECT_PROFILE_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 object profile worker",
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

fn object_profile_summary(
    profile: &a4n_db_models::models::asset::object_profile::Model,
    generated_image_count: i64,
) -> ObjectProfileSummary {
    ObjectProfileSummary {
        id: profile.id,
        project_id: profile.project_id,
        ownership_kind: profile.ownership_kind.clone(),
        name: profile.name.clone(),
        normalized_name: profile.normalized_name.clone(),
        aliases_json: profile.aliases_json.clone(),
        object_type: profile.object_type.clone(),
        importance: profile.importance.clone(),
        visual_description: profile.visual_description.clone(),
        material_and_state: profile.material_and_state.clone(),
        story_usage: profile.story_usage.clone(),
        visual_usage: profile.visual_usage.clone(),
        generation_usage: profile.generation_usage.clone(),
        location_context: profile.location_context.clone(),
        first_seen_chapter: profile.first_seen_chapter,
        last_seen_chapter: profile.last_seen_chapter,
        evidence_json: profile.evidence_json.clone(),
        image_prompt_seed: profile.image_prompt_seed.clone(),
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

fn asset_error(error: a4n_db::asset::AssetDbError) -> WorkerFailure {
    match error {
        a4n_db::asset::AssetDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::asset::AssetDbError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_db::asset::AssetDbError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{operation}: {current}"))
        }
        a4n_db::asset::AssetDbError::Database(error) => {
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
        "Object profile worker authentication required",
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
    use a4n_object_profile_worker_contract::{
        OBJECT_PROFILE_LIST_OPERATION, OBJECT_PROFILE_WORKER_ID, ObjectProfileProjectInput,
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
                worker_id: OBJECT_PROFILE_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ObjectProfileProjectInput { project_id: 1 },
        };
        assert!(validate_request(&request).is_err());
        assert_eq!(OBJECT_PROFILE_LIST_OPERATION, "profile.object.list");
    }
}
