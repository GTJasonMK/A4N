//! PostgreSQL-backed workflow lifecycle mutation worker.

use std::{env, net::SocketAddr, sync::Arc};

use a4n_api_contract::workflow::WorkflowCancellationResult;
use a4n_db::DbHandle;
use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerHealth,
    WorkerHealthStatus, WorkerOperationRef, WorkerOperationStatus, WorkerPrincipal,
    WorkerPrincipalKind, WorkerResponse,
};
use a4n_workflow_lifecycle_worker_contract::{
    OperationAck, ResumeWorkflowInput, WORKFLOW_LIFECYCLE_CANCEL_OPERATION,
    WORKFLOW_LIFECYCLE_CAPABILITY, WORKFLOW_LIFECYCLE_MARK_RUNNING_OPERATION,
    WORKFLOW_LIFECYCLE_RESUME_OPERATION, WORKFLOW_LIFECYCLE_WORKER_CONTRACT_VERSION,
    WORKFLOW_LIFECYCLE_WORKER_ID, WorkflowCancellationInput, WorkflowRunIdInput,
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

const WORKER_ADDR_ENV: &str = "A4N_WORKFLOW_LIFECYCLE_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_WORKFLOW_LIFECYCLE_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47678";

#[derive(Clone)]
pub struct WorkflowLifecycleWorker {
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

impl WorkflowLifecycleWorker {
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
            .route(
                "/v1/commands/workflow.lifecycle.mark_running",
                post(mark_running),
            )
            .route("/v1/commands/workflow.lifecycle.cancel", post(cancel))
            .route("/v1/commands/workflow.lifecycle.resume", post(resume))
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
    WorkflowLifecycleWorker::new(database, WORKFLOW_LIFECYCLE_WORKER_ID, auth_token)
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
        capabilities: [WORKFLOW_LIFECYCLE_CAPABILITY]
            .into_iter()
            .map(|name| WorkerCapability {
                name: name.into(),
                version: WORKFLOW_LIFECYCLE_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            })
            .collect(),
    })
    .into_response()
}

async fn mark_running(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<WorkflowRunIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.mark_running(request)
    })
    .await
}

async fn cancel(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<WorkflowCancellationInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.cancel(request)).await
}

async fn resume(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ResumeWorkflowInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.resume(request)).await
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
    async fn mark_running(
        &self,
        request: WorkerEnvelope<WorkflowRunIdInput>,
    ) -> Result<OperationAck, WorkerFailure> {
        validate_request(&request, WORKFLOW_LIFECYCLE_MARK_RUNNING_OPERATION)?;
        require_service_principal(&request.principal)?;
        a4n_db::workflow::resume_workflow_run(
            self.database.connection(),
            request.payload.workflow_run_id,
        )
        .await
        .map_err(workflow_error)?;
        Ok(OperationAck { accepted: true })
    }

    async fn cancel(
        &self,
        request: WorkerEnvelope<WorkflowCancellationInput>,
    ) -> Result<WorkflowCancellationResult, WorkerFailure> {
        validate_request(&request, WORKFLOW_LIFECYCLE_CANCEL_OPERATION)?;
        require_service_principal(&request.principal)?;
        if request.payload.reason.trim().is_empty() {
            return Err(WorkerFailure::invalid_request("取消原因不能为空"));
        }
        let result = a4n_db::workflow::cancel_workflow_run_persistence(
            self.database.connection(),
            request.payload.workflow_run_id,
            &request.payload.reason,
        )
        .await
        .map_err(workflow_error)?;
        Ok(WorkflowCancellationResult {
            workflow_run_id: result.workflow_run_id,
            active_task_id: result.active_task_id,
            interrupted_generated_video_ids: result.interrupted_generated_video_ids,
            already_terminal: result.already_terminal,
            cancelled: result.cancelled,
        })
    }

    async fn resume(
        &self,
        request: WorkerEnvelope<ResumeWorkflowInput>,
    ) -> Result<OperationAck, WorkerFailure> {
        validate_request(&request, WORKFLOW_LIFECYCLE_RESUME_OPERATION)?;
        require_service_principal(&request.principal)?;
        if request.payload.active_task_id.trim().is_empty() {
            return Err(WorkerFailure::invalid_request("active_task_id 不能为空"));
        }
        a4n_db::workflow::resume_workflow_run_with_active_task(
            self.database.connection(),
            request.payload.workflow_run_id,
            &request.payload.active_task_id,
        )
        .await
        .map_err(workflow_error)?;
        Ok(OperationAck { accepted: true })
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
    if request.lease.worker_id != WORKFLOW_LIFECYCLE_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 workflow lifecycle worker",
        ));
    }
    Ok(())
}

fn require_service_principal(principal: &WorkerPrincipal) -> Result<(), WorkerFailure> {
    if principal.kind != WorkerPrincipalKind::Service {
        return Err(WorkerFailure::unauthorized(
            "workflow lifecycle 仅允许 service principal",
        ));
    }
    Ok(())
}

fn workflow_error(error: a4n_db::workflow::WorkflowWriteError) -> WorkerFailure {
    match error {
        a4n_db::workflow::WorkflowWriteError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::workflow::WorkflowWriteError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::workflow::WorkflowWriteError::StatusConflict { current, operation } => {
            WorkerFailure::conflict(format!("当前状态 {current} 不允许执行 {operation}"))
        }
        a4n_db::workflow::WorkflowWriteError::InvalidData(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::workflow::WorkflowWriteError::Database(error) => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_rejects_user_principals() {
        let user = WorkerPrincipal {
            kind: WorkerPrincipalKind::User,
            subject: "7".into(),
        };
        let service = WorkerPrincipal {
            kind: WorkerPrincipalKind::Service,
            subject: "runtime".into(),
        };

        assert!(require_service_principal(&user).is_err());
        assert!(require_service_principal(&service).is_ok());
    }
}
