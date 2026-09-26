//! PostgreSQL-backed profile-material worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_api_contract::profile::{
    ListProfileMaterialsInput, ProfileMaterialGroup, SetProfilePrimaryMaterialInput,
};
use a4n_db::DbHandle;
use a4n_domain::profile_material::ProfileMaterialKind;
use a4n_profile_material_core::{
    apply_profile_material_query, image_type_for_profile_material_kind, object_source_from_model,
    profile_material_group_projections, requested_profile_material_kinds, scene_source_from_model,
};
use a4n_profile_material_worker_contract::{
    PROFILE_MATERIAL_LIST_OPERATION, PROFILE_MATERIAL_MUTATION_CAPABILITY,
    PROFILE_MATERIAL_READ_CAPABILITY, PROFILE_MATERIAL_SET_PRIMARY_OPERATION,
    PROFILE_MATERIAL_WORKER_CONTRACT_VERSION, PROFILE_MATERIAL_WORKER_ID,
    ProfileMaterialSetPrimaryResponse,
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

#[derive(Debug)]
struct WorkerFailure {
    code: WorkerErrorCode,
    message: String,
    retryable: bool,
}

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
    ProfileMaterialWorker::new(database, PROFILE_MATERIAL_WORKER_ID, auth_token)
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
    async fn list(
        &self,
        request: WorkerEnvelope<ListProfileMaterialsInput>,
    ) -> Result<Vec<ProfileMaterialGroup>, WorkerFailure> {
        validate_request(&request, PROFILE_MATERIAL_LIST_OPERATION)?;
        validate_input_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        if input.image_limit_per_profile == 0 || input.image_limit_per_profile > MAX_IMAGE_LIMIT {
            return Err(WorkerFailure::invalid_request(
                "image_limit_per_profile 必须在 1 到 32 之间",
            ));
        }
        verify_permission(
            &self.database,
            input.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        let kinds = requested_profile_material_kinds(input.kind);
        let image_types = kinds
            .iter()
            .map(|kind| image_type_for_profile_material_kind(*kind))
            .collect::<Vec<_>>();
        let include_scene = kinds.contains(&ProfileMaterialKind::Scene);
        let include_object = kinds.contains(&ProfileMaterialKind::Object);
        let (scene_profiles, object_profiles, rows) = tokio::join!(
            async {
                if include_scene {
                    a4n_db::scene_profile::scene_profiles_by_project(
                        &self.database,
                        input.project_id,
                    )
                    .await
                    .map_err(scene_profile_error)
                } else {
                    Ok(Vec::new())
                }
            },
            async {
                if include_object {
                    a4n_db::asset::object_profiles_by_project(&self.database, input.project_id)
                        .await
                        .map_err(asset_error)
                } else {
                    Ok(Vec::new())
                }
            },
            async {
                a4n_db::profile_material::profile_material_list_rows(
                    &self.database,
                    input.project_id,
                    &image_types,
                )
                .await
                .map_err(profile_material_error)
            },
        );
        let mut sources = scene_profiles?
            .into_iter()
            .map(scene_source_from_model)
            .collect::<Vec<_>>();
        sources.extend(object_profiles?.into_iter().map(object_source_from_model));
        let rows = rows?;
        let mut groups = profile_material_group_projections(
            sources,
            rows.images,
            rows.primary_rows,
            input.image_limit_per_profile,
        );
        apply_profile_material_query(&mut groups, &input);
        Ok(groups
            .into_iter()
            .map(|projection| projection.group)
            .collect())
    }

    async fn set_primary(
        &self,
        request: WorkerEnvelope<SetProfilePrimaryMaterialInput>,
    ) -> Result<ProfileMaterialSetPrimaryResponse, WorkerFailure> {
        validate_request(&request, PROFILE_MATERIAL_SET_PRIMARY_OPERATION)?;
        validate_input_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        verify_permission(
            &self.database,
            input.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.ensure_profile(input.project_id, input.profile_kind, input.profile_id)
            .await?;
        if let Some(image_id) = input.image_id {
            let image = a4n_db::video::generated_image_by_id(&self.database, image_id)
                .await
                .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
                .ok_or_else(|| WorkerFailure::not_found("图片不存在"))?;
            if image.project_id != input.project_id {
                return Err(WorkerFailure::invalid_request(format!(
                    "图片 #{image_id} 不属于项目 {}",
                    input.project_id
                )));
            }
            if a4n_db::profile_material::generated_image_profile_material_identity(&image)
                != Some((input.profile_kind.as_str(), input.profile_id))
            {
                return Err(WorkerFailure::invalid_request(format!(
                    "图片 #{image_id} 不是 {} 档案 #{} 的素材图",
                    input.profile_kind.as_str(),
                    input.profile_id
                )));
            }
            a4n_db::profile_material::upsert_profile_primary_material(
                &self.database,
                input.project_id,
                input.profile_kind.as_str(),
                input.profile_id,
                image_id,
            )
            .await
            .map_err(profile_material_error)?;
        } else {
            a4n_db::profile_material::delete_profile_primary_material(
                &self.database,
                input.project_id,
                input.profile_kind.as_str(),
                input.profile_id,
            )
            .await
            .map_err(profile_material_error)?;
        }
        let group = self
            .profile_material_group(input.project_id, input.profile_kind, input.profile_id)
            .await?;
        Ok(ProfileMaterialSetPrimaryResponse { group })
    }

    async fn ensure_profile(
        &self,
        project_id: i64,
        kind: ProfileMaterialKind,
        profile_id: i64,
    ) -> Result<(), WorkerFailure> {
        let profile_project_id = match kind {
            ProfileMaterialKind::Scene => {
                a4n_db::scene_profile::scene_profile_project_id(&self.database, profile_id)
                    .await
                    .map_err(scene_profile_error)?
            }
            ProfileMaterialKind::Object => {
                a4n_db::asset::object_profile_project_id(&self.database, profile_id)
                    .await
                    .map_err(asset_error)?
            }
        };
        if profile_project_id != project_id {
            let label = match kind {
                ProfileMaterialKind::Scene => "场景档案",
                ProfileMaterialKind::Object => "物体档案",
            };
            return Err(WorkerFailure::invalid_request(format!(
                "{label} #{profile_id} 不属于项目 {project_id}"
            )));
        }
        Ok(())
    }

    async fn profile_material_group(
        &self,
        project_id: i64,
        kind: ProfileMaterialKind,
        profile_id: i64,
    ) -> Result<ProfileMaterialGroup, WorkerFailure> {
        let image_types = [image_type_for_profile_material_kind(kind)];
        let (source, rows) = tokio::join!(
            async {
                match kind {
                    ProfileMaterialKind::Scene => {
                        a4n_db::scene_profile::scene_profile_by_id(&self.database, profile_id)
                            .await
                            .map(scene_source_from_model)
                            .map_err(scene_profile_error)
                    }
                    ProfileMaterialKind::Object => {
                        a4n_db::asset::object_profile_by_id(&self.database, profile_id)
                            .await
                            .map(object_source_from_model)
                            .map_err(asset_error)
                    }
                }
            },
            async {
                a4n_db::profile_material::profile_material_list_rows(
                    &self.database,
                    project_id,
                    &image_types,
                )
                .await
                .map_err(profile_material_error)
            },
        );
        let source = source?;
        if source.profile.project_id != project_id {
            return Err(WorkerFailure::invalid_request(format!(
                "档案 #{} 不属于项目 {project_id}",
                profile_id
            )));
        }
        let rows = rows?;
        profile_material_group_projections(vec![source], rows.images, rows.primary_rows, 32)
            .into_iter()
            .next()
            .map(|projection| projection.group)
            .ok_or_else(|| WorkerFailure::conflict("主素材已更新，但无法重新读取档案素材"))
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}

async fn verify_permission(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
    role: a4n_domain::collaboration::ProjectAccessRole,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(&database, project_id, user_id, role)
        .await
        .map(|_| ())
        .map_err(collaboration_error)
}

fn validate_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    expected_operation: &str,
) -> Result<(), WorkerFailure> {
    request
        .validate()
        .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
    if request.operation_id != expected_operation {
        return Err(WorkerFailure::invalid_request(format!(
            "operation_id 不匹配: expected={}, actual={}",
            expected_operation, request.operation_id
        )));
    }
    if request.lease.worker_id != PROFILE_MATERIAL_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 profile material worker",
        ));
    }
    Ok(())
}

fn validate_input_scope<T>(
    request: &WorkerEnvelope<T>,
    project_id: i64,
) -> Result<(), WorkerFailure> {
    if project_id <= 0 || request.project_id != Some(project_id) {
        return Err(WorkerFailure::invalid_request(
            "worker scope project_id 与 payload 不一致或无效",
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
