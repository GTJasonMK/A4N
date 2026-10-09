//! PostgreSQL-backed project membership and invitation worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_db::DbHandle;
use a4n_project_collaboration_worker_contract::{
    EmptyInput, InviteInput, InviteMemberInput, MemberInput,
    PROJECT_COLLABORATION_ACCEPT_INVITE_OPERATION, PROJECT_COLLABORATION_CAPABILITY,
    PROJECT_COLLABORATION_DECLINE_INVITE_OPERATION, PROJECT_COLLABORATION_INVITE_MEMBER_OPERATION,
    PROJECT_COLLABORATION_LEAVE_OPERATION, PROJECT_COLLABORATION_LIST_INVITES_OPERATION,
    PROJECT_COLLABORATION_LIST_MEMBERS_OPERATION, PROJECT_COLLABORATION_LIST_MY_INVITES_OPERATION,
    PROJECT_COLLABORATION_REMOVE_MEMBER_OPERATION, PROJECT_COLLABORATION_REVOKE_INVITE_OPERATION,
    PROJECT_COLLABORATION_UPDATE_ROLE_OPERATION, PROJECT_COLLABORATION_WORKER_CONTRACT_VERSION,
    PROJECT_COLLABORATION_WORKER_ID, ProjectCollaborationAck, ProjectInput, ProjectInviteInput,
    ProjectInviteSummary, ProjectMemberSummary, UpdateMemberRoleInput,
};
use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerFailure, WorkerHealth, WorkerHealthStatus,
    WorkerOperationRef, WorkerOperationStatus, WorkerPrincipalKind, WorkerResponse,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_PROJECT_COLLABORATION_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_PROJECT_COLLABORATION_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47668";

#[derive(Clone)]
pub struct ProjectCollaborationWorker {
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

impl ProjectCollaborationWorker {
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
                "/v1/commands/project.collaboration.members.list",
                post(list_members),
            )
            .route(
                "/v1/commands/project.collaboration.members.invite",
                post(invite_member),
            )
            .route(
                "/v1/commands/project.collaboration.invites.list",
                post(list_invites),
            )
            .route(
                "/v1/commands/project.collaboration.invites.list_mine",
                post(list_my_invites),
            )
            .route(
                "/v1/commands/project.collaboration.invites.accept",
                post(accept_invite),
            )
            .route(
                "/v1/commands/project.collaboration.invites.decline",
                post(decline_invite),
            )
            .route(
                "/v1/commands/project.collaboration.invites.revoke",
                post(revoke_invite),
            )
            .route(
                "/v1/commands/project.collaboration.members.update_role",
                post(update_member_role),
            )
            .route(
                "/v1/commands/project.collaboration.members.remove",
                post(remove_member),
            )
            .route(
                "/v1/commands/project.collaboration.project.leave",
                post(leave_project),
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
    ProjectCollaborationWorker::new(database, PROJECT_COLLABORATION_WORKER_ID, auth_token)
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
        capabilities: vec![WorkerCapability {
            name: PROJECT_COLLABORATION_CAPABILITY.into(),
            version: PROJECT_COLLABORATION_WORKER_CONTRACT_VERSION.into(),
            enabled: true,
        }],
    })
    .into_response()
}

async fn list_members(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.list_members(request)
    })
    .await
}

async fn invite_member(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<InviteMemberInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.invite_member(request)
    })
    .await
}

async fn list_invites(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.list_invites(request)
    })
    .await
}

async fn list_my_invites(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<EmptyInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.list_my_invites(request)
    })
    .await
}

async fn accept_invite(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<InviteInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.accept_invite(request)
    })
    .await
}

async fn decline_invite(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<InviteInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.decline_invite(request)
    })
    .await
}

async fn revoke_invite(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectInviteInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.revoke_invite(request)
    })
    .await
}

async fn update_member_role(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<UpdateMemberRoleInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.update_member_role(request)
    })
    .await
}

async fn remove_member(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<MemberInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.remove_member(request)
    })
    .await
}

async fn leave_project(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.leave_project(request)
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
    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}

fn project_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
    payload_project_id: i64,
) -> Result<i64, WorkerFailure> {
    request_validate(request, operation)?;
    let project_id = request
        .project_id
        .ok_or_else(|| WorkerFailure::invalid_request("项目 scope 缺少 project_id"))?;
    if project_id <= 0 || payload_project_id != project_id {
        return Err(WorkerFailure::invalid_request(
            "worker scope project_id 与 payload 不一致或无效",
        ));
    }
    Ok(project_id)
}

fn unscoped_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    request_validate(request, operation)?;
    if request.project_id.is_some() {
        return Err(WorkerFailure::invalid_request(
            "该协作操作不接受 project scope",
        ));
    }
    Ok(())
}

fn request_validate<T: serde::Serialize>(
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
    if request.lease.worker_id != PROJECT_COLLABORATION_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 project collaboration worker",
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

fn positive_id(field: &str, value: i64) -> Result<(), WorkerFailure> {
    if value <= 0 {
        return Err(WorkerFailure::invalid_request(format!(
            "{field} 必须大于 0"
        )));
    }
    Ok(())
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
        error: Some(failure.into()),
    }
}

fn unauthorized() -> Response {
    StatusCode::UNAUTHORIZED.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{WorkerLease, WorkerPrincipal};

    #[test]
    fn project_scope_rejects_payload_mismatch() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: PROJECT_COLLABORATION_LIST_MEMBERS_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(9),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: PROJECT_COLLABORATION_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: ProjectInput { project_id: 8 },
        };
        assert!(
            project_request(&request, PROJECT_COLLABORATION_LIST_MEMBERS_OPERATION, 8).is_err()
        );
    }
}
