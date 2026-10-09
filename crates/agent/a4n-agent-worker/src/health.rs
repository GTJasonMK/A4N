mod delegation;
mod memory;
mod responses;
mod runs;
mod skills;
mod threads;
mod tools;

use std::{net::SocketAddr, sync::Arc};

use a4n_agent_application::agent::AgentApplication;
use a4n_agent_runtime_contract::AgentWorkerToolReadApplicationPort;
use a4n_agent_runtime_contract::{
    AGENT_APPROVAL_RESOLVE_OPERATION, AGENT_DELEGATION_REVIEW_OPERATION,
    AGENT_DELEGATION_SPAWN_OPERATION, AGENT_EVENT_LIST_OPERATION, AGENT_MEMORY_CREATE_OPERATION,
    AGENT_MEMORY_LIST_OPERATION, AGENT_MEMORY_RETRACT_OPERATION, AGENT_RUN_CONTROL_CHILD_OPERATION,
    AGENT_RUN_CONTROL_OPERATION, AGENT_RUN_SEND_CHILD_OPERATION, AGENT_RUN_SEND_OPERATION,
    AGENT_RUN_SHOW_OPERATION, AGENT_SKILL_LOAD_OPERATION, AGENT_SKILL_SEARCH_OPERATION,
    AGENT_SKILL_SETTINGS_LIST_OPERATION, AGENT_SKILL_SETTINGS_UPDATE_OPERATION,
    AGENT_THREAD_ARCHIVE_OPERATION, AGENT_THREAD_CREATE_OPERATION,
    AGENT_THREAD_ENSURE_MAIN_OPERATION, AGENT_THREAD_FORK_OPERATION, AGENT_THREAD_LIST_OPERATION,
    AGENT_THREAD_MESSAGES_OPERATION, AGENT_THREAD_SHOW_OPERATION, AGENT_THREAD_UPDATE_OPERATION,
    AGENT_TOOL_DESCRIBE_OPERATION, AGENT_TOOL_SEARCH_OPERATION,
};
use a4n_api_contract::agent::{
    AgentApprovalApplicationPort, AgentDelegationApplicationPort, AgentEventReadApplicationPort,
    AgentMemoryMutationApplicationPort, AgentMemoryReadApplicationPort,
    AgentRunMutationApplicationPort, AgentRunReadApplicationPort, AgentSkillApplicationPort,
    AgentThreadMutationApplicationPort, AgentThreadReadApplicationPort,
};
use a4n_error::AppError;
use a4n_worker_contract::{
    AGENT_HOST_COMMAND_OPERATION, AGENT_RUN_OPERATION, AGENT_WORKER_AUTH_HEADER,
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerHealth, WorkerHealthStatus, WorkerPrincipalKind,
};
use anyhow::anyhow;
use axum::{
    Json, Router,
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use tokio::{net::TcpListener, sync::oneshot};

use self::responses::{WorkerFailure, app_error_failure, completed, failed, unauthorized};

const WORKER_ADDR_ENV: &str = "A4N_AGENT_WORKER_ADDR";
const HEALTH_PATH: &str = WORKER_HEALTH_PATH;
const CAPABILITIES_PATH: &str = WORKER_CAPABILITIES_PATH;
const CAPABILITY_VERSION: &str = "agent-worker-v2";
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
    AGENT_RUN_SHOW_OPERATION,
    AGENT_EVENT_LIST_OPERATION,
    AGENT_MEMORY_LIST_OPERATION,
    AGENT_MEMORY_CREATE_OPERATION,
    AGENT_MEMORY_RETRACT_OPERATION,
    AGENT_APPROVAL_RESOLVE_OPERATION,
    AGENT_DELEGATION_SPAWN_OPERATION,
    AGENT_DELEGATION_REVIEW_OPERATION,
    AGENT_SKILL_SETTINGS_LIST_OPERATION,
    AGENT_SKILL_SETTINGS_UPDATE_OPERATION,
    AGENT_SKILL_SEARCH_OPERATION,
    AGENT_SKILL_LOAD_OPERATION,
    AGENT_TOOL_SEARCH_OPERATION,
    AGENT_TOOL_DESCRIBE_OPERATION,
    AGENT_RUN_SEND_OPERATION,
    AGENT_RUN_CONTROL_OPERATION,
    AGENT_RUN_SEND_CHILD_OPERATION,
    AGENT_RUN_CONTROL_CHILD_OPERATION,
];

#[derive(Clone)]
struct HealthState {
    worker_id: Arc<str>,
    auth_token: Arc<str>,
    application: Arc<AgentApplication>,
    runtime: Arc<a4n_agent_runtime::AgentRuntimeState>,
}

pub(crate) fn router(
    worker_id: String,
    auth_token: String,
    application: Arc<AgentApplication>,
    runtime: Arc<a4n_agent_runtime::AgentRuntimeState>,
) -> Router {
    Router::new()
        .route(HEALTH_PATH, get(healthz))
        .route(CAPABILITIES_PATH, get(capabilities))
        .route(
            "/v1/commands/agent.thread.ensure_main",
            axum::routing::post(threads::ensure_main_thread),
        )
        .route(
            "/v1/commands/agent.thread.create",
            axum::routing::post(threads::create_thread),
        )
        .route(
            "/v1/commands/agent.thread.fork",
            axum::routing::post(threads::fork_thread),
        )
        .route(
            "/v1/commands/agent.thread.update",
            axum::routing::post(threads::update_thread),
        )
        .route(
            "/v1/commands/agent.thread.archive",
            axum::routing::post(threads::archive_thread),
        )
        .route(
            "/v1/commands/agent.thread.list",
            axum::routing::post(threads::list_threads),
        )
        .route(
            "/v1/commands/agent.thread.show",
            axum::routing::post(threads::show_thread),
        )
        .route(
            "/v1/commands/agent.thread.messages",
            axum::routing::post(threads::list_messages),
        )
        .route(
            "/v1/commands/agent.run.show",
            axum::routing::post(runs::show_run),
        )
        .route(
            "/v1/commands/agent.event.list",
            axum::routing::post(runs::list_events),
        )
        .route(
            "/v1/commands/agent.memory.list",
            axum::routing::post(memory::list_memories),
        )
        .route(
            "/v1/commands/agent.memory.create",
            axum::routing::post(memory::create_memory),
        )
        .route(
            "/v1/commands/agent.memory.retract",
            axum::routing::post(memory::retract_memory),
        )
        .route(
            "/v1/commands/agent.approval.resolve",
            axum::routing::post(resolve_approval),
        )
        .route(
            "/v1/commands/agent.delegation.spawn",
            axum::routing::post(delegation::spawn_delegation),
        )
        .route(
            "/v1/commands/agent.delegation.review",
            axum::routing::post(delegation::review_delegation),
        )
        .route(
            "/v1/commands/agent.skill.settings.list",
            axum::routing::post(skills::list_skill_settings),
        )
        .route(
            "/v1/commands/agent.skill.settings.update",
            axum::routing::post(skills::update_skill_configuration),
        )
        .route(
            "/v1/commands/agent.skill.search",
            axum::routing::post(skills::search_skills),
        )
        .route(
            "/v1/commands/agent.skill.load",
            axum::routing::post(skills::load_skill),
        )
        .route(
            "/v1/commands/agent.tool.search",
            axum::routing::post(tools::search_tools),
        )
        .route(
            "/v1/commands/agent.tool.describe",
            axum::routing::post(tools::describe_tool),
        )
        .route(
            "/v1/commands/agent.run.send",
            axum::routing::post(runs::send_message),
        )
        .route(
            "/v1/commands/agent.run.control",
            axum::routing::post(runs::control_run),
        )
        .route(
            "/v1/commands/agent.run.send_child",
            axum::routing::post(runs::send_message_to_child),
        )
        .route(
            "/v1/commands/agent.run.control_child",
            axum::routing::post(runs::control_child),
        )
        .with_state(HealthState {
            worker_id: Arc::<str>::from(worker_id),
            auth_token: Arc::<str>::from(auth_token),
            application,
            runtime,
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

async fn resolve_approval(
    State(state): State<HealthState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    user_command(
        &state,
        &headers,
        request,
        AGENT_APPROVAL_RESOLVE_OPERATION,
        |app, user_id, payload| async move {
            app.resolve_approval(user_id, decode_payload(payload)?)
                .await
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
    let response = match validate_user_request(&request, operation) {
        Ok(user_id) => {
            match command(state.application.clone(), user_id, request.payload.clone()).await {
                Ok(payload) => completed(&request, payload),
                Err(error) => failed(&request, app_error_failure(error)),
            }
        }
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

fn validate_user_request(
    request: &WorkerEnvelope<Value>,
    operation: &str,
) -> Result<i64, WorkerFailure> {
    request.validate_worker_command(operation, "agent-worker", "agent worker")?;
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
        a4n_worker_runtime::worker_token_authorized(
            headers,
            AGENT_WORKER_AUTH_HEADER,
            self.auth_token.as_bytes(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
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
        let application = Arc::new(AgentApplication::new(a4n_db::DbHandle::new(Arc::new(
            sea_orm::DatabaseConnection::Disconnected,
        ))));
        let host = Arc::new(crate::host::HttpAgentRuntimeHost::new(
            reqwest::Client::new(),
            "http://127.0.0.1:1".into(),
            TOKEN.into(),
            application.clone(),
        ));
        let runtime = Arc::new(a4n_agent_runtime::AgentRuntimeState::new(host));
        router(WORKER_ID.into(), TOKEN.into(), application, runtime)
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
                {"name": AGENT_THREAD_MESSAGES_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_RUN_SHOW_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_EVENT_LIST_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_MEMORY_LIST_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_MEMORY_CREATE_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_MEMORY_RETRACT_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_APPROVAL_RESOLVE_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_DELEGATION_SPAWN_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_DELEGATION_REVIEW_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_SKILL_SETTINGS_LIST_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_SKILL_SETTINGS_UPDATE_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_SKILL_SEARCH_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_SKILL_LOAD_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_TOOL_SEARCH_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_TOOL_DESCRIBE_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_RUN_SEND_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_RUN_CONTROL_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_RUN_SEND_CHILD_OPERATION, "version": CAPABILITY_VERSION, "enabled": true},
                {"name": AGENT_RUN_CONTROL_CHILD_OPERATION, "version": CAPABILITY_VERSION, "enabled": true}
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
