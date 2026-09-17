use std::{net::SocketAddr, sync::Arc};

use a4n_worker_contract::{
    AGENT_HOST_COMMAND_OPERATION, AGENT_RUN_OPERATION, AGENT_WORKER_AUTH_HEADER,
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerHealth, WorkerHealthStatus,
};
use anyhow::anyhow;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use subtle::ConstantTimeEq;
use tokio::{net::TcpListener, sync::oneshot};

const WORKER_ADDR_ENV: &str = "A4N_AGENT_WORKER_ADDR";
const HEALTH_PATH: &str = WORKER_HEALTH_PATH;
const CAPABILITIES_PATH: &str = WORKER_CAPABILITIES_PATH;
const CAPABILITY_VERSION: &str = "agent-worker-v1";
const WORKER_OPERATIONS: &[&str] = &[AGENT_RUN_OPERATION, AGENT_HOST_COMMAND_OPERATION];

#[derive(Clone)]
struct HealthState {
    worker_id: Arc<str>,
    auth_token: Arc<str>,
}

pub(crate) fn router(worker_id: String, auth_token: String) -> Router {
    Router::new()
        .route(HEALTH_PATH, get(healthz))
        .route(CAPABILITIES_PATH, get(capabilities))
        .with_state(HealthState {
            worker_id: Arc::<str>::from(worker_id),
            auth_token: Arc::<str>::from(auth_token),
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
        router(WORKER_ID.into(), TOKEN.into())
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
                }
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
