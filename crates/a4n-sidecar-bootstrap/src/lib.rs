//! Shared process bootstrap rules for A4N sidecar binaries.

mod data_manifest;
mod environment;
mod event_stream;
mod logging;
mod metrics;

use std::{
    future::Future,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use a4n_process_catalog::{parse_service_socket_addr, service_catalog_entry};
use a4n_service_protocol::{
    SIDECAR_AUTH_HEADER, SIDECAR_AUTH_TOKEN_ENV, SIDECAR_HEALTH_PATH, SIDECAR_SHUTDOWN_PATH,
};
use anyhow::anyhow;
use axum::{
    Router,
    extract::{Extension, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::post,
};
pub use data_manifest::{ensure_data_directory_manifest, require_data_directory_manifest};
pub use event_stream::{
    failed_service_event_response, service_event_channel, service_event_line,
    service_event_ndjson_response,
};
use futures::channel::oneshot;
use metrics::with_runtime_metrics;
use subtle::ConstantTimeEq;
use tower::limit::concurrency::GlobalConcurrencyLimitLayer;

use crate::environment::{optional_non_empty_string, string_or_default};

/// Apply one concurrency budget to the whole sidecar, including all routes.
///
/// Axum clones a router layer per route; the global variant keeps one semaphore
/// across those clones instead of accidentally giving every endpoint its own
/// budget.
pub fn concurrency_limit(max_concurrent: usize) -> GlobalConcurrencyLimitLayer {
    GlobalConcurrencyLimitLayer::new(max_concurrent)
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
        .route(SIDECAR_SHUTDOWN_PATH, post(request_shutdown))
        .layer(Extension(signal))
}

pub async fn serve_sidecar(router: Router, addr: SocketAddr, service: &str) -> anyhow::Result<()> {
    let auth_token = optional_non_empty_string(SIDECAR_AUTH_TOKEN_ENV)?.map(Arc::<str>::from);
    validate_bind_addr(addr, auth_token.is_some())?;
    let (shutdown_signal, shutdown) = sidecar_shutdown();
    let app = with_runtime_metrics(
        router.merge(shutdown_router(shutdown_signal)),
        service,
        auth_token,
    )?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

pub fn init_tracing(log_env: &str, default_filter: &str) -> anyhow::Result<()> {
    let filter_text = string_or_default(log_env, default_filter)?;
    logging::init(log_env, &filter_text)
}

pub fn socket_addr(addr_env: &str, default_addr: &str) -> anyhow::Result<SocketAddr> {
    let value = string_or_default(addr_env, default_addr)?;
    parse_service_socket_addr(addr_env, &value).map_err(anyhow::Error::msg)
}

pub fn service_socket_addr(service: &str) -> anyhow::Result<SocketAddr> {
    let entry =
        service_catalog_entry(service).ok_or_else(|| anyhow!("未知 sidecar service: {service}"))?;
    socket_addr(&entry.addr_env, &entry.default_addr)
}

fn validate_bind_addr(addr: SocketAddr, has_auth: bool) -> anyhow::Result<()> {
    if !addr.ip().is_loopback() && !has_auth {
        return Err(anyhow!(
            "sidecar 只能在配置 authentication 后绑定非 loopback 地址"
        ));
    }
    Ok(())
}

/// Require the manager-issued token on every non-health endpoint.
async fn require_auth(
    State(expected): State<Option<Arc<str>>>,
    request: Request,
    next: Next,
) -> Response {
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
    if path == SIDECAR_HEALTH_PATH {
        return None;
    }
    let Some(expected) = expected else {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_loopback_bind_requires_authentication() {
        let addr: SocketAddr = "0.0.0.0:47641".parse().unwrap();
        assert!(validate_bind_addr(addr, false).is_err());
        assert!(validate_bind_addr(addr, true).is_ok());
    }

    #[test]
    fn loopback_bind_is_allowed_without_authentication() {
        let addr: SocketAddr = "127.0.0.1:47641".parse().unwrap();
        assert!(validate_bind_addr(addr, false).is_ok());
    }

    #[test]
    fn health_endpoint_bypasses_authentication() {
        assert_eq!(auth_rejection(SIDECAR_HEALTH_PATH, None, None), None);
    }

    #[test]
    fn missing_server_token_fails_closed() {
        let rejection = auth_rejection("/v1/generate", None, None).unwrap();
        assert_eq!(rejection.0, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(rejection.1, "sidecar authentication is not configured");
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
