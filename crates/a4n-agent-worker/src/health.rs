use std::{net::SocketAddr, sync::Arc};

use a4n_agent_application::agent::AgentApplication;
use a4n_agent_runtime_contract::{
    AGENT_THREAD_ARCHIVE_OPERATION, AGENT_THREAD_CREATE_OPERATION,
    AGENT_THREAD_ENSURE_MAIN_OPERATION, AGENT_THREAD_FORK_OPERATION, AGENT_THREAD_LIST_OPERATION,
    AGENT_THREAD_MESSAGES_OPERATION, AGENT_THREAD_SHOW_OPERATION, AGENT_THREAD_UPDATE_OPERATION,
};
use a4n_api_contract::agent::{AgentThreadMutationApplicationPort, AgentThreadReadApplicationPort};
use a4n_error::AppError;
use a4n_worker_contract::{
    AGENT_HOST_COMMAND_OPERATION, AGENT_RUN_OPERATION, AGENT_WORKER_AUTH_HEADER,
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerHealth,
    WorkerHealthStatus, WorkerOperationRef, WorkerOperationStatus, WorkerPrincipalKind,
    WorkerResponse,
};
use anyhow::anyhow;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use subtle::ConstantTimeEq;
use tokio::{net::TcpListener, sync::oneshot};

const WORKER_ADDR_ENV: &str = "A4N_AGENT_WORKER_ADDR";
const HEALTH_PATH: &str = WORKER_HEALTH_PATH;
const CAPABILITIES_PATH: &str = WORKER_CAPABILITIES_PATH;
const CAPABILITY_VERSION: &str = "agent-worker-v1";
const WORKER_OPERATIONS: &[&str] = &[
    AGENT_RUN_OPERATION,
    AGENT_HOST_COMMAND_OPERATION,
    AGENT_THREAD_ENSURE_MAIN_OPERATION,
    AGENT_THREAD_CREATE_OPERATION,
    AGENT_THREAD_FORK_OPERATION,
    AGENT_THREAD_UPDATE_OPERATION,
    AGENT_THREAD_ARCHIVE_OPERATION,
    AGENT_THREAD_LIST_OPERATION,
    AGENT_THREAD_SHOW_OPERATION,
    AGENT_THREAD_MESSAGES_OPERATION,
];

#[derive(Clone)]
struct HealthState {
    worker_id: Arc<str>,
    auth_token: Arc<str>,
    application: Option<Arc<AgentApplication>>,
}

pub(crate) fn router(
    worker_id: String,
    auth_token: String,
    application: Option<Arc<AgentApplication>>,
) -> Router {
    Router::new()
        .route(HEALTH_PATH, get(healthz))
        .route(CAPABILITIES_PATH, get(capabilities))
        .route(
            "/v1/commands/agent.thread.ensure_main",
            axum::routing::post(ensure_main_thread),
        )
        .route(
            "/v1/commands/agent.thread.create",
            axum::routing::post(create_thread),
        )
        .route(
            "/v1/commands/agent.thread.fork",
            axum::routing::post(fork_thread),
        )
        .route(
            "/v1/commands/agent.thread.update",
            axum::routing::post(update_thread),
        )
        .route(
            "/v1/commands/agent.thread.archive",
            axum::routing::post(archive_thread),
        )
        .route(
            "/v1/commands/agent.thread.list",
            axum::routing::post(list_threads),
        )
        .route(
            "/v1/commands/agent.thread.show",
            axum::routing::post(show_thread),
        )
        .route(
            "/v1/commands/agent.thread.messages",
            axum::routing::post(list_messages),
        )
        .with_state(HealthState {
            worker_id: Arc::<str>::from(worker_id),
            auth_token: Arc::<str>::from(auth_token),
            application,
        })
}

pub(crate) async fn serve(
    listener: TcpListener,
    router: Router,
    shutdown: oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            let _ = shutdown.await;
        })
        .await
        .map_err(anyhow::Error::from)
}

async fn ensure_main_thread(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_THREAD_ENSURE_MAIN_OPERATION,
        |app, user_id, payload| async move {
            app.ensure_main_thread(user_id, decode_payload(payload)?)
                .await
        },
    )
    .await
}

async fn create_thread(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_THREAD_CREATE_OPERATION,
        |app, user_id, payload| async move {
            app.create_thread(user_id, decode_payload(payload)?).await
        },
    )
    .await
}

async fn fork_thread(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(&state, &headers, request, AGENT_THREAD_FORK_OPERATION, |app, user_id, payload| async move {
        app.fork_thread(user_id, decode_payload(payload)?).await
    })
    .await
}

async fn update_thread(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_THREAD_UPDATE_OPERATION,
        |app, user_id, payload| async move {
            app.update_thread(user_id, decode_payload(payload)?).await
        },
    )
    .await
}

async fn archive_thread(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_THREAD_ARCHIVE_OPERATION,
        |app, user_id, payload| async move {
            app.archive_thread(user_id, decode_payload(payload)?).await
        },
    )
    .await
}

async fn list_threads(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(&state, &headers, request, AGENT_THREAD_LIST_OPERATION, |app, user_id, payload| async move {
        app.list_threads(user_id, decode_payload(payload)?).await
    })
    .await
}

async fn show_thread(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_THREAD_SHOW_OPERATION,
        |app, user_id, payload| async move {
            let thread_id: String = decode_payload(payload)?;
            app.show_thread(user_id, &thread_id).await
        },
    )
    .await
}

async fn list_messages(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_THREAD_MESSAGES_OPERATION,
        |app, user_id, payload| async move {
            app.list_messages(user_id, decode_payload(payload)?).await
        },
    )
    .await
}

async fn user_command<O, F, Fut>(
    state: &HealthState,
    headers: &HeaderMap,
    request: WorkerEnvelope<Value>,
    operation: &str,
    command: F,
) -> Response
where
    O: Serialize,
    F: FnOnce(Arc<AgentApplication>, i64, Value) -> Fut,
    Fut: std::future::Future<Output = Result<O, AppError>>,
{
    if !state.authorized(headers) {
        return unauthorized();
    }
    let response = match validate_user_request(&request, operation).and_then(|user_id| {
        state
            .application
            .clone()
            .ok_or_else(|| WorkerFailure::internal("agent worker application 未初始化"))
            .map(|application| (application, user_id, request.payload.clone()))
    }) {
        Ok((application, user_id, payload)) => match command(application, user_id, payload).await {
            Ok(payload) => completed(&request, payload),
            Err(error) => failed(&request, app_error_failure(error)),
        },
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

fn validate_user_request(
    request: &WorkerEnvelope<Value>,
    operation: &str,
) -> Result<i64, WorkerFailure> {
    request
        .validate()
        .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
    if request.operation_id != operation {
        return Err(WorkerFailure::invalid_request(format!(
            "operation_id 不匹配: expected={operation}, actual={}",
            request.operation_id
        )));
    }
    if request.lease.worker_id != "agent-worker" {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 agent worker",
        ));
    }
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

fn decode_payload<T: DeserializeOwned>(payload: Value) -> Result<T, AppError> {
    serde_json::from_value(payload).map_err(|error| AppError::InvalidArgument(error.to_string()))
}

fn app_error_failure(error: AppError) -> WorkerFailure {
    match error {
        AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        AppError::NotFound(message) => WorkerFailure::not_found(message),
        AppError::Forbidden(message) | AppError::Unauthorized(message) => {
            WorkerFailure::unauthorized(message)
        }
        AppError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}; operation={operation}"))
        }
        other => WorkerFailure::internal(other.to_string()),
    }
}

#[derive(Debug)]
struct WorkerFailure {
    code: WorkerErrorCode,
    message: String,
    retryable: bool,
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
    fn internal(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Internal,
            message: message.into(),
            retryable: false,
        }
    }
}

fn completed<T: Serialize>(request: &WorkerEnvelope<Value>, payload: T) -> WorkerResponse<T> {
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

fn failed<T>(request: &WorkerEnvelope<Value>, failure: WorkerFailure) -> WorkerResponse<T> {
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

pub(crate) fn bind_addr(control_addr: SocketAddr) -> anyhow::Result<SocketAddr> {
    let configured = a4n_config::optional_non_empty_env_string(WORKER_ADDR_ENV)?;
    parse_bind_addr(configured.as_deref(), control_addr)
}

fn parse_bind_addr(
    configured: Option<&str>,
    control_addr: SocketAddr,
) -> anyhow::Result<SocketAddr> {
    let address = match configured {
        Some(value) => value
            .parse::<SocketAddr>()
            .map_err(|error| anyhow!("{WORKER_ADDR_ENV} 无效: {error}"))?,
        None => {
            let port = control_addr
                .port()
                .checked_add(1)
                .filter(|port| *port != 0)
                .unwrap_or(18080);
            SocketAddr::from(([127, 0, 0, 1], port))
        }
    };
    if !address.ip().is_loopback() {
        return Err(anyhow!("{WORKER_ADDR_ENV} 必须绑定 loopback 地址"));
    }
    if address.port() == 0 {
        return Err(anyhow!("{WORKER_ADDR_ENV} 不允许使用端口 0"));
    }
    Ok(address)
}

async fn healthz(State(state): State<HealthState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    Json(WorkerHealth {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: state.worker_id.to_string(),
        status: WorkerHealthStatus::Healthy,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
    .into_response()
}

async fn capabilities(State(state): State<HealthState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    Json(WorkerCapabilities {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: state.worker_id.to_string(),
        capabilities: WORKER_OPERATIONS
            .iter()
            .map(|name| WorkerCapability {
                name: (*name).to_string(),
                version: CAPABILITY_VERSION.to_string(),
                enabled: true,
            })
            .collect(),
    })
    .into_response()
}

impl HealthState {
    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(AGENT_WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        "Agent worker authentication required",
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use serde_json::Value;
    use std::time::Duration;
    use tokio::time::timeout;
    use tower::ServiceExt;

    const WORKER_ID: &str = "agent-test-worker";
    const TOKEN: &str = "agent-test-token";

    async fn request(router: Router, path: &str, token: Option<&str>) -> axum::response::Response {
        let mut request = Request::get(path).body(Body::empty()).unwrap();
        if let Some(token) = token {
            request.headers_mut().insert(
                AGENT_WORKER_AUTH_HEADER,
                token
                    .parse()
                    .expect("test token must be a valid header value"),
            );
        }
        router
            .oneshot(request)
            .await
            .expect("router should respond")
    }

    async fn json_body(response: axum::response::Response) -> Value {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        serde_json::from_slice(&body).expect("response should be JSON")
    }

    fn test_router() -> Router {
        router(WORKER_ID.into(), TOKEN.into(), None)
    }

    #[tokio::test]
    async fn health_and_capabilities_require_the_worker_token() {
        for path in [HEALTH_PATH, CAPABILITIES_PATH] {
            assert_eq!(
                request(test_router(), path, None).await.status(),
                StatusCode::UNAUTHORIZED
            );
            assert_eq!(
                request(test_router(), path, Some("wrong-token"))
                    .await
                    .status(),
                StatusCode::UNAUTHORIZED
            );
            assert_eq!(
                request(test_router(), path, Some(TOKEN)).await.status(),
                StatusCode::OK
            );
        }
    }

    #[tokio::test]
    async fn health_reports_ready_worker_schema_without_sidecar_fields() {
        let value = json_body(request(test_router(), HEALTH_PATH, Some(TOKEN)).await).await;
        assert_eq!(value["worker_id"], WORKER_ID);
        assert_eq!(value["status"], "healthy");
        assert_eq!(value["schema_version"], WORKER_SCHEMA_VERSION);
        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
        assert!(value.get("service").is_none());
        let decoded: WorkerHealth = serde_json::from_value(value).expect("health contract");
        decoded.validate().expect("health contract should validate");
    }

    #[tokio::test]
    async fn capabilities_report_worker_operations_and_schema() {
        let value = json_body(request(test_router(), CAPABILITIES_PATH, Some(TOKEN)).await).await;
        assert_eq!(value["worker_id"], WORKER_ID);
        assert_eq!(value["schema_version"], WORKER_SCHEMA_VERSION);
        assert_eq!(
            value["capabilities"],
            serde_json::json!([
                {
                    "name": AGENT_RUN_OPERATION,
                    "version": CAPABILITY_VERSION,
                    "enabled": true
                },
                {
                    "name": AGENT_HOST_COMMAND_OPERATION,
                    "version": CAPABILITY_VERSION,
                    "enabled": true
                },
                {"name": AGENT_THREAD_ENSURE_MAIN_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_THREAD_CREATE_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_THREAD_FORK_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_THREAD_UPDATE_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_THREAD_ARCHIVE_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_THREAD_LIST_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_THREAD_SHOW_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_THREAD_MESSAGES_OPERATION, "version": CAPABILITY_VERSION, "enabled": true}
            ])
        );
        assert!(value.get("capability_version").is_none());
        let decoded: WorkerCapabilities =
            serde_json::from_value(value).expect("capabilities contract");
        decoded
            .validate()
            .expect("capabilities contract should validate");
    }

    #[test]
    fn bind_address_defaults_to_loopback_after_control() {
        let control = "127.0.0.1:18080".parse().unwrap();
        assert_eq!(
            parse_bind_addr(None, control).unwrap(),
            "127.0.0.1:18081".parse().unwrap()
        );
    }

    #[test]
    fn bind_address_rejects_non_loopback_and_zero_port() {
        let control = "127.0.0.1:18080".parse().unwrap();
        assert!(parse_bind_addr(Some("0.0.0.0:18081"), control).is_err());
        assert!(parse_bind_addr(Some("127.0.0.1:0"), control).is_err());
        assert!(parse_bind_addr(Some("not-an-address"), control).is_err());
    }

    #[tokio::test]
    async fn health_server_exits_after_graceful_shutdown_signal() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener should bind");
        let (shutdown, receiver) = oneshot::channel();
        let task = tokio::spawn(serve(listener, test_router(), receiver));
        shutdown.send(()).expect("server should still be running");
        timeout(Duration::from_secs(1), task)
            .await
            .expect("server should stop promptly")
            .expect("server task should join")
            .expect("server should shut down cleanly");
    }
}
