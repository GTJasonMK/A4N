//! PostgreSQL-backed administrative application worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_admin_worker_contract::{
    ADMIN_ACTIVE_OPERATION, ADMIN_CREATE_INVITE_OPERATION, ADMIN_CREATE_USER_OPERATION,
    ADMIN_DATABASE_HEALTH_OPERATION, ADMIN_GENERATION_ANALYTICS_OPERATION, ADMIN_INVITES_OPERATION,
    ADMIN_OVERVIEW_OPERATION, ADMIN_PENDING_TASKS_OPERATION, ADMIN_PERMISSION_OPERATION,
    ADMIN_PROJECTS_OPERATION, ADMIN_READ_CAPABILITY, ADMIN_TOKEN_SUMMARY_OPERATION,
    ADMIN_UPDATE_ROLE_OPERATION, ADMIN_USAGE_TRENDS_OPERATION, ADMIN_USER_USAGE_OPERATION,
    ADMIN_USERS_OPERATION, ADMIN_WORKER_CONTRACT_VERSION, ADMIN_WORKER_ID, ADMIN_WRITE_CAPABILITY,
    AdminCallerInput, AdminCreateInviteInput, AdminCreateUserInput, AdminGenerationAnalyticsInput,
    AdminPermissionInput, AdminUpdateRoleInput, AdminUsageTrendsInput, AdminUserInput,
    AdminUserUsageInput, OperationAck,
};
use a4n_api_contract::admin::AdminPermission;
#[cfg(test)]
use a4n_db::DatabaseConnection;
use a4n_db::DbHandle;
use a4n_error::AppError;
use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerHealth,
    WorkerHealthStatus, WorkerOperationRef, WorkerOperationStatus, WorkerPrincipal,
    WorkerPrincipalKind, WorkerResponse,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_ADMIN_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_ADMIN_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47679";

mod auth;
mod health;
mod mutations;
mod reads;
use health::{capabilities, health};

#[derive(Clone)]
pub struct AdminWorker {
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

impl AdminWorker {
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
                "/v1/commands/admin.permission.require",
                post(require_permission),
            )
            .route("/v1/commands/admin.user.active", post(is_active_admin_user))
            .route("/v1/commands/admin.usage.user", post(user_usage))
            .route("/v1/commands/admin.token.summary", post(token_summary))
            .route("/v1/commands/admin.usage.trends", post(usage_trends))
            .route("/v1/commands/admin.overview", post(overview))
            .route("/v1/commands/admin.projects.list", post(projects))
            .route("/v1/commands/admin.tasks.pending", post(pending_tasks))
            .route(
                "/v1/commands/admin.generation.analytics",
                post(generation_analytics),
            )
            .route("/v1/commands/admin.database.health", post(database_health))
            .route("/v1/commands/admin.users.list", post(users))
            .route("/v1/commands/admin.invites.list", post(invites))
            .route("/v1/commands/admin.users.create", post(create_user))
            .route("/v1/commands/admin.invites.create", post(create_invite))
            .route("/v1/commands/admin.users.role.update", post(update_role))
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
    AdminWorker::new(database, ADMIN_WORKER_ID, auth_token)
        .serve(address)
        .await
}

macro_rules! handler {
    ($name:ident, $input:ty, $operation:expr, $method:ident) => {
        async fn $name(
            State(state): State<WorkerState>,
            headers: HeaderMap,
            Json(request): Json<WorkerEnvelope<$input>>,
        ) -> Response {
            command_response(&state, &headers, request, |request| state.$method(request)).await
        }
    };
}

handler!(
    require_permission,
    AdminPermissionInput,
    ADMIN_PERMISSION_OPERATION,
    require_permission
);
handler!(
    is_active_admin_user,
    AdminUserInput,
    ADMIN_ACTIVE_OPERATION,
    is_active_admin_user
);
handler!(
    user_usage,
    AdminUserUsageInput,
    ADMIN_USER_USAGE_OPERATION,
    user_usage
);
handler!(
    token_summary,
    AdminUserInput,
    ADMIN_TOKEN_SUMMARY_OPERATION,
    token_summary
);
handler!(
    usage_trends,
    AdminUsageTrendsInput,
    ADMIN_USAGE_TRENDS_OPERATION,
    usage_trends
);
handler!(
    overview,
    AdminCallerInput,
    ADMIN_OVERVIEW_OPERATION,
    overview
);
handler!(
    projects,
    AdminCallerInput,
    ADMIN_PROJECTS_OPERATION,
    projects
);
handler!(
    pending_tasks,
    AdminCallerInput,
    ADMIN_PENDING_TASKS_OPERATION,
    pending_tasks
);
handler!(
    generation_analytics,
    AdminGenerationAnalyticsInput,
    ADMIN_GENERATION_ANALYTICS_OPERATION,
    generation_analytics
);
handler!(
    database_health,
    AdminCallerInput,
    ADMIN_DATABASE_HEALTH_OPERATION,
    database_health
);
handler!(users, AdminCallerInput, ADMIN_USERS_OPERATION, users);
handler!(invites, AdminCallerInput, ADMIN_INVITES_OPERATION, invites);
handler!(
    create_user,
    AdminCreateUserInput,
    ADMIN_CREATE_USER_OPERATION,
    create_user
);
handler!(
    create_invite,
    AdminCreateInviteInput,
    ADMIN_CREATE_INVITE_OPERATION,
    create_invite
);
handler!(
    update_role,
    AdminUpdateRoleInput,
    ADMIN_UPDATE_ROLE_OPERATION,
    update_role
);

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
    if request.lease.worker_id != ADMIN_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 admin worker",
        ));
    }
    Ok(())
}

fn admin_error(error: a4n_db::admin::AdminDbError) -> WorkerFailure {
    match error {
        a4n_db::admin::AdminDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::admin::AdminDbError::Database(error)
        | a4n_db::admin::AdminDbError::DatabaseOperation { source: error, .. } => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

fn app_error(error: AppError) -> WorkerFailure {
    match error {
        AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        AppError::Forbidden(message) | AppError::Unauthorized(message) => {
            WorkerFailure::forbidden(message)
        }
        AppError::NotFound(message) => WorkerFailure::not_found(message),
        other => WorkerFailure::unavailable(other.to_string()),
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

    fn forbidden(message: impl Into<String>) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caller_must_be_a_matching_user_principal() {
        let state = AdminWorker {
            state: WorkerState {
                database: DbHandle::new(Arc::new(DatabaseConnection::Disconnected)),
                worker_id: ADMIN_WORKER_ID.into(),
                auth_token: "token".into(),
            },
        };
        let principal = WorkerPrincipal {
            kind: WorkerPrincipalKind::User,
            subject: "7".into(),
        };
        assert_eq!(state.state.caller(&principal, 7).unwrap(), 7);
        assert!(state.state.caller(&principal, 8).is_err());
    }
}
