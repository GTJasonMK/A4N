//! PostgreSQL-backed administrative application worker.

use std::{env, net::SocketAddr, sync::Arc};

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
use a4n_db_models::models::user::invite_code;
use a4n_domain::auth::{UserRole, parse_user_role};
use a4n_error::AppError;
use a4n_worker_client::WORKER_AUTH_HEADER;
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
    AdminWorker::new(database, ADMIN_WORKER_ID, auth_token)
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
                name: ADMIN_READ_CAPABILITY.into(),
                version: ADMIN_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: ADMIN_WRITE_CAPABILITY.into(),
                version: ADMIN_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
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

impl WorkerState {
    async fn require_permission(
        &self,
        request: WorkerEnvelope<AdminPermissionInput>,
    ) -> Result<OperationAck, WorkerFailure> {
        validate_request(&request, ADMIN_PERMISSION_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, request.payload.permission)
            .await?;
        Ok(OperationAck { accepted: true })
    }

    async fn is_active_admin_user(
        &self,
        request: WorkerEnvelope<AdminUserInput>,
    ) -> Result<bool, WorkerFailure> {
        validate_request(&request, ADMIN_ACTIVE_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        if caller != request.payload.user_id {
            return Err(WorkerFailure::invalid_request(
                "查询管理员状态必须绑定目标用户主体",
            ));
        }
        let user = a4n_db::auth::user_by_id(&self.database, caller)
            .await
            .map_err(app_error)?;
        Ok(user.is_active && parse_user_role(&user.role).map_err(app_error)? == UserRole::Admin)
    }

    async fn user_usage(
        &self,
        request: WorkerEnvelope<AdminUserUsageInput>,
    ) -> Result<Vec<a4n_admin_worker_contract::UsageLogSummary>, WorkerFailure> {
        validate_request(&request, ADMIN_USER_USAGE_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        a4n_db::admin::usage_by_user(
            &self.database,
            request.payload.user_id,
            request.payload.limit,
        )
        .await
        .map_err(admin_error)
    }

    async fn token_summary(
        &self,
        request: WorkerEnvelope<AdminUserInput>,
    ) -> Result<serde_json::Value, WorkerFailure> {
        validate_request(&request, ADMIN_TOKEN_SUMMARY_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        a4n_db::admin::token_summary(&self.database, request.payload.user_id)
            .await
            .map_err(admin_error)
    }

    async fn usage_trends(
        &self,
        request: WorkerEnvelope<AdminUsageTrendsInput>,
    ) -> Result<Vec<a4n_admin_worker_contract::DailyUsagePoint>, WorkerFailure> {
        validate_request(&request, ADMIN_USAGE_TRENDS_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        a4n_db::admin::usage_trends(&self.database, request.payload.input)
            .await
            .map_err(admin_error)
    }

    async fn overview(
        &self,
        request: WorkerEnvelope<AdminCallerInput>,
    ) -> Result<a4n_admin_worker_contract::AdminOverviewSummary, WorkerFailure> {
        validate_request(&request, ADMIN_OVERVIEW_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageSettings)
            .await?;
        a4n_db::admin::admin_overview_summary(&self.database)
            .await
            .map_err(admin_error)
    }

    async fn projects(
        &self,
        request: WorkerEnvelope<AdminCallerInput>,
    ) -> Result<Vec<a4n_admin_worker_contract::AdminProjectSummary>, WorkerFailure> {
        validate_request(&request, ADMIN_PROJECTS_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        a4n_db::admin::all_project_summaries(&self.database)
            .await
            .map_err(admin_error)
    }

    async fn pending_tasks(
        &self,
        request: WorkerEnvelope<AdminCallerInput>,
    ) -> Result<Vec<a4n_admin_worker_contract::TaskQueueSummary>, WorkerFailure> {
        validate_request(&request, ADMIN_PENDING_TASKS_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageSettings)
            .await?;
        a4n_db::admin::pending_tasks(&self.database)
            .await
            .map_err(admin_error)
    }

    async fn generation_analytics(
        &self,
        request: WorkerEnvelope<AdminGenerationAnalyticsInput>,
    ) -> Result<a4n_admin_worker_contract::GenerationCallAnalytics, WorkerFailure> {
        validate_request(&request, ADMIN_GENERATION_ANALYTICS_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageSettings)
            .await?;
        a4n_db::admin::generation_call_analytics(&self.database, request.payload.input)
            .await
            .map_err(admin_error)
    }

    async fn database_health(
        &self,
        request: WorkerEnvelope<AdminCallerInput>,
    ) -> Result<OperationAck, WorkerFailure> {
        validate_request(&request, ADMIN_DATABASE_HEALTH_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageSettings)
            .await?;
        a4n_db::admin::check_database_health(&self.database)
            .await
            .map_err(admin_error)?;
        Ok(OperationAck { accepted: true })
    }

    async fn users(
        &self,
        request: WorkerEnvelope<AdminCallerInput>,
    ) -> Result<Vec<a4n_admin_worker_contract::UserSummary>, WorkerFailure> {
        validate_request(&request, ADMIN_USERS_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        a4n_db::auth::list_users(&self.database)
            .await
            .map_err(app_error)
    }

    async fn invites(
        &self,
        request: WorkerEnvelope<AdminCallerInput>,
    ) -> Result<Vec<a4n_admin_worker_contract::InviteCodeSummary>, WorkerFailure> {
        validate_request(&request, ADMIN_INVITES_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        let codes = a4n_db::auth::list_invite_codes(&self.database)
            .await
            .map_err(app_error)?;
        Ok(codes.iter().map(invite_code_summary).collect())
    }

    async fn create_user(
        &self,
        request: WorkerEnvelope<AdminCreateUserInput>,
    ) -> Result<a4n_admin_worker_contract::UserSummary, WorkerFailure> {
        validate_request(&request, ADMIN_CREATE_USER_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        let input = request.payload.input;
        let user = a4n_db::auth::create_user(
            &self.database,
            &input.username,
            &input.email,
            &input.password,
            &input.role,
        )
        .await
        .map_err(app_error)?;
        a4n_db::auth::current_user_summary(&self.database, user.id)
            .await
            .map_err(app_error)
    }

    async fn create_invite(
        &self,
        request: WorkerEnvelope<AdminCreateInviteInput>,
    ) -> Result<a4n_admin_worker_contract::InviteCodeSummary, WorkerFailure> {
        validate_request(&request, ADMIN_CREATE_INVITE_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        let code = a4n_db::auth::create_invite_code(
            &self.database,
            &request.payload.input.code,
            Some(caller),
        )
        .await
        .map_err(app_error)?;
        Ok(invite_code_summary(&code))
    }

    async fn update_role(
        &self,
        request: WorkerEnvelope<AdminUpdateRoleInput>,
    ) -> Result<a4n_admin_worker_contract::UserSummary, WorkerFailure> {
        validate_request(&request, ADMIN_UPDATE_ROLE_OPERATION)?;
        let caller = self.caller(&request.principal, request.payload.caller_user_id)?;
        self.require_admin(caller, AdminPermission::ManageUsers)
            .await?;
        let user = a4n_db::auth::update_user_role(
            &self.database,
            request.payload.user_id,
            &request.payload.input.role,
        )
        .await
        .map_err(app_error)?;
        a4n_db::auth::current_user_summary(&self.database, user.id)
            .await
            .map_err(app_error)
    }

    fn caller(
        &self,
        principal: &WorkerPrincipal,
        payload_user_id: i64,
    ) -> Result<i64, WorkerFailure> {
        if principal.kind != WorkerPrincipalKind::User {
            return Err(WorkerFailure::unauthorized(
                "admin worker 仅允许 user principal",
            ));
        }
        let caller = principal
            .subject
            .parse::<i64>()
            .map_err(|_| WorkerFailure::unauthorized("用户主体不是有效用户 ID"))?;
        if caller <= 0 || caller != payload_user_id {
            return Err(WorkerFailure::unauthorized("用户主体与请求调用者不匹配"));
        }
        Ok(caller)
    }

    async fn require_admin(
        &self,
        caller_user_id: i64,
        _permission: AdminPermission,
    ) -> Result<(), WorkerFailure> {
        let user = a4n_db::auth::user_by_id(&self.database, caller_user_id)
            .await
            .map_err(app_error)?;
        if !user.is_active {
            return Err(WorkerFailure::forbidden(format!(
                "用户 {caller_user_id} 已停用"
            )));
        }
        if parse_user_role(&user.role).map_err(app_error)? != UserRole::Admin {
            return Err(WorkerFailure::forbidden("当前用户不是管理员"));
        }
        Ok(())
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.as_bytes().ct_eq(self.auth_token.as_bytes()).into())
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

fn app_error(error: impl Into<AppError>) -> WorkerFailure {
    match error.into() {
        AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        AppError::Forbidden(message) | AppError::Unauthorized(message) => {
            WorkerFailure::forbidden(message)
        }
        AppError::NotFound(message) => WorkerFailure::not_found(message),
        other => WorkerFailure::unavailable(other.to_string()),
    }
}

fn invite_code_summary(code: &invite_code::Model) -> a4n_admin_worker_contract::InviteCodeSummary {
    a4n_admin_worker_contract::InviteCodeSummary {
        id: code.id,
        code: code.code.clone(),
        created_by_user_id: code.created_by_user_id,
        used_by_user_id: code.used_by_user_id,
        created_at: Some(code.created_at.to_string()),
        used_at: code.used_at.map(|value| value.to_string()),
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
