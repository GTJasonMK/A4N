//! Shared process bootstrap rules for A4N sidecar binaries.

mod data_manifest;
mod event_stream;
mod logging;
mod metrics;

use std::{
    env::VarError,
    future::Future,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use a4n_service_protocol::{
    SIDECAR_AUTH_HEADER, SIDECAR_AUTH_TOKEN_ENV, SIDECAR_HOST_PROTOCOL_VERSION_HEADER,
};
use anyhow::anyhow;
use axum::{
    Router,
    extract::{Extension, Request},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::post,
};
pub use data_manifest::{ensure_data_directory_manifest, require_data_directory_manifest};
pub use event_stream::{
    service_event_line, service_event_ndjson_response, single_service_event_stream,
};
use futures::channel::oneshot;
use metrics::with_runtime_metrics;
use subtle::ConstantTimeEq;
use tower::limit::concurrency::GlobalConcurrencyLimitLayer;

/// Apply one concurrency budget to the whole sidecar, including all routes.
///
/// Axum clones a router layer per route; the global variant keeps one semaphore
/// across those clones instead of accidentally giving every endpoint its own
/// budget.
pub fn concurrency_limit(max_concurrent: usize) -> GlobalConcurrencyLimitLayer {
    GlobalConcurrencyLimitLayer::new(max_concurrent)
}

pub fn host_protocol_version(headers: &HeaderMap) -> Option<u32> {
    headers
        .get(SIDECAR_HOST_PROTOCOL_VERSION_HEADER)?
        .to_str()
        .ok()?
        .parse()
        .ok()
}

#[derive(Clone)]
struct SidecarShutdownSignal {
    sender: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

fn sidecar_shutdown() -> (
    SidecarShutdownSignal,
    impl Future<Output = ()> + Send + 'static,
) {
    let (sender, receiver) = oneshot::channel();
    (
        SidecarShutdownSignal {
            sender: Arc::new(Mutex::new(Some(sender))),
        },
        async move {
            let _ = receiver.await;
        },
    )
}

async fn request_shutdown(Extension(signal): Extension<SidecarShutdownSignal>) -> StatusCode {
    if let Some(sender) = signal
        .sender
        .lock()
        .expect("sidecar shutdown signal lock should not be poisoned")
        .take()
    {
        let _ = sender.send(());
    }
    StatusCode::ACCEPTED
}

fn shutdown_router(signal: SidecarShutdownSignal) -> Router {
    Router::new()
        .route("/shutdown", post(request_shutdown))
        .route_layer(axum::middleware::from_fn(require_auth))
        .layer(Extension(signal))
}

pub async fn serve_sidecar(router: Router, addr: SocketAddr, service: &str) -> anyhow::Result<()> {
    let (shutdown_signal, shutdown) = sidecar_shutdown();
    let app = with_runtime_metrics(router.merge(shutdown_router(shutdown_signal)), service)?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

pub fn init_tracing(log_env: &str, default_filter: &str) -> anyhow::Result<()> {
    let filter_text = env_value_or_default(log_env, default_filter)?;
    logging::init(log_env, &filter_text)
}

pub fn socket_addr(addr_env: &str, default_addr: &str) -> anyhow::Result<SocketAddr> {
    let value = env_value_or_default(addr_env, default_addr)?;
    let addr = parse_socket_addr(addr_env, &value)?;
    let has_auth = std::env::var(SIDECAR_AUTH_TOKEN_ENV)
        .ok()
        .is_some_and(|token| !token.trim().is_empty());
    validate_bind_addr(addr_env, addr, has_auth)?;
    Ok(addr)
}

fn parse_socket_addr(addr_env: &str, value: &str) -> anyhow::Result<SocketAddr> {
    value
        .parse()
        .map_err(|error| anyhow!("{addr_env} 不是有效监听地址: {error}"))
}

fn validate_bind_addr(addr_env: &str, addr: SocketAddr, has_auth: bool) -> anyhow::Result<()> {
    if !addr.ip().is_loopback() && !has_auth {
        return Err(anyhow!(
            "{addr_env} 只能在配置 sidecar authentication 后绑定非 loopback 地址"
        ));
    }
    Ok(())
}

/// Require the manager-issued token on every non-health endpoint.
/// Unit-test routers are allowed to omit the token; real sidecar binaries are
/// compiled without `cfg(test)` and fail closed when the token is absent.
pub async fn require_auth(request: Request, next: Next) -> Response {
    if cfg!(test) {
        return next.run(request).await;
    }

    let expected = std::env::var(SIDECAR_AUTH_TOKEN_ENV).ok();
    let actual = request
        .headers()
        .get(SIDECAR_AUTH_HEADER)
        .map(|value| value.as_bytes());
    if let Some((status, message)) =
        auth_rejection(request.uri().path(), expected.as_deref(), actual)
    {
        return (status, message).into_response();
    }
    next.run(request).await
}

fn auth_rejection(
    path: &str,
    expected: Option<&str>,
    actual: Option<&[u8]>,
) -> Option<(StatusCode, &'static str)> {
    if path == "/healthz" {
        return None;
    }
    let Some(expected) = expected.filter(|value| !value.trim().is_empty()) else {
        return Some((
            StatusCode::SERVICE_UNAVAILABLE,
            "sidecar authentication is not configured",
        ));
    };
    let Some(actual) = actual else {
        return Some((StatusCode::UNAUTHORIZED, "sidecar authentication required"));
    };
    if !bool::from(actual.ct_eq(expected.as_bytes())) {
        return Some((StatusCode::UNAUTHORIZED, "sidecar authentication failed"));
    }
    None
}

fn env_value_or_default(key: &str, default_value: &str) -> anyhow::Result<String> {
    resolve_env_value(key, std::env::var(key), default_value).map_err(anyhow::Error::msg)
}

fn resolve_env_value(
    key: &str,
    configured: Result<String, VarError>,
    default_value: &str,
) -> Result<String, String> {
    let value = match configured {
        Ok(value) => value,
        Err(VarError::NotPresent) => default_value.to_string(),
        Err(error) => return Err(format!("读取 {key} 失败: {error}")),
    };
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{key} 不能为空"));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_value_uses_default() {
        assert_eq!(
            resolve_env_value("A4N_SERVICE_ADDR", Err(VarError::NotPresent), "127.0.0.1:1")
                .unwrap(),
            "127.0.0.1:1"
        );
    }

    #[test]
    fn configured_value_is_trimmed() {
        assert_eq!(
            resolve_env_value("A4N_SERVICE_ADDR", Ok(" 127.0.0.1:2 ".into()), "unused").unwrap(),
            "127.0.0.1:2"
        );
    }

    #[test]
    fn configured_blank_value_is_rejected() {
        let error = resolve_env_value("A4N_SERVICE_ADDR", Ok("  ".into()), "127.0.0.1:1")
            .expect_err("configured blank value must fail");
        assert!(error.contains("不能为空"));
    }

    #[test]
    fn invalid_socket_addr_is_rejected() {
        let error = parse_socket_addr("A4N_SERVICE_ADDR", "token=sidecar-secret")
            .expect_err("invalid address must fail")
            .to_string();
        assert!(error.contains("A4N_SERVICE_ADDR 不是有效监听地址"));
        assert!(!error.contains("sidecar-secret"));
    }

    #[test]
    fn non_loopback_bind_requires_authentication() {
        let addr: SocketAddr = "0.0.0.0:47641".parse().unwrap();
        assert!(validate_bind_addr("A4N_SERVICE_ADDR", addr, false).is_err());
        assert!(validate_bind_addr("A4N_SERVICE_ADDR", addr, true).is_ok());
    }

    #[test]
    fn loopback_bind_is_allowed_without_authentication() {
        let addr: SocketAddr = "127.0.0.1:47641".parse().unwrap();
        assert!(validate_bind_addr("A4N_SERVICE_ADDR", addr, false).is_ok());
    }

    #[test]
    fn health_endpoint_bypasses_authentication() {
        assert_eq!(auth_rejection("/healthz", None, None), None);
    }

    #[test]
    fn host_protocol_version_is_read_from_the_health_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            SIDECAR_HOST_PROTOCOL_VERSION_HEADER,
            a4n_service_protocol::PROTOCOL_VERSION.into(),
        );
        assert_eq!(
            host_protocol_version(&headers),
            Some(a4n_service_protocol::PROTOCOL_VERSION)
        );
        assert_eq!(host_protocol_version(&HeaderMap::new()), None);
    }

    #[test]
    fn missing_server_token_fails_closed() {
        let rejection = auth_rejection("/v1/generate", None, None).unwrap();
        assert_eq!(rejection.0, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(rejection.1, "sidecar authentication is not configured");

        let blank_rejection = auth_rejection("/v1/generate", Some("   "), None).unwrap();
        assert_eq!(blank_rejection, rejection);
    }

    #[test]
    fn missing_or_wrong_request_token_is_unauthorized_without_leaking_token() {
        let expected = "manager-issued-secret-token";
        let missing = auth_rejection("/v1/generate", Some(expected), None).unwrap();
        assert_eq!(missing.0, StatusCode::UNAUTHORIZED);

        let wrong = auth_rejection("/v1/generate", Some(expected), Some(b"wrong-token")).unwrap();
        assert_eq!(wrong.0, StatusCode::UNAUTHORIZED);
        assert!(!missing.1.contains(expected));
        assert!(!wrong.1.contains(expected));
    }

    #[test]
    fn matching_request_token_is_allowed() {
        assert_eq!(
            auth_rejection(
                "/v1/generate",
                Some("manager-issued-secret-token"),
                Some(b"manager-issued-secret-token"),
            ),
            None
        );
    }
}
